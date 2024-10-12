use clap::Parser;
use rusqlite::{params, OptionalExtension};

use crate::traits::{IArgs, IDatabase};

#[derive(Debug, Parser, Clone)]
pub struct Filter {
    #[arg(short, long)]
    /// Filter file names with a glob
    pub filter: Option<String>,
}

impl<'a> IArgs<'a> for Filter {
    type Value = Option<String>;

    fn configure(&mut self, value: Self::Value) -> std::io::Result<()> {
        // if self.filter.is_none() {
        //     let filter: String = cliclack::input("Extract Directory")
        //         .default_input(&value.unwrap_or_else(|| "".into()).as_str())
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
