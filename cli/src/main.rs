#![allow(dead_code, unused_variables)]

use std::path::Path;

use clap::Parser;
use extricrate::dependencies::{
    ModuleName, NormalizedUseStatement, UseStatement, UseStatementDetail, UseStatementType,
};

/// Extricrate is a refactoring tool to extract a crate.
#[derive(Debug, Parser, Clone)]
#[clap(version, author)]
pub struct CLIOpts {
    /// Export
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Parser, Clone)]
pub enum Command {
    /// List the modules and their in-crate dependencies.
    #[clap(name = "list_dependencies")]
    ListDependencies(ListDependenciesOpts),
    /// Extract a module to a separate crate.
    #[clap(name = "extract")]
    Extract(ExtractOpts),
}

#[derive(Debug, Parser, Clone)]
pub struct ListDependenciesOpts {
    /// Module to list dependencies for. Defaults to all the modules.
    #[clap(long, env = "EXTRICRATE_MODULE")]
    pub module: Option<String>,
}

#[derive(Debug, Parser, Clone)]
pub struct ExtractOpts {
    /// Module to extract to a crate.
    #[clap(long, env = "EXTRICRATE_MODULE")]
    pub module: String,
    /// Target crate to create.
    #[clap(long, env = "EXTRICRATE_CRATE_NAME")]
    pub crate_name: String,
}

mod logging;
mod transform;

fn main() {
    let opts = CLIOpts::parse();
    logging::init();

    let statements = UseStatement {
        source_module: ModuleName("module name".to_string()),
        target_modules: vec![ModuleName("target module name".to_string())],
        statement: UseStatementDetail {
            items: vec![NormalizedUseStatement {
                module_name: ModuleName("module name".to_string()),
                statement_type: UseStatementType::Simple("use crate::log::Bar;".to_string()),
            }],
            span: _,
        },
    };

    match opts.command {
        Command::ListDependencies(opts) => todo!(),
        Command::Extract(opts) => transform::transform(
            Path::new(&opts.module),
            Path::new(&opts.crate_name),
            vec![statements],
        ),
    }
}
