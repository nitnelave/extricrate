#![allow(dead_code, unused_variables)]

pub mod dependencies {
    use std::collections::{HashMap, HashSet, VecDeque};
    use std::fs::read_to_string;

    use quote::ToTokens;
    use syn::visit::{self, Visit};
    use syn::{
        Ident, ItemUse, UseGlob, UseGroup, UseName, UsePath, UseRename, UseTree, parse_file,
    };
    use thiserror::Error;

    #[derive(Debug, PartialEq, Eq)]
    pub struct ModuleName(String);
    impl ModuleName {
        fn new(name: String) -> Self {
            Self(name)
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    pub struct Position {
        line: u32,
        col: u32,
    }

    #[derive(Debug, PartialEq, Eq)]
    pub struct Extent {
        start: Position,
        // Inclusive
        end: Position,
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

    #[derive(Debug, PartialEq, Eq)]
    pub struct UseStatement {
        /// Where the use statement appears.
        source_module: ModuleName,
        /// List of referenced inner modules.
        /// Several targets, to represent `use crate::{log, foo::{bar, baz}};`
        target_modules: Vec<ModuleName>,
        /// Where in the source file the use statement is.
        extent: Extent,
        normalized_statements: Vec<NormalizedUseStatement>,
    }

    pub type UseStatements = Vec<UseStatement>;

    #[derive(Debug, Hash, PartialEq, Eq)]
    pub struct File(String);

    pub type UseStatementMap = HashMap<File, UseStatements>;

    #[derive(Debug)]
    struct UseVisitor {
        dependencies: Vec<UseStatementType>,
    }
    impl UseVisitor {
        fn new() -> Self {
            Self {
                dependencies: Vec::new(),
            }
        }
    }

    impl<'ast> Visit<'ast> for UseVisitor {
        fn visit_item_use(&mut self, node: &'ast ItemUse) {
            let tokens = node.to_token_stream();
            let mut items = flatten_use_tree("", &node.tree);
            self.dependencies.append(&mut items);
            visit::visit_item_use(self, node);
        }
    }

    fn flatten_use_tree(prefix: &str, tree: &UseTree) -> Vec<UseStatementType> {
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
                let full_name = prefixed(ident);
                vec![UseStatementType::Simple(full_name)]
            }

            UseTree::Rename(UseRename { ident, rename, .. }) => {
                let full_name = prefixed(ident);
                vec![UseStatementType::Alias(full_name, rename.to_string())]
            }

            UseTree::Glob(UseGlob { .. }) => {
                vec![UseStatementType::WildCard]
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
    }

    /// List all the `use` statements in the crate, by file/module.
    pub fn list_use_statements(
        crate_root: &std::path::Path,
    ) -> Result<UseStatementMap, ListUseStatementError> {
        let mut files_visited = HashSet::new();
        let mut files_to_visit = VecDeque::new();
        files_to_visit.push_back(crate_root);
        while let Some(file_to_visit) = files_to_visit.pop_front() {
            if files_visited.contains(file_to_visit) {
                continue;
            }

            if !file_to_visit.exists() {
                return Err(ListUseStatementError::FileNotFound);
            }

            let content = read_to_string(file_to_visit)
                .map_err(|_| ListUseStatementError::FileNotReadable)?;

            let parsed_file =
                parse_file(&content).map_err(|_| ListUseStatementError::FileNotParsable)?;

            let mut visitor = UseVisitor::new();
            visitor.visit_file(&parsed_file);
            for dependency in visitor.dependencies {
                // TODO: check if the dependency is local to the crate. if so, and it hasn't been
                // visited before, add it to the list
            }
            files_visited.insert(file_to_visit);
        }

        todo!();
    }

    pub type ModuleDependencies = HashMap<ModuleName, Vec<ModuleName>>;

    /// List the (circular?) dependencies of modules inside the given crate, based on the use statements.
    pub fn list_dependencies(use_statements: &UseStatementMap) -> ModuleDependencies {
        todo!()
    }

    #[cfg(test)]
    mod tests {
        use std::{collections::HashMap, path::Path};

        use pretty_assertions::assert_eq;

        use crate::dependencies::{
            Extent, File, ModuleName, NormalizedUseStatement, Position, UseStatement,
            UseStatementType, list_use_statements,
        };

        #[test]
        fn get_simple_dependency() {
            let test_project =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/simple/main.rs");
            let res = list_use_statements(&test_project).expect("Failed to list statements");
            let mut expected = HashMap::new();
            expected.insert(
                File("src/main.rs".to_owned()),
                vec![UseStatement {
                    source_module: ModuleName::new("main".to_owned()),
                    target_modules: vec![ModuleName::new("std::collections::HashMap".to_owned())],
                    extent: Extent {
                        start: Position { line: 1, col: 1 },
                        end: Position { line: 1, col: 31 },
                    },
                    normalized_statements: vec![NormalizedUseStatement {
                        module_name: ModuleName::new("main".to_owned()),
                        statement_type: UseStatementType::Simple(
                            "std::collections::HashMap".to_owned(),
                        ),
                    }],
                }],
            );
            assert_eq!(res, expected);
        }
    }
}

pub mod refactor {
    use crate::dependencies::{ModuleName, UseStatementMap};

    pub fn extract_crate(
        crate_root: &std::path::Path,
        module: &ModuleName,
        target_crate_name: &str,
        target_crate_root: &std::path::Path,
        use_statements: &UseStatementMap,
    ) {
        // Should probably return errors.
        todo!()
    }
}
