use clap::{Parser, Subcommand};

use crate::common::{filter::Filter, input::Input};

#[derive(Debug, Parser)]
pub struct Test {
    #[command(subcommand)]
    pub commands: TestCommands,
}

#[derive(Subcommand, Debug)]
pub enum TestCommands {
    Filter {
        #[command(flatten)]
        input: Input,
        #[command(flatten)]
        filter: Filter,
    },
    Distribution {
        #[command(flatten)]
        input: Input,
    },
}
