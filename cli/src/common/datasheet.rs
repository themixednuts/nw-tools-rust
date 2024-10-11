use clap::{Parser, ValueEnum};
use rusqlite::Connection;

use crate::{traits::IArgs, BYTES, CSV, JSON_MINI, JSON_PRETTY, SQL, XML, YAML};

#[derive(Debug, Parser)]
pub struct DatasheetConfig {
    #[arg(long, value_enum, default_value_t)]
    pub datasheet: DatasheetFormat,
    #[arg(long, value_enum, default_value_t)]
    /// Save datasheet filenames as
    pub datasheet_filenames: DatasheetOutputMode,
    #[arg(long)]
    pub with_meta: bool,
    #[arg(long, value_enum, default_value_t)]
    pub inline_locale: Localization,
}

#[derive(ValueEnum, Debug, Clone, Default, PartialEq, Eq)]
pub enum Localization {
    #[default]
    EN,
    ES,
    IT,
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
    /// Minified JSON
    MINI,
    /// Pretty JSON
    PRETTY,
    CSV,
    YAML,
    SQL,
}

impl ToString for DatasheetFormat {
    fn to_string(&self) -> String {
        match self {
            DatasheetFormat::BYTES => BYTES,
            DatasheetFormat::XML => XML,
            DatasheetFormat::MINI => JSON_MINI,
            DatasheetFormat::PRETTY => JSON_PRETTY,
            DatasheetFormat::CSV => CSV,
            DatasheetFormat::YAML => YAML,
            DatasheetFormat::SQL => SQL,
        }
        .into()
    }
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
