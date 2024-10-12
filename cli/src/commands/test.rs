use clap::{Parser, Subcommand};

use crate::common::CommonConfig;

#[derive(Debug, Parser)]
pub struct Test {
    #[command(subcommand)]
    pub commands: TestCommands,
}

#[derive(Subcommand, Debug)]
pub enum TestCommands {
    Filter(CommonConfig),
}
