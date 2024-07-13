use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(arg_required_else_help=true, version, about, long_about = None)]
pub struct Cli {
    /// paths in which to recursively search for dead files
    pub paths: Vec<PathBuf>,
}
