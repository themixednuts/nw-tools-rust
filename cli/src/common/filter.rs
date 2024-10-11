use clap::Parser;
use regex::Regex;
use rusqlite::{params, OptionalExtension};

use crate::traits::{IArgs, IDatabase};

#[derive(Debug, Parser)]
pub struct Filter {
    #[arg(short, long)]
    /// Filter file names with regex
    pub filter: Option<Regex>,
}

impl<'a> IArgs<'a> for Filter {
    type Value = Option<Regex>;

    fn configure(&mut self, _: Self::Value) -> std::io::Result<()> {
        // if self.filter.is_none() {
        //     let filter: Regex = cliclack::input("Extract Directory")
        //         .default_input(&value.unwrap_or_else(|| Regex::new("").unwrap()).as_str())
        //         .interact()?;
        //     self.filter = Some(filter);
        // }
        Ok(())
    }
}

impl IDatabase for Filter {
    fn load(&self, conn: &rusqlite::Connection) -> rusqlite::Result<Option<String>> {
        conn.query_row(
            "select value from configs where name = ?",
            ["filter"],
            |row| row.get(0),
        )
        .optional()
    }

    fn save(&self, conn: &rusqlite::Connection) -> rusqlite::Result<usize> {
        conn.execute(
            r#"insert or replace into configs (name, value) values ("filter", ?)"#,
            params![self.filter.as_ref().unwrap().to_string()],
        )
    }
}
