#![allow(dead_code, unused_variables)]
pub mod dependencies {
    use std::collections::{HashMap, HashSet, VecDeque};
    use std::fs::{File as FsFile, read_to_string};
    use std::path::{Path, PathBuf};

    use proc_macro2::Span;
    use syn::{
        File as SynFile, Ident, Item, ItemMod, ItemUse, UseGlob, UseGroup, UseName, UsePath,
        UseRename, UseTree, parse_file,
        spanned::Spanned,
        visit::{self, Visit},
    };
    use thiserror::Error;

    #[derive(Debug, PartialEq, Eq, Clone, Hash)]
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
        pub module_name: ModuleName,
        pub statement_type: UseStatementType,
    }

    fn should_remove_prefix(import_name: &str) -> bool {
        import_name == "self"
            || import_name
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
    }

    impl NormalizedUseStatement {
        fn get_module(&self) -> ModuleName {
            match &self.statement_type {
                UseStatementType::Simple(name) => {
                    if should_remove_prefix(name) {
                        return ModuleName(self.module_name.0.clone());
                    }
                    ModuleName(format!("{}::{}", self.module_name.0, name))
                }
                UseStatementType::Alias(old, new) => {
                    if should_remove_prefix(old) {
                        return ModuleName(self.module_name.0.clone());
                    }
                    ModuleName(format!("{}::{}", self.module_name.0, old))
                }
                UseStatementType::WildCard => ModuleName(self.module_name.0.clone()),
            }
        }
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
        target_modules: HashSet<ModuleName>,
        /// Where in the source file the use statement is.
        statement: UseStatementDetail,
    }

    pub type UseStatements = Vec<UseStatement>;

    pub fn transform(input_path: &Path, output_path: &Path, use_statements: UseStatements) {
        // Check whether the output path exists or not
        if !output_path.exists() {
            FsFile::create(output_path).expect("Err: failed to create a file");
        }

        // Read the input path content
        let content = read_to_string(input_path).expect("Err: failed to read the file content");
        let syntax: SynFile = syn::parse_file(&content).unwrap();

        let mut output = content.clone();
        for item in syntax.items {
            if let Item::Use(use_item) = item {
                let span = use_item.span();
                let original = quote::quote!(#use_item).to_string();

                if let Some(first_space) = original.find(' ') {
                    let (first_part, rest) = original.split_at(first_space + 1);
                    let split_rest = rest.replace(" ", "");
                    let result = format!("{}{}", first_part, split_rest);

                    let mut source: ModuleName;
                    if let Some(input_str) = input_path.to_str() {
                        source = ModuleName(input_str.to_string());
                    }

                    let mut target: HashSet<ModuleName> = HashSet::new();
                    if let Some(output_str) = output_path.to_str() {
                        target.insert(ModuleName(output_str.to_string()));
                    }

                    let statements = UseStatement {
                        source_module: source,
                        target_modules: target,
                        statement: UseStatementDetail {
                            items: vec![NormalizedUseStatement {
                                module_name: ModuleName("module name".to_string()),
                                statement_type: UseStatementType::Simple(result),
                            }],
                            span: _,
                        },
                    };
                    // output = output.replacen(&result, "todo!();", 1);
                } else {
                    println!("{}", original);
                }
            }
        }
        std::fs::write("output.rs", output).unwrap();
    }

    #[derive(Debug, Hash, PartialEq, Eq)]
    pub struct File(String);

    pub type UseStatementMap = HashMap<File, UseStatements>;

    #[derive(Debug)]
    enum ModStatement {
        External { ident: Ident, span: Span },
        Inline { ident: Ident, span: Span },
    }

    #[derive(Debug)]
    pub struct UseStatementDetail {
        items: Vec<NormalizedUseStatement>,
        span: Span,
    }

    #[derive(Debug)]
    struct Visitor {
        use_statements: Vec<UseStatement>,
        mod_statements: Vec<ModStatement>,
        /// Stack of module identifiers from the crate root through both file-based (`mod foo;`) and inline (`mod bar { … }`) modules
        ancestors: Vec<String>,
    }

    #[derive(Debug)]
    struct FileToVisit {
        file: PathBuf,
        module_ancestors: Vec<String>,
    }

    impl Visitor {
        fn new(ancestors: &[String]) -> Self {
            Self {
                use_statements: Vec::new(),
                mod_statements: Vec::new(),
                ancestors: ancestors.to_owned(),
            }
        }
        fn with_defaults() -> Self {
            Self::new(&[])
        }
    }
    impl Default for Visitor {
        fn default() -> Self {
            Visitor::with_defaults()
        }
    }

    impl<'ast> Visit<'ast> for Visitor {
        fn visit_item_mod(&mut self, node: &'ast ItemMod) {
            if node.content.is_some() {
                self.mod_statements.push(ModStatement::Inline {
                    span: node.span(),
                    ident: node.ident.to_owned(),
                });
            } else {
                self.mod_statements.push(ModStatement::External {
                    span: node.span(),
                    ident: node.ident.to_owned(),
                });
            }
            self.ancestors.push(node.ident.to_string());
            visit::visit_item_mod(self, node);

            self.ancestors.pop();
        }

        fn visit_item_use(&mut self, node: &'ast ItemUse) {
            let items = flatten_use_tree(&self.ancestors, &[], &node.tree);

            let path_segments = std::iter::once("crate".to_string())
                .chain(self.ancestors.iter().cloned())
                .collect::<Vec<_>>();

            self.use_statements.push(UseStatement {
                source_module: path_segments.join("::").into(),
                target_modules: items
                    .iter()
                    .map(|item| item.get_module())
                    .collect::<HashSet<_>>(),
                statement: UseStatementDetail {
                    items,
                    span: node.span(),
                },
            });
        }
    }

    fn flatten_use_tree(
        ancestors: &[String],
        prefix: &[String],
        tree: &UseTree,
    ) -> Vec<NormalizedUseStatement> {
        let desugar_self_import = |ident: &Ident| {
            let mut ret = Vec::new();
            ret.extend_from_slice(prefix);
            if !should_remove_prefix(&ident.to_string()) {
                ret.push(ident.to_string());
                return (ModuleName(ret.join("::")), "self".to_string());
            }
            (ModuleName(ret.join("::")), ident.to_string())
        };
        match tree {
            UseTree::Path(UsePath {
                ident,
                tree: subtree,
                ..
            }) => {
                let current_segment = ident.to_string();
                let mut new_prefix = Vec::new();

                match current_segment.as_str() {
                    "self" => {
                        new_prefix.push("crate".into());
                        new_prefix.extend_from_slice(ancestors);
                    }
                    "super" => {
                        new_prefix.push("crate".into());
                        new_prefix.extend_from_slice(ancestors);
                        new_prefix.pop();
                    }
                    _ => {
                        new_prefix.extend_from_slice(prefix);
                        new_prefix.push(current_segment);
                    }
                }

                flatten_use_tree(ancestors, &new_prefix, subtree)
            }

            UseTree::Name(UseName { ident }) => {
                let (module_name, ident) = desugar_self_import(ident);
                vec![NormalizedUseStatement {
                    module_name,
                    statement_type: UseStatementType::Simple(ident),
                }]
            }
            UseTree::Rename(UseRename { ident, rename, .. }) => {
                let (module_name, ident) = desugar_self_import(ident);
                vec![NormalizedUseStatement {
                    module_name,
                    statement_type: UseStatementType::Alias(ident, rename.to_string()),
                }]
            }
            UseTree::Glob(UseGlob { .. }) => {
                vec![NormalizedUseStatement {
                    module_name: ModuleName(prefix.join("::")),
                    statement_type: UseStatementType::WildCard,
                }]
            }

            UseTree::Group(UseGroup { items, .. }) => items
                .iter()
                .flat_map(|sub| flatten_use_tree(ancestors, prefix, sub))
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
        #[error("linked module does not exist: {0}")]
        ModuleDoesNotExist(String),
        #[error("crate entrypoint not found")]
        CrateEntrypointNotFound,
        #[error("Could not find source file for module {0}")]
        SourceFileForModuleNotFound(String),
    }

    fn get_crate_entrypoint(crate_root: &Path) -> Result<PathBuf, ListUseStatementError> {
        // TODO: support multiple targets and custom paths different than src/main.rs or src/lib.rs

        let cargo_toml = crate_root.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Err(ListUseStatementError::PathIsNotACrate);
        }

        let main_rs = crate_root.join(Path::new("src/main.rs"));
        if main_rs.exists() {
            return Ok(main_rs);
        }

        let lib_rs = crate_root.join(Path::new("src/lib.rs"));
        if lib_rs.exists() {
            return Ok(lib_rs);
        };
        Err(ListUseStatementError::CrateEntrypointNotFound)
    }

    // NOTE: path attribute on mod is currently not supported
    fn mod_to_path(
        crate_root: &Path,
        ancestors: &[String],
        ident: &Ident,
    ) -> Result<PathBuf, ListUseStatementError> {
        let ident = ident.to_string();
        let mut root_path = crate_root.join("src");
        root_path.extend(ancestors);

        let file_module = root_path.join(format!("{}.rs", ident));
        let folder_module = root_path.join(&ident).join("mod.rs");
        if file_module.exists() {
            return Ok(file_module);
        } else if folder_module.exists() {
            return Ok(folder_module);
        }
        Err(ListUseStatementError::SourceFileForModuleNotFound(ident))
    }

    /// List all the `use` statements in the crate, by file/module.
    pub fn list_use_statements(
        crate_root: &Path,
    ) -> Result<UseStatementMap, ListUseStatementError> {
        let mut files_visited = HashSet::new();
        let mut files_to_visit = VecDeque::new();
        let mut use_statement_map: UseStatementMap = HashMap::new();
        let entry_point = get_crate_entrypoint(crate_root)?;
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

            let mut visitor = Visitor::new(&file_to_visit.module_ancestors);
            visitor.visit_file(&parsed_file);

            for mod_statement in visitor.mod_statements {
                if let ModStatement::External { ident, span: _ } = mod_statement {
                    let file = mod_to_path(crate_root, &file_to_visit.module_ancestors, &ident)?;
                    let mut new_ancestors = file_to_visit.module_ancestors.clone();
                    new_ancestors.push(ident.to_string());
                    files_to_visit.push_back(FileToVisit {
                        file,
                        module_ancestors: new_ancestors,
                    })
                }
            }

            use_statement_map.insert(
                File(
                    file_to_visit
                        .file
                        .strip_prefix(crate_root)
                        .unwrap_or(&file_to_visit.file)
                        .to_string_lossy()
                        .to_string(),
                ),
                visitor.use_statements,
            );
            files_visited.insert(file_to_visit.file);
        }

        Ok(use_statement_map)
    }

    pub type ModuleDependencies = HashMap<ModuleName, HashSet<ModuleName>>;

    /// List the dependencies of modules inside the given crate, including circular, based on the use statements.
    pub fn list_dependencies(use_statements: &UseStatementMap) -> ModuleDependencies {
        let mut module_dependencies: ModuleDependencies = HashMap::new();
        for (file, use_statements) in use_statements.iter() {
            for use_statement in use_statements {
                module_dependencies
                    .entry(use_statement.source_module.clone())
                    .or_default()
                    .extend(
                        use_statement
                            .statement
                            .items
                            .iter()
                            .map(|item| item.module_name.clone()),
                    );
            }
        }
        module_dependencies
    }

    #[cfg(test)]
    mod tests {
        use std::{
            collections::{HashMap, HashSet},
            path::Path,
        };

        use pretty_assertions::assert_eq;
        use proc_macro2::{LineColumn, Span};
        use syn::visit::Visit;

        use crate::dependencies::{
            File, ModuleName, NormalizedUseStatement, UseStatement, UseStatementDetail,
            UseStatementType, Visitor, list_dependencies, list_use_statements,
        };

        #[test]
        fn build_dependency_map() {
            let use_statements = HashMap::from([
                (
                    File("main.rs".into()),
                    vec![UseStatement {
                        source_module: ModuleName("crate".into()),
                        target_modules: HashSet::from([ModuleName("".into())]),
                        statement: UseStatementDetail {
                            items: vec![NormalizedUseStatement {
                                module_name: ModuleName("crate::module_a".into()),
                                statement_type: UseStatementType::Simple("Baz".to_string()),
                            }],
                            span: Span::call_site(),
                        },
                    }],
                ),
                (
                    File("module_a/mod.rs".into()),
                    vec![UseStatement {
                        source_module: ModuleName("crate::module_a".into()),
                        target_modules: HashSet::from([ModuleName("".into())]),
                        statement: UseStatementDetail {
                            items: vec![NormalizedUseStatement {
                                module_name: ModuleName("crate::module_b".into()),
                                statement_type: UseStatementType::Simple("Bar".to_string()),
                            }],
                            span: Span::call_site(),
                        },
                    }],
                ),
            ]);
            let dependency_map = HashMap::from([
                (
                    ModuleName("crate".into()),
                    HashSet::from([ModuleName("crate::module_a".into())]),
                ),
                (
                    ModuleName("crate::module_a".into()),
                    HashSet::from([ModuleName("crate::module_b".into())]),
                ),
            ]);
            let module_dependencies = list_dependencies(&use_statements);
            assert_eq!(module_dependencies, dependency_map);
        }

        #[test]
        fn gets_a_simple_dependency() {
            let test_project = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/simple/");
            let res = list_use_statements(&test_project).expect("Failed to list statements");

            let main_statement = &res.get(&File("src/main.rs".to_owned())).unwrap()[0];
            let module_a_statement = &res.get(&File("src/module_a/mod.rs".to_owned())).unwrap()[0];
            let module_b_statement = &res
                .get(&File("src/module_a/module_b.rs".to_owned()))
                .unwrap()[0];
            assert_eq!(main_statement.source_module, "crate".into());
            assert_eq!(
                main_statement.target_modules,
                HashSet::from(["crate::module_a".into()])
            );
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
                    module_name: "crate::module_a".into(),
                    statement_type: UseStatementType::Simple("self".to_owned()),
                }]
            );
            assert_eq!(module_a_statement.source_module, "crate::module_a".into());
            assert_eq!(
                module_a_statement.target_modules,
                HashSet::from(["std::collections".into()])
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
            assert_eq!(
                module_b_statement.source_module,
                "crate::module_a::module_b".into()
            );
            assert_eq!(
                module_b_statement.target_modules,
                HashSet::from(["foo".into()])
            );
            assert_eq!(
                module_b_statement.statement.span.start(),
                LineColumn { line: 1, column: 0 }
            );
            assert_eq!(
                module_b_statement.statement.span.end(),
                LineColumn { line: 1, column: 8 }
            );
            assert_eq!(
                module_b_statement.statement.items,
                vec![NormalizedUseStatement {
                    module_name: "foo".into(),
                    statement_type: UseStatementType::Simple("self".to_owned()),
                }]
            );
        }

        #[test]
        fn gets_structs_dependency() {
            let test_project =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/structs/");
            let res = list_use_statements(&test_project).expect("Failed to list statements");

            let main_statement = &res.get(&File("src/main.rs".to_owned())).unwrap()[0];
            let module_a_statement = &res.get(&File("src/module_a/mod.rs".to_owned())).unwrap()[0];
            assert_eq!(main_statement.source_module, "crate".into());
            assert_eq!(
                main_statement.target_modules,
                HashSet::from(["crate::module_a".into()])
            );
            assert_eq!(
                main_statement.statement.span.start(),
                LineColumn { line: 2, column: 0 }
            );
            assert_eq!(
                main_statement.statement.span.end(),
                LineColumn {
                    line: 2,
                    column: 32
                }
            );
            assert_eq!(
                main_statement.statement.items,
                vec![
                    NormalizedUseStatement {
                        module_name: "crate::module_a".into(),
                        statement_type: UseStatementType::Simple("Bar".to_owned()),
                    },
                    NormalizedUseStatement {
                        module_name: "crate::module_a".into(),
                        statement_type: UseStatementType::Simple("Foo".to_owned()),
                    }
                ]
            );
            assert_eq!(module_a_statement.source_module, "crate::module_a".into());
            assert_eq!(
                module_a_statement.target_modules,
                HashSet::from(["std::collections".into()])
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
        fn build_a_simple_dependency_map() {
            let test_project = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/simple/");
            let use_statements =
                list_use_statements(&test_project).expect("Failed to list statements");
            let module_dependencies = list_dependencies(&use_statements);
            assert_eq!(
                module_dependencies,
                HashMap::from([
                    (
                        ModuleName("crate".into(),),
                        HashSet::from([ModuleName("crate::module_a".into())])
                    ),
                    (
                        ModuleName("crate::module_a".into()),
                        HashSet::from([ModuleName("std::collections".into())])
                    ),
                    (
                        ModuleName("crate::module_a::module_b".into()),
                        HashSet::from([ModuleName("foo".into())])
                    ),
                ])
            );
        }

        #[test]
        fn build_a_simple_dependency_map_with_structs() {
            let test_project =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/structs/");
            let use_statements =
                list_use_statements(&test_project).expect("Failed to list statements");
            let module_dependencies = list_dependencies(&use_statements);
            assert_eq!(
                module_dependencies,
                HashMap::from([
                    (
                        ModuleName("crate".into(),),
                        HashSet::from([ModuleName("crate::module_a".into())])
                    ),
                    (
                        ModuleName("crate::module_a".into()),
                        HashSet::from([ModuleName("std::collections".into())])
                    ),
                ])
            );
        }

        #[test]
        fn gets_a_nested_dependency() {
            let test_project = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/inline/");
            let res = list_use_statements(&test_project).expect("Failed to list statements");

            let main_statement = &res.get(&File("src/main.rs".to_owned())).unwrap()[0];
            let nested_statement = &res.get(&File("src/main.rs".to_owned())).unwrap()[1];
            assert_eq!(main_statement.source_module, "crate::module_a".into());
            assert_eq!(
                main_statement.target_modules,
                HashSet::from(["crate::module_a::module_b".into()])
            );
            assert_eq!(
                main_statement.statement.span.start(),
                LineColumn { line: 2, column: 4 }
            );
            assert_eq!(
                main_statement.statement.span.end(),
                LineColumn {
                    line: 2,
                    column: 34
                }
            );
            assert_eq!(
                main_statement.statement.items,
                vec![NormalizedUseStatement {
                    module_name: "crate::module_a::module_b".into(),
                    statement_type: UseStatementType::Simple("self".to_owned()),
                }]
            );
            assert_eq!(
                nested_statement.source_module,
                "crate::module_a::module_b".into()
            );
            assert_eq!(
                nested_statement.target_modules,
                HashSet::from(["foo".into()])
            );
            assert_eq!(
                nested_statement.statement.span.start(),
                LineColumn { line: 4, column: 8 }
            );
            assert_eq!(
                nested_statement.statement.span.end(),
                LineColumn {
                    line: 4,
                    column: 21
                }
            );
            assert_eq!(
                nested_statement.statement.items,
                vec![NormalizedUseStatement {
                    module_name: "foo".into(),
                    statement_type: UseStatementType::Simple("Bar".to_owned()),
                }]
            );
        }

        #[test]
        fn flattens_alias() {
            let src = "use crate::foo::Bar as Baz;";
            let file = syn::parse_file(src).unwrap();
            let mut visitor = Visitor::default();
            visitor.visit_file(&file);
            let items = &visitor.use_statements[0].statement.items;
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
            let mut visitor = Visitor::default();
            visitor.visit_file(&file);
            let items = &visitor.use_statements[0].statement.items;
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
            let mut visitor = Visitor::default();
            visitor.visit_file(&file);
            let names: Vec<_> = visitor.use_statements[0]
                .statement
                .items
                .iter()
                .map(|i| (&i.module_name, &i.statement_type))
                .collect();
            assert_eq!(
                names,
                vec![
                    (
                        &ModuleName("crate::foo".to_owned()),
                        &UseStatementType::Simple("self".into())
                    ),
                    (
                        &ModuleName("crate::bar::baz".to_owned()),
                        &UseStatementType::Simple("self".into())
                    ),
                    (
                        &ModuleName("crate::bar::qux".to_owned()),
                        &UseStatementType::Simple("self".into())
                    ),
                ]
            );
        }

        #[test]
        fn super_import_resolved() {
            let src = r#"
            mod module_a {
                mod module_b {
                        use super::module_c::Foo;
                    }
                }
            "#;
            let file = syn::parse_file(src).unwrap();
            let mut visitor = Visitor::default();
            visitor.visit_file(&file);

            assert_eq!(visitor.use_statements.len(), 1);

            let detail = &visitor.use_statements[0].statement;
            let items = &detail.items;
            assert_eq!(items.len(), 1);

            assert_eq!(
                items[0],
                NormalizedUseStatement {
                    module_name: "crate::module_a::module_c".into(),
                    statement_type: UseStatementType::Simple("Foo".into()),
                }
            );
        }

        #[test]
        fn self_import_resolved() {
            let src = r#"
                mod module_a {
                    mod module_b {
                        use self::module_c::Foo;
                    }
                }
            "#;
            let file = syn::parse_file(src).unwrap();
            let mut visitor = Visitor::default();
            visitor.visit_file(&file);

            assert_eq!(visitor.use_statements.len(), 1);

            let items = &visitor.use_statements[0].statement.items;
            assert_eq!(items.len(), 1);

            assert_eq!(
                items[0],
                NormalizedUseStatement {
                    module_name: "crate::module_a::module_b::module_c".into(),
                    statement_type: UseStatementType::Simple("Foo".into()),
                }
            );
        }

        #[test]
        fn aliases_resolved() {
            let src = r#"
                use foo::bar as baz;
                use foo::Bar as Baz;
            "#;
            let file = syn::parse_file(src).unwrap();
            let mut visitor = Visitor::default();
            visitor.visit_file(&file);

            assert_eq!(visitor.use_statements.len(), 2);

            assert_eq!(
                visitor.use_statements[0].target_modules,
                HashSet::from([ModuleName("foo::bar".into())])
            );
            assert_eq!(
                visitor.use_statements[0].statement.items,
                vec![NormalizedUseStatement {
                    module_name: "foo::bar".into(),
                    statement_type: UseStatementType::Alias("self".to_string(), "baz".to_string())
                }]
            );
            assert_eq!(
                visitor.use_statements[1].statement.items,
                vec![NormalizedUseStatement {
                    module_name: "foo".into(),
                    statement_type: UseStatementType::Alias("Bar".to_string(), "Baz".to_string())
                }]
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
