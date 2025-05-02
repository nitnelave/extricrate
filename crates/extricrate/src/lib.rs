#![allow(dead_code, unused_variables)]

pub mod dependencies {
    use std::collections::HashMap;

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

    #[derive(Debug, PartialEq, Eq)]
    pub enum UseStatementType {
        /// `use crate::log::Bar;`
        Simple(String),
        /// `use crate::log::Bar as Baz;`
        Alias(String, String),
        /// `use crate::log::*;`
        WildCard,
    }

    /// A single, separate use statement.
    #[derive(Debug, PartialEq, Eq)]
    pub struct NormalizedUseStatement {
        module_name: ModuleName,
        statement_type: UseStatementType,
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

    /// List all the `use` statements in the crate, by file/module.
    pub fn list_use_statements(crate_root: &std::path::Path) -> UseStatementMap {
        todo!()
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
            let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/simple");
            let res = list_use_statements(&fixture);
            let mut expected = HashMap::new();
            expected.insert(
                File("src/main.rs".to_owned()),
                vec![UseStatement {
                    source_module: ModuleName::new("main"),
                    target_modules: vec![ModuleName::new("std::collections::HashMap")],
                    extent: Extent {
                        start: Position { line: 1, col: 0 },
                        end: Position { line: 1, col: 30 },
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
