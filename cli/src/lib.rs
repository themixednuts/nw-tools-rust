pub mod commands;
pub mod common;
mod traits;

use clap::{self, Parser};
use commands::Commands;
use std::{io, sync::LazyLock};
use traits::IArgs;

pub static ARGS: LazyLock<Args> = LazyLock::new(|| match cli() {
    Ok(args) => args,
    Err(_) => std::process::exit(0),
});

const STEAM_DIR: &'static str = r#"C:\Program Files (x86)\Steam\steamapps\common\New World"#;
const PRETTY: &'static str = "json";
const MINI: &'static str = "mini";
const XML: &'static str = "xml";
const CSV: &'static str = "csv";
const SQL: &'static str = "sql";
const BYTES: &'static str = "bytes";
const YAML: &'static str = "yaml";

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

fn cli() -> io::Result<Args> {
    ctrlc::set_handler(move || {
        cliclack::outro_cancel("Operation cancelled.").unwrap();
        std::process::exit(0);
    })
    .expect("setting Ctrl-C handler");
    let mut args = Args::parse();

    cliclack::clear_screen()?;
    cliclack::intro("New World Tools")?;

    match &mut args.command {
        Commands::Extract(ext) => ext.configure(())?,
    };

    Ok(args)
}
