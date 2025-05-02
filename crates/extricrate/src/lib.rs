#![allow(dead_code, unused_variables)]

pub mod dependencies {
    use std::collections::HashMap;

    pub struct ModuleName(String);

    pub struct Position {
        line: u32,
        col: u32,
    }

    pub struct Extent {
        start: Position,
        // Inclusive
        end: Position,
    }

    pub enum UseStatementType {
        /// `use crate::log::Bar;`
        Simple(String),
        /// `use crate::log::Bar as Baz;`
        Alias(String, String),
        /// `use crate::log::*;`
        WildCard,
    }

    /// A single, separate use statement.
    pub struct NormalizedUseStatement {
        module_name: ModuleName,
        statement_type: UseStatementType,
    }

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
        use pretty_assertions::assert_eq;

        #[test]
        fn sample_test() {
            assert_eq!(1, 1);
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
