use clap::Subcommand;
use extract::Extract;
use test::Test;

pub mod extract;
pub mod test;

#[derive(Subcommand, Debug)]
pub enum Commands {
    Extract(Extract),
    Test(Test),
}
