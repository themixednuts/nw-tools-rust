use clap::{Parser, ValueEnum};
use dirs::home_dir;
use regex::Regex;
use rusqlite::{params, OptionalExtension};
use std::{io, path::PathBuf, str::FromStr};

use crate::{traits::InteractiveArgs, STEAM_DIR};

#[derive(Debug, Parser)]
pub struct Extract {
    /// New World root directory. Needs to be root, not ./assets as it looks for the bin for parsing strings.
    #[arg(short, long, value_parser = validate_path)]
    pub input: Option<PathBuf>,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(short, long)]
    /// Filter file names with regex
    pub filter: Option<Regex>,
    #[arg(short = 's', long, default_value = "bytes")]
    pub objectstream: ObjectStreamFormat,
    #[arg(short, long, default_value = "bytes")]
    pub datasheet: DatasheetFormat,
}

#[derive(ValueEnum, Debug, Clone, Default, PartialEq, Eq)]
pub enum DatasheetFormat {
    #[default]
    BYTES,
    XML,
    JSON,
    JSONPRETTY,
    CSV,
    YAML,
}
#[derive(ValueEnum, Debug, Clone, Default, PartialEq, Eq)]
pub enum ObjectStreamFormat {
    #[default]
    BYTES,
    XML,
    JSON,
    JSONPRETTY,
    // CSV,
    // YAML,
}

impl InteractiveArgs for Extract {
    fn interactive(&mut self) -> io::Result<()> {
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

        let last_input: Option<String> = conn
            .query_row(
                "select value from configs where name = ?",
                ["input"],
                |row| row.get(0),
            )
            .optional()
            .unwrap();

        if self.input.is_none() {
            let input: PathBuf = cliclack::input("New World Directory")
                .default_input(&last_input.unwrap_or_else(|| STEAM_DIR.to_string()))
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

        let input = self.input.as_ref().unwrap();
        conn.execute(
            r#"insert or replace into configs (name, value) values ("input", ?)"#,
            params![input.to_str()],
        )
        .unwrap();

        let nw_type = if input
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
        };

        let home_dir = home_dir().unwrap();

        let last_output: Option<String> = conn
            .query_row(
                "select value from configs where name = ?",
                ["output"],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        if self.output.is_none() {
            let output: PathBuf = cliclack::input("Extract Directory")
                .default_input(&last_output.unwrap_or_else(|| {
                    home_dir
                        .join(r"Documents\nw\".to_owned() + nw_type)
                        .to_str()
                        .unwrap_or_default()
                        .to_string()
                }))
                .interact()?;
            self.output = Some(output);
        }
        let output = self.output.as_ref().unwrap();
        conn.execute(
            r#"insert or replace into configs (name, value) values ("output", ?)"#,
            params![output.to_str()],
        )
        .unwrap();

        if self.filter.is_none()
            && self.objectstream == ObjectStreamFormat::BYTES
            && self.datasheet == DatasheetFormat::BYTES
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
                        "Datasheet",
                        "bytes = default | json | jsonpretty | yaml | csv",
                    ),
                    ("object", "ObjectStream", "bytes = default | xml | json"),
                ])
                .interact()?;

                if options.contains(&"filter") {
                    let rx: Regex = cliclack::input("Include").required(false).interact()?;
                    if rx.as_str() == Regex::new("").unwrap().as_str() {
                        self.filter = None;
                    } else {
                        self.filter = Some(rx)
                    }
                }

                let filter = self.filter.as_ref();
                conn.execute(
                    r#"insert or replace into configs (name, value) values ("filter", ?)"#,
                    params![filter.map(|v| v.as_str())],
                )
                .unwrap();
                if options.contains(&"object") {
                    let obj_stream = cliclack::Select::new("ObjectStream Format")
                        .items(&[
                            ("bytes", "Binary", "default"),
                            ("xml", "XML", ""),
                            ("json", "JSON", ""),
                            ("pretty", "JSON Pretty", ""),
                        ])
                        .initial_value("bytes")
                        .interact()?;

                    self.objectstream = match obj_stream {
                        "xml" => ObjectStreamFormat::XML,
                        "json" => ObjectStreamFormat::JSON,
                        "pretty" => ObjectStreamFormat::JSONPRETTY,
                        _ => ObjectStreamFormat::BYTES,
                    };
                }

                if options.contains(&"datasheet") {
                    let datasheet = cliclack::Select::new("Datasheet Format")
                        .items(&[
                            ("bytes", "Binary", "default"),
                            ("json", "JSON", ""),
                            ("pretty", "JSON Pretty", ""),
                            ("csv", "CSV", ""),
                            ("yaml", "YAML", "vomit"),
                        ])
                        .initial_value("bytes")
                        .interact()?;

                    self.datasheet = match datasheet {
                        "json" => DatasheetFormat::JSON,
                        "pretty" => DatasheetFormat::JSONPRETTY,
                        "csv" => DatasheetFormat::CSV,
                        "yaml" => DatasheetFormat::YAML,
                        _ => DatasheetFormat::BYTES,
                    };
                }
            }
        }
        conn.close().unwrap();
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
