pub mod datasheet;
pub mod distribution;
pub mod filter;
pub mod input;
pub mod lua;
pub mod objectstream;
pub mod output;
pub mod vshapec;

use clap::Parser;
use filter::Filter;
use input::Input;
use output::Output;
use rusqlite::Connection;
use std::path::PathBuf;

use crate::traits::{IArgs, IDatabase};

#[derive(Debug, Parser, Clone)]
pub struct CommonConfig {
    #[command(flatten)]
    pub input: Input,
    #[command(flatten)]
    pub output: Output,
    #[command(flatten)]
    pub filter: Filter,
}

impl CommonConfig {}

impl<'a> IArgs<'a> for CommonConfig {
    type Value = &'a Connection;
    fn configure(&mut self, value: Self::Value) -> std::io::Result<()> {
        let last_input = self.input.load(value).unwrap();
        self.input.configure(last_input)?;
        self.input.save(value).unwrap();

        let last = self.output.load(value).unwrap();
        self.output
            .configure((last, nw_type(&self.input.input.as_ref().unwrap())))?;
        self.output.save(value).unwrap();

        // let last = self.filter.load(value).unwrap();
        // self.filter.configure(None)?;
        // self.filter.save(value).unwrap();

        Ok(())
    }
}

fn validate_path(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if path.join(r"Bin64\NewWorld.exe").exists() && path.join(r"assets").exists() {
        Ok(path)
    } else if path.exists() {
        Err("New World does not exist in that path.".into())
    } else {
        Err("Not a valid path".into())
    }
}

fn nw_type(input: &PathBuf) -> &'static str {
    if input
        .to_str()
        .is_some_and(|path| path.to_lowercase().contains("new world marketing"))
    {
        "marketing"
    } else if input
        .to_str()
        .is_some_and(|path| path.to_lowercase().contains("new world public test realm"))
    {
        "ptr"
    } else {
        "live"
    }
}
