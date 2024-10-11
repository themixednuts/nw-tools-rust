use clap::{Parser, ValueEnum};
use rusqlite::Connection;

use crate::traits::IArgs;

#[derive(Debug, Parser)]
pub struct DatasheetConfig {
    #[arg(long, default_value = "bytes")]
    pub datasheet: DatasheetFormat,
    #[arg(long, default_value = "original")]
    /// Save datasheet filenames as
    pub datasheet_filenames: DatasheetOutputMode,
    #[arg(long)]
    pub datasheet_schema: bool,
}

impl<'a> IArgs<'a> for DatasheetConfig {
    type Value = &'a Connection;

    fn configure(&mut self, _: Self::Value) -> std::io::Result<()> {
        todo!()
    }
}

#[derive(ValueEnum, Debug, Clone, Default, PartialEq, Eq)]
pub enum DatasheetFormat {
    #[default]
    BYTES,
    XML,
    JSON,
    PRETTY,
    CSV,
    YAML,
    SQL,
}

impl<'a> IArgs<'a> for DatasheetFormat {
    type Value = ();

    fn configure(&mut self, _: Self::Value) -> std::io::Result<()> {
        todo!()
    }
}
#[derive(ValueEnum, Debug, Clone, Default, PartialEq, Eq)]
pub enum DatasheetOutputMode {
    #[default]
    ORIGINAL,
    /// <TableType>/<TableName>
    TYPENAME,
}
