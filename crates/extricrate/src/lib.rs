#![allow(dead_code, unused_variables)]

pub mod dependencies {
    use std::collections::HashMap;
    use std::fs::read_to_string;

    use quote::ToTokens;
    use syn::visit::{self, Visit};
    use syn::{ItemUse, UseGlob, UseGroup, UseName, UsePath, UseRename, UseTree, parse_file};
    use thiserror::Error;

    #[derive(Debug, PartialEq, Eq)]
    pub struct ModuleName(String);
    impl ModuleName {
        fn new(name: &str) -> Self {
            Self(name.to_owned())
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

    struct UseVisitor;

    impl<'ast> Visit<'ast> for UseVisitor {
        fn visit_item_use(&mut self, node: &'ast ItemUse) {
            let tokens = node.to_token_stream();
            dbg!(flatten_use_tree("", &node.tree));
            visit::visit_item_use(self, node);
        }
    }

    fn flatten_use_tree(prefix: &str, tree: &UseTree) -> Vec<UseStatementType> {
        match tree {
            UseTree::Path(UsePath { ident, tree, .. }) => {
                let new_prefix = if prefix.is_empty() {
                    ident.to_string()
                } else {
                    format!("{}::{}", prefix, ident)
                };
                flatten_use_tree(&new_prefix, tree)
            }

            UseTree::Name(UseName { ident, .. }) => {
                let full = if prefix.is_empty() {
                    ident.to_string()
                } else {
                    format!("{}::{}", prefix, ident)
                };
                vec![UseStatementType::Simple(full)]
            }

            UseTree::Rename(UseRename { ident, rename, .. }) => {
                let full = if prefix.is_empty() {
                    ident.to_string()
                } else {
                    format!("{}::{}", prefix, ident)
                };
                vec![UseStatementType::Alias(full, rename.to_string())]
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
        if !crate_root.exists() {
            return Err(ListUseStatementError::FileNotFound);
        }
        let content =
            read_to_string(crate_root).map_err(|_| ListUseStatementError::FileNotReadable)?;
        let parsed_crate_root =
            parse_file(&content).map_err(|_| ListUseStatementError::FileNotParsable)?;

        UseVisitor.visit_file(&parsed_crate_root);

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
                    source_module: ModuleName::new("main"),
                    target_modules: vec![ModuleName::new("std::collections::HashMap")],
                    extent: Extent {
                        start: Position { line: 1, col: 1 },
                        end: Position { line: 1, col: 31 },
                    },
                    normalized_statements: vec![NormalizedUseStatement {
                        module_name: ModuleName::new("main"),
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
