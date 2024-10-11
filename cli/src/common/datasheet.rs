use std::fmt::Display;

use clap::{Parser, ValueEnum};
use rusqlite::Connection;

use crate::{traits::IArgs, BYTES, CSV, MINI, PRETTY, SQL, XML, YAML};

#[derive(Debug, Parser)]
pub struct DatasheetConfig {
    #[arg(long, value_enum, default_value_t)]
    pub datasheet: DatasheetFormat,
    #[arg(long, value_enum, default_value_t)]
    /// Save datasheet filenames as
    pub datasheet_filenames: DatasheetOutputMode,
    #[arg(long)]
    pub with_meta: bool,
    #[arg(long, value_enum)]
    pub inline_locale: Option<Localization>,
}

#[derive(ValueEnum, Debug, Clone, Default, PartialEq, Eq)]
pub enum Localization {
    #[default]
    EN,
    ES,
    IT,
    DE,
    MX,
    FR,
    PL,
    BR,
}

impl Display for Localization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Localization::EN => "en-us",
            Localization::ES => "en-es",
            Localization::IT => "it-it",
            Localization::DE => "de-de",
            Localization::MX => "es-mx",
            Localization::FR => "fr-fr",
            Localization::PL => "pl-pl",
            Localization::BR => "pt-br",
        };
        write!(f, "{}", value)
    }
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

impl Display for DatasheetFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            DatasheetFormat::BYTES => BYTES,
            DatasheetFormat::XML => XML,
            DatasheetFormat::MINI => MINI,
            DatasheetFormat::PRETTY => PRETTY,
            DatasheetFormat::CSV => CSV,
            DatasheetFormat::YAML => YAML,
            DatasheetFormat::SQL => SQL,
        };
        write!(f, "{}", value)
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
