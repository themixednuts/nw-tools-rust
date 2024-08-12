use clap::Subcommand;
use extract::Extract;

pub mod extract;

#[derive(Subcommand, Debug)]
pub enum Commands {
    Extract(Extract),
}
