use std::path::PathBuf;

use clap::Parser;
use dirs::document_dir;
use rusqlite::{params, OptionalExtension};

use crate::traits::{IArgs, IDatabase};

#[derive(Debug, Parser, Clone)]
pub struct Output {
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

impl<'a> IArgs<'a> for Output {
    type Value = (Option<String>, &'static str);

    fn configure(&mut self, value: Self::Value) -> std::io::Result<()> {
        let docs_dir = document_dir().unwrap();

        if self.output.is_none() {
            let output: PathBuf = cliclack::input("Extract Directory")
                .default_input(&value.0.unwrap_or_else(|| {
                    docs_dir
                        .join(r"nw\".to_owned() + value.1)
                        .to_str()
                        .unwrap_or_default()
                        .to_string()
                }))
                .interact()?;
            self.output = Some(output);
        }
        Ok(())
    }
}

impl IDatabase for Output {
    fn load(&self, conn: &rusqlite::Connection) -> rusqlite::Result<Option<String>> {
        conn.query_row(
            "select value from configs where name = ?",
            ["output"],
            |row| row.get(0),
        )
        .optional()
    }

    fn save(&self, conn: &rusqlite::Connection) -> rusqlite::Result<usize> {
        conn.execute(
            r#"insert or replace into configs (name, value) values ("output", ?)"#,
            params![self.output.as_ref().unwrap().to_str()],
        )
    }
}
