use clap::Parser;
use regex::Regex;
use rusqlite::params;
use std::io;

use crate::{
    common::{
        datasheet::{DatasheetConfig, DatasheetFormat, DatasheetOutputMode},
        objectstream::{ObjectStreamConfig, ObjectStreamFormat},
        CommonConfig,
    },
    traits::IArgs,
    BYTES, CSV, MINI, PRETTY, SQL, XML, YAML,
};

#[derive(Debug, Parser)]
pub struct Extract {
    #[command(flatten)]
    pub common: CommonConfig,
    #[command(flatten)]
    pub datasheet: DatasheetConfig,
    #[command(flatten)]
    pub objectstream: ObjectStreamConfig,
}

impl<'a> IArgs<'a> for Extract {
    type Value = ();

    fn configure(&mut self, _: Self::Value) -> io::Result<()> {
        let config_dir = dirs::config_local_dir().unwrap().join(".nwtools");
        let conn = rusqlite::Connection::open(config_dir).unwrap();

        conn.pragma_update_and_check(None, "journal_mode", "WAL", |_res| Ok(()))
            .unwrap();

        conn.pragma_update(None, "synchronous", 1).unwrap();

        conn
        .execute(
            "create table if not exists configs (name text primary key, value text) strict, without rowid",
            params![],
        )
        .unwrap();

        self.common.configure(&conn)?;

        if self.common.filter.filter.is_none()
            && self.objectstream.objectstream == ObjectStreamFormat::BYTES
            && self.datasheet.datasheet == DatasheetFormat::BYTES
        {
            let is_default = cliclack::confirm("Use defaults?")
                .initial_value(true)
                .interact()?;

            if !is_default {
                let options = cliclack::multiselect(
                    "Select options to adjust next. (Space to toggle. Enter to submit.)",
                )
                .items(&[
                    ("filter", "Filter", ""),
                    (
                        "datasheet",
                        "Datasheet Format",
                        &format!(
                            "{} = default | {} | {} | {} | {}",
                            BYTES, MINI, PRETTY, YAML, CSV
                        ),
                    ),
                    (
                        "datasheet-output-mode",
                        "Datasheet Output Mode",
                        "original = default, typename",
                    ),
                    // ("with-meta")
                    (
                        "objectstream",
                        "ObjectStream",
                        &format!("{} = default | {} | {} | {}", BYTES, XML, MINI, PRETTY),
                    ),
                ])
                .interact()?;

                if options.contains(&"filter") {
                    let rx: Regex = cliclack::input("Include").required(false).interact()?;
                    if rx.as_str() == Regex::new("").unwrap().as_str() {
                        self.common.filter.filter = None;
                    } else {
                        self.common.filter.filter = Some(rx)
                    }
                }

                let filter = self.common.filter.filter.as_ref();
                conn.execute(
                    r#"insert or replace into configs (name, value) values ("filter", ?)"#,
                    params![filter.map(|v| v.as_str())],
                )
                .unwrap();
                if options.contains(&"objectstream") {
                    let obj_stream = cliclack::Select::new("ObjectStream Format")
                        .items(&[
                            (BYTES, "Binary", "default"),
                            (XML, "XML", ""),
                            (PRETTY, "JSON Pretty", ""),
                            (MINI, "JSON Minified", ""),
                        ])
                        .initial_value("bytes")
                        .interact()?;

                    self.objectstream.objectstream = match obj_stream {
                        XML => ObjectStreamFormat::XML,
                        MINI => ObjectStreamFormat::MINI,
                        PRETTY => ObjectStreamFormat::PRETTY,
                        _ => ObjectStreamFormat::BYTES,
                    };
                }

                if options.contains(&"datasheet") {
                    let datasheet = cliclack::Select::new("Datasheet Format")
                        .items(&[
                            (BYTES, "Binary", "default"),
                            (MINI, "JSON Minified", ""),
                            (PRETTY, "JSON Pretty", ""),
                            (CSV, "CSV", ""),
                            (YAML, "YAML", "vomit"),
                        ])
                        .initial_value("bytes")
                        .interact()?;

                    self.datasheet.datasheet = match datasheet {
                        MINI => DatasheetFormat::MINI,
                        PRETTY => DatasheetFormat::PRETTY,
                        CSV => DatasheetFormat::CSV,
                        YAML => DatasheetFormat::YAML,
                        SQL => DatasheetFormat::SQL,
                        _ => DatasheetFormat::BYTES,
                    };
                }

                if options.contains(&"datasheet-output-mode") {
                    let mode = cliclack::Select::new("Datasheet Output Mode")
                        .items(&[
                            ("original", "Original", "datatables/javelindata_[name]"),
                            (
                                "typename",
                                "Type Name",
                                "datatables/[Table Type]/[Table Name]",
                            ),
                        ])
                        .initial_value("original")
                        .interact()?;
                    self.datasheet.datasheet_filenames = match mode {
                        "typename" => DatasheetOutputMode::TYPENAME,
                        _ => DatasheetOutputMode::ORIGINAL,
                    };
                }
            }
        }
        conn.close().unwrap();
        Ok(())
    }
}
