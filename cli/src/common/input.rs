use crate::{
    traits::{IArgs, IDatabase},
    STEAM_DIR,
};
use clap::Parser;
use rusqlite::{params, Connection, OptionalExtension};
use std::{path::PathBuf, str::FromStr};

use super::validate_path;

#[derive(Debug, Parser, Clone)]
pub struct Input {
    /// New World root directory. Needs to be root, not ./assets as it looks for the bin for parsing strings.
    #[arg(short, long, value_parser = validate_path)]
    pub input: Option<PathBuf>,
}

impl<'a> IArgs<'a> for Input {
    type Value = Option<String>;

    fn configure(&mut self, value: Self::Value) -> std::io::Result<()> {
        if self.input.is_none() {
            let input: PathBuf = cliclack::input("New World Directory")
                .default_input(&value.unwrap_or_else(|| STEAM_DIR.to_string()))
                .validate_interactively(|path: &String| match PathBuf::from_str(path) {
                    Ok(p) => {
                        if p.join(r"Bin64\NewWorld.exe").exists() && p.join(r"assets").exists() {
                            Ok(())
                        } else if p.exists() {
                            Err("New World does not exist in that path.")
                        } else {
                            Err("Not a valid path")
                        }
                    }
                    _ => Err("Not a valid path"),
                })
                .interact()?;
            self.input = Some(input);
        };
        Ok(())
    }
}

impl IDatabase for Input {
    fn load(&self, conn: &Connection) -> rusqlite::Result<Option<String>> {
        conn.query_row(
            "select value from configs where name = ?",
            ["input"],
            |row| row.get(0),
        )
        .optional()
    }

    fn save(&self, conn: &Connection) -> rusqlite::Result<usize> {
        conn.execute(
            r#"insert or replace into configs (name, value) values ("input", ?)"#,
            params![self.input.as_ref().unwrap().to_str()],
        )
    }
}
