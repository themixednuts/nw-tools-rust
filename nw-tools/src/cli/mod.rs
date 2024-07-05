use clap::{self, Parser};
use cliclack;
use dirs::{self, home_dir};
use file_system::FileSystem;
use std::{path::PathBuf, str::FromStr};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub input: Option<PathBuf>,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

pub fn interactive() -> std::io::Result<Args> {
    cliclack::clear_screen()?;
    cliclack::intro("New World Tools")?;

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

    let input_clone = input.to_owned();

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
    let output: PathBuf = cliclack::input("Extract Directory")
        .default_input(
            home_dir
                .join(r"Documents\nw\".to_owned() + nw_type)
                .to_str()
                .unwrap_or_default(),
        )
        .interact()?;

    let output_clone = output.clone();
    tokio::spawn(async move {
        FileSystem::init(&input_clone, &output_clone).await;
    });

    cliclack::outro("Let's Begin!")?;
    Ok(Args {
        input: Some(input),
        output: Some(output),
    })
}
