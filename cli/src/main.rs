#![allow(dead_code, unused_variables)]

use clap::Parser;
use extricrate::transform::transform;
use std::path::Path;

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
    /// Module to extract from a crate.
    #[clap(long, env = "EXTRICRATE_MODULE")]
    pub module: String,
    /// Target crate to create.
    #[clap(long, env = "EXTRICRATE_CRATE_NAME")]
    pub crate_name: String,
}

mod logging;

fn main() {
    let opts = CLIOpts::parse();
    logging::init();
    match opts.command {
        Command::ListDependencies(opts) => todo!(),
        Command::Extract(opts) => transform(Path::new(&opts.module), Path::new(&opts.crate_name)),
    }
}
