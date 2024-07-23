use clap::{self, Parser, Subcommand};
use dirs::{self, home_dir};
use regex::Regex;
use std::{path::PathBuf, str::FromStr, sync::OnceLock};

pub static ARGS: OnceLock<Args> = OnceLock::new();

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub input: Option<PathBuf>,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(short, long)]
    pub filter: Option<Regex>,
    #[arg(short, long)]
    pub exclude: Option<Regex>,
}

pub fn interactive() -> std::io::Result<&'static Args> {
    // ctrlc::set_handler(move || {
    //     outro_cancel("Cancelled").unwrap();
    // })
    // .expect("setting Ctrl-C handler");
    let mut args = Args::parse();
    cliclack::clear_screen()?;
    cliclack::intro("New World Tools")?;

    if args.input.is_none() {
        let input: PathBuf = cliclack::input("New World Directory")
            .default_input(&std::env::var("NW_DIR").unwrap_or_else(|_| {
                r"C:\Program Files (x86)\Steam\steamapps\common\New World".to_string()
            }))
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
        args.input = Some(input);
    };

    let input = args.input.as_ref().unwrap();

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

    if args.output.is_none() {
        let output: PathBuf = cliclack::input("Extract Directory")
            .default_input(
                home_dir
                    .join(r"Documents\nw\".to_owned() + nw_type)
                    .to_str()
                    .unwrap_or_default(),
            )
            .interact()?;
        args.output = Some(output);
    }

    let is_default = cliclack::confirm("Use defaults?")
        .initial_value(true)
        .interact()?;

    if !is_default {
        let options = cliclack::multiselect(
            "Select options to adjust next. (Space to toggle. Enter to submit.)",
        )
        .items(&[
            ("filter", "Filter", ""),
            ("exclude", "Exclude", ""),
            (
                "fmt",
                "Formats",
                "defaults -> ObjectStream[xml] | Datasheet[json]",
            ),
        ])
        .interact()?;

        if options.contains(&"filter") && args.filter.is_none() {
            let rx: Regex = cliclack::input("Include").required(false).interact()?;
            if rx.as_str() == Regex::new("").unwrap().as_str() {
                args.filter = None;
            } else {
                args.filter = Some(rx)
            }
        }

        if options.contains(&"exclude") && args.exclude.is_none() {
            let rx: Regex = cliclack::input("Exclude").required(false).interact()?;
            if rx.as_str() == Regex::new("").unwrap().as_str() {
                args.exclude = None;
            } else {
                args.exclude = Some(rx)
            }
        }
    }

    Ok(ARGS.get_or_init(|| args))
}
