#![allow(dead_code, unused_variables)]

pub mod dependencies {
    use std::collections::{HashMap, HashSet, VecDeque};
    use std::fs::read_to_string;
    use std::iter;
    use std::path::{Path, PathBuf};

    use proc_macro2::Span;
    use quote::ToTokens;
    use syn::ItemMod;
    use syn::{
        Ident, ItemUse, UseGlob, UseGroup, UseName, UsePath, UseRename, UseTree, parse_file,
        spanned::Spanned,
        visit::{self, Visit},
    };
    use thiserror::Error;

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub struct ModuleName(String);

    impl From<String> for ModuleName {
        fn from(value: String) -> Self {
            Self(value)
        }
    }
    impl From<&str> for ModuleName {
        fn from(value: &str) -> Self {
            Self(value.to_owned())
        }
    }

    /// A single, separate use statement.
    #[derive(Debug, PartialEq, Eq)]
    pub struct NormalizedUseStatement {
        module_name: ModuleName,
        statement_type: UseStatementType,
    }

    #[derive(Debug, PartialEq, Eq)]
    pub enum UseStatementType {
        /// `use crate::log::Bar;`
        Simple(String),
        /// `use crate::log::Bar as Baz;`
        Alias(String, String),
        /// `use crate::log::*;`
        WildCard,
    }

    #[derive(Debug)]
    pub struct UseStatement {
        /// Where the use statement appears.
        source_module: ModuleName,
        /// List of referenced modules.
        /// Several targets, to represent `use crate::{log, foo::{bar, baz}};`
        target_modules: Vec<ModuleName>,
        /// Where in the source file the use statement is.
        statement: UseStatementDetail,
    }

    pub type UseStatements = Vec<UseStatement>;

    #[derive(Debug, Hash, PartialEq, Eq)]
    pub struct File(String);

    pub type UseStatementMap = HashMap<File, UseStatements>;

    #[derive(Debug)]
    struct ModStatement {
        ident: Ident,
        span: Span,
    }

    #[derive(Debug)]
    struct UseStatementDetail {
        items: Vec<NormalizedUseStatement>,
        span: Span,
    }

    #[derive(Debug)]
    struct Visitor {
        use_statements: Vec<UseStatementDetail>,
        mod_statements: Vec<ModStatement>,
    }

    #[derive(Debug)]
    struct FileToVisit {
        file: PathBuf,
        module_ancestors: Vec<String>,
    }

    impl Visitor {
        fn new() -> Self {
            Self {
                use_statements: Vec::new(),
                mod_statements: Vec::new(),
            }
        }
    }

    // TODO: Visit also `mod` nodes, otherwise we would be missing some modules
    impl<'ast> Visit<'ast> for Visitor {
        fn visit_item_mod(&mut self, node: &'ast ItemMod) {
            self.mod_statements.push(ModStatement {
                span: node.span(),
                ident: node.ident.to_owned(),
            });
        }

        fn visit_item_use(&mut self, node: &'ast ItemUse) {
            let tokens = node.to_token_stream();
            let items = flatten_use_tree("", &node.tree);
            self.use_statements.push(UseStatementDetail {
                items,
                span: node.span(),
            });

            visit::visit_item_use(self, node);
        }
    }

    fn flatten_use_tree(prefix: &str, tree: &UseTree) -> Vec<NormalizedUseStatement> {
        let prefixed = |ident: &Ident| {
            if prefix.is_empty() {
                ident.to_string()
            } else {
                format!("{}::{}", prefix, ident)
            }
        };
        match tree {
            UseTree::Path(UsePath { ident, tree, .. }) => {
                let new_prefix = prefixed(ident);
                flatten_use_tree(&new_prefix, tree)
            }

            UseTree::Name(UseName { ident, .. }) => {
                vec![NormalizedUseStatement {
                    module_name: ModuleName(prefix.to_owned()),
                    statement_type: UseStatementType::Simple(ident.to_string()),
                }]
            }

            UseTree::Rename(UseRename { ident, rename, .. }) => {
                vec![NormalizedUseStatement {
                    module_name: ModuleName(prefix.to_owned()),
                    statement_type: UseStatementType::Alias(ident.to_string(), rename.to_string()),
                }]
            }

            UseTree::Glob(UseGlob { .. }) => {
                vec![NormalizedUseStatement {
                    module_name: ModuleName(prefix.to_owned()),
                    statement_type: UseStatementType::WildCard,
                }]
            }

            UseTree::Group(UseGroup { items, .. }) => items
                .iter()
                .flat_map(|subtree| flatten_use_tree(prefix, subtree))
                .collect(),
        }
    }

    #[derive(Debug, Error)]
    pub enum ListUseStatementError {
        #[error("file not found")]
        FileNotFound,
        #[error("file not parsable")]
        FileNotParsable,
        #[error("file not readable")]
        FileNotReadable,
        #[error("path is not a crate")]
        PathIsNotACrate,
        #[error("linked module does not exists: {0}")]
        ModuleDoesNotExists(String),
    }

    fn get_crate_entrypoint(crate_root: &Path) -> Option<PathBuf> {
        // TODO: support multiple targets and custom paths different than src/main.rs or src/lib.rs

        let cargo_toml = crate_root.join("Cargo.toml");
        if !cargo_toml.exists() {
            return None;
        }

        let main_rs = crate_root.join(Path::new("src/main.rs"));
        if main_rs.exists() {
            return Some(main_rs);
        }

        let lib_rs = crate_root.join(Path::new("src/lib.rs"));
        if lib_rs.exists() {
            return Some(lib_rs);
        };
        None
    }

    fn mod_to_path(crate_root: &Path, ancestors: &[String], ident: &Ident) -> Option<PathBuf> {
        let ident = ident.to_string();
        let mut root_path = crate_root.join("src");
        root_path.extend(ancestors);

        let file_module = root_path.join(format!("{}.rs", ident));
        let folder_module = root_path.join(&ident).join("mod.rs");
        if crate_root.join(&file_module).exists() {
            return Some(file_module);
        } else if crate_root.join(&folder_module).exists() {
            return Some(folder_module);
        }
        None
    }

    /// List all the `use` statements in the crate, by file/module.
    pub fn list_use_statements(
        crate_root: &Path,
    ) -> Result<UseStatementMap, ListUseStatementError> {
        let mut files_visited = HashSet::new();
        let mut files_to_visit = VecDeque::new();
        let mut use_statement_map: UseStatementMap = HashMap::new();
        let entry_point =
            get_crate_entrypoint(crate_root).ok_or(ListUseStatementError::PathIsNotACrate)?;
        let src_folder = entry_point
            .parent()
            .expect("Failed to get entry point parent folder");
        files_to_visit.push_back(FileToVisit {
            file: entry_point.clone(),
            module_ancestors: vec![],
        });
        while let Some(file_to_visit) = files_to_visit.pop_front() {
            if files_visited.contains(&file_to_visit.file) {
                continue;
            }

            if !file_to_visit.file.exists() {
                return Err(ListUseStatementError::FileNotFound);
            }

            let content = read_to_string(&file_to_visit.file)
                .map_err(|_| ListUseStatementError::FileNotReadable)?;

            let parsed_file =
                parse_file(&content).map_err(|_| ListUseStatementError::FileNotParsable)?;

            let mut visitor = Visitor::new();
            visitor.visit_file(&parsed_file);

            files_to_visit.extend(visitor.mod_statements.iter().filter_map(|s| {
                mod_to_path(crate_root, &file_to_visit.module_ancestors, &s.ident).map(|file| {
                    FileToVisit {
                        file,
                        module_ancestors: file_to_visit
                            .module_ancestors
                            .iter()
                            .cloned()
                            .chain(iter::once(s.ident.to_string()))
                            .collect(),
                    }
                })
            }));
            let statements = visitor
                .use_statements
                .into_iter()
                .map(
                    |UseStatementDetail {
                         items,
                         span: extent,
                     }| {
                        let target_modules =
                            items.iter().map(|item| item.module_name.clone()).collect();

                        dbg!(&file_to_visit);
                        UseStatement {
                            // TODO: this is not the correct module if there is a scoped mod in the file
                            source_module: file_to_visit
                                .module_ancestors
                                .last()
                                .unwrap_or(&"".to_owned())
                                .clone()
                                .into(),
                            target_modules,
                            statement: UseStatementDetail {
                                items,
                                span: extent,
                            },
                        }
                    },
                )
                .collect();

            use_statement_map.insert(
                File(
                    file_to_visit
                        .file
                        .strip_prefix(crate_root)
                        .unwrap_or(&file_to_visit.file)
                        .to_string_lossy()
                        .to_string(),
                ),
                statements,
            );
            files_visited.insert(file_to_visit.file);
        }

        Ok(use_statement_map)
    }

    fn is_local_import(name: &ModuleName) -> bool {
        name.0 == "crate" || name.0.starts_with("crate::")
    }

    pub type ModuleDependencies = HashMap<ModuleName, Vec<ModuleName>>;

    /// List the (circular?) dependencies of modules inside the given crate, based on the use statements.
    pub fn list_dependencies(use_statements: &UseStatementMap) -> ModuleDependencies {
        todo!()
    }

    #[cfg(test)]
    mod tests {
        use std::path::Path;

        use pretty_assertions::assert_eq;
        use proc_macro2::LineColumn;
        use syn::visit::Visit;

        use crate::dependencies::{
            File, ModuleName, NormalizedUseStatement, UseStatementType, Visitor,
            list_use_statements,
        };

        #[test]
        fn gets_a_simple_dependency() {
            let test_project = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/simple/");
            let res = list_use_statements(&test_project).expect("Failed to list statements");

            let main_statement = &res.get(&File("src/main.rs".to_owned())).unwrap()[0];
            let module_a_statement = &res.get(&File("src/module_a/mod.rs".to_owned())).unwrap()[0];
            assert_eq!(main_statement.source_module, "".into());
            assert_eq!(main_statement.target_modules, vec!["crate".into()]);
            assert_eq!(
                main_statement.statement.span.start(),
                LineColumn { line: 2, column: 0 }
            );
            assert_eq!(
                main_statement.statement.span.end(),
                LineColumn {
                    line: 2,
                    column: 20
                }
            );
            assert_eq!(
                main_statement.statement.items,
                vec![NormalizedUseStatement {
                    module_name: "crate".into(),
                    statement_type: UseStatementType::Simple("module_a".to_owned()),
                }]
            );
            assert_eq!(module_a_statement.source_module, "module_a".into());
            assert_eq!(
                module_a_statement.target_modules,
                vec!["std::collections".into()]
            );
            assert_eq!(
                module_a_statement.statement.span.start(),
                LineColumn { line: 1, column: 0 }
            );
            assert_eq!(
                module_a_statement.statement.span.end(),
                LineColumn {
                    line: 1,
                    column: 30
                }
            );
            assert_eq!(
                module_a_statement.statement.items,
                vec![NormalizedUseStatement {
                    module_name: "std::collections".into(),
                    statement_type: UseStatementType::Simple("HashMap".to_owned()),
                }]
            );
        }

        #[test]
        fn flattens_alias() {
            let src = "use crate::foo::Bar as Baz;";
            let file = syn::parse_file(src).unwrap();
            let mut visitor = Visitor::new();
            visitor.visit_file(&file);
            let items = &visitor.use_statements[0].items;
            assert_eq!(items.len(), 1);
            assert_eq!(
                items[0],
                NormalizedUseStatement {
                    module_name: "crate::foo".into(),
                    statement_type: UseStatementType::Alias("Bar".into(), "Baz".into()),
                }
            );
        }

        #[test]
        fn flattens_wildcard() {
            let src = "use crate::foo::*;";
            let file = syn::parse_file(src).unwrap();
            let mut visitor = Visitor::new();
            visitor.visit_file(&file);
            let items = &visitor.use_statements[0].items;
            assert_eq!(
                items,
                &vec![NormalizedUseStatement {
                    module_name: "crate::foo".into(),
                    statement_type: UseStatementType::WildCard,
                }]
            );
        }

        #[test]
        fn flattens_grouped() {
            let src = "use crate::{foo, bar::{baz, qux}};";
            let file = syn::parse_file(src).unwrap();
            let mut visitor = Visitor::new();
            visitor.visit_file(&file);
            let names: Vec<_> = visitor.use_statements[0]
                .items
                .iter()
                .map(|i| (&i.module_name, &i.statement_type))
                .collect();
            assert_eq!(
                names,
                vec![
                    (
                        &ModuleName("crate".to_owned()),
                        &UseStatementType::Simple("foo".into())
                    ),
                    (
                        &ModuleName("crate::bar".to_owned()),
                        &UseStatementType::Simple("baz".into())
                    ),
                    (
                        &ModuleName("crate::bar".to_owned()),
                        &UseStatementType::Simple("qux".into())
                    ),
                ]
            );
        }
    }
}

pub mod refactor {
    use std::path::Path;

    use crate::dependencies::{ModuleName, UseStatementMap};

    pub fn extract_crate(
        crate_root: &Path,
        module: &ModuleName,
        target_crate_name: &str,
        target_crate_root: &std::path::Path,
        use_statements: &UseStatementMap,
    ) {
        // Should probably return errors.
        todo!()
    }
}
