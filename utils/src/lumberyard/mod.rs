use std::{
    collections::HashMap,
    io::Cursor,
    os::windows::fs::MetadataExt,
    str::FromStr,
    sync::{Arc, OnceLock},
};

use crc32fast::hash;
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::{
    fs,
    io::{self, AsyncBufReadExt, AsyncReadExt},
    sync::mpsc,
    task::JoinSet,
};
use tracing::{info, info_span, Instrument};
use uuid::Uuid;
use walkdir::{DirEntry, WalkDir};

static AZ_CRC: OnceLock<Regex> = OnceLock::new();
static AZ_TYPE_INFO: OnceLock<Regex> = OnceLock::new();
static ATTRIBUTE: OnceLock<Regex> = OnceLock::new();

pub async fn parse_lumberyard_source() -> io::Result<()> {
    tracing_subscriber::fmt::init();
    AZ_CRC
        .set(Regex::new(r#"AZ_CRC\("([^"]+)"(?:,\s*0x([0-9a-fA-F]+))?\)"#).unwrap())
        .unwrap();
    AZ_TYPE_INFO
        .set(
            Regex::new(r#"AZ_TYPE_INFO\(\s*(\w+)\s*,\s*\"(\{[0-9A-Fa-f\-]+\})\"\s*(?:,.*)?\)"#)
                .unwrap(),
        )
        .unwrap();
    ATTRIBUTE
        .set(Regex::new(r#"(\w+)\s*=\s*"([^"]*)""#).unwrap())
        .unwrap();

    let mut tasks = JoinSet::new();
    let (tx, mut rx) = mpsc::unbounded_channel::<(HashMap<u32, String>, HashMap<Uuid, String>)>();
    let tx = Arc::new(tx);

    tasks.spawn(async move {
        let mut crcs: HashMap<u32, String> = HashMap::new();
        let mut uuids: HashMap<Uuid, String> = HashMap::new();
        while let Some((crc, uuid)) = rx.recv().await {
            if !crc.is_empty() {
                crcs.par_extend(crc);
            }
            if !uuid.is_empty() {
                uuids.par_extend(uuid);
            }
        }

        let ly = LumberyardSource { crcs, uuids };

        let mut file = tokio::fs::File::create("E:/docs/nw-tools-rust/ly.json")
            .await
            .unwrap();
        let data = serde_json::to_string_pretty(&ly).unwrap();
        let mut data = data.as_bytes();
        tokio::io::copy(&mut data, &mut file).await.unwrap();
    });

    let (task_tx, mut task_rx) = mpsc::unbounded_channel();
    let task_tx = Arc::new(task_tx);

    info!("Starting to parse lumberyard");

    tokio::task::spawn_blocking(move || {
        WalkDir::new("e:/lumberyard/dev")
            .sort_by_file_name()
            .into_iter()
            .filter_map(|dir| dir.ok())
            .filter(|entry| entry.file_type().is_file())
            .par_bridge()
            .for_each(move |file| {
                let file_name = file.file_name().to_str().unwrap();
                let span = info_span!("File", name = %file_name);
                let tx = tx.clone();
                let fut = async move {
                    let size = file.metadata().unwrap().file_size();
                    let mut file = fs::File::open(file.path()).await.unwrap();
                    let mut buf = Vec::with_capacity(size as usize);
                    file.read_to_end(&mut buf).await.unwrap();
                    let (crc, uuid) = parse(&mut buf).await;
                    tx.send((crc, uuid)).unwrap();
                }
                .instrument(span);
                task_tx.send(fut).unwrap();
            });
    })
    .await
    .unwrap();

    while let Some(fut) = task_rx.recv().await {
        tasks.spawn(fut);
    }

    while let Some(_task) = tasks.join_next().await {
        // let len = tasks.len();
        // let is_err = task.is_err();
        // info!("Tasks: {} | Err: {}", len, is_err);
    }

    info!("Done");
    Ok(())
}

async fn parse(buf: &mut Vec<u8>) -> (HashMap<u32, String>, HashMap<Uuid, String>) {
    if buf.starts_with(b"<ObjectStream") {
        info!("ObjectStream");
        return xml(buf).await;
    };

    let mut crcs = HashMap::new();
    let mut uuids = HashMap::new();

    let mut buf = Cursor::new(buf).lines();
    while let Ok(Some(line)) = buf.next_line().await {
        if line.trim().is_empty() {
            continue;
        }

        if let Some(captures) = AZ_CRC.get().unwrap().captures(&line) {
            let name = captures.get(1).unwrap().as_str().to_string();
            let crc = if let Some(crc) = captures.get(2) {
                let crc = u32::from_str_radix(crc.as_str(), 16).ok().unwrap();
                info!(crc, name);
                Some((crc, name))
            } else {
                let crc = hash(name.to_lowercase().as_bytes());
                info!(crc, name);
                Some((crc, name))
            };
            if let Some((crc, name)) = crc {
                crcs.insert(crc, name);
            }
        }

        if let Some(captures) = AZ_TYPE_INFO.get().unwrap().captures(&line) {
            let class = captures.get(1).unwrap().as_str().to_string();
            let uuid = captures.get(2).unwrap().as_str().to_string();
            info!(uuid, class);
            uuids.insert(Uuid::from_str(&uuid).unwrap(), class);
        }
    }

    (crcs, uuids)
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LumberyardSource {
    pub uuids: HashMap<Uuid, String>,
    pub crcs: HashMap<u32, String>,
}

async fn xml(buf: &mut Vec<u8>) -> (HashMap<u32, String>, HashMap<Uuid, String>) {
    info_span!("ObjectStream");
    let mut crcs = HashMap::new();
    let mut uuids = HashMap::new();
    let mut buf = Cursor::new(buf).lines();

    while let Ok(Some(line)) = buf.next_line().await {
        let fields = line.trim();
        // .trim_start_matches("<Class ")
        // .trim_end_matches('>')
        // .trim_end_matches('/');

        let mut name = String::new();
        for caps in ATTRIBUTE.get().unwrap().captures_iter(fields) {
            let key = caps.get(1).map_or("", |m| m.as_str());
            let value = caps.get(2).map_or("", |m| m.as_str());

            if key == "name" {
                name.clone_from(&value.to_string());
                let crc = hash(value.to_lowercase().as_bytes());
                info!("crc: {} name: {}", crc, name);
                crcs.insert(crc, value.to_owned());
            } else if key == "field" {
                let crc = hash(value.to_lowercase().as_bytes());
                info!("crc: {} value: {}", crc, value);
                crcs.insert(crc, value.to_owned());
            }
            if key == "type" && !name.is_empty() {
                info!("value: {} name: {}", value, name);
                uuids.insert(Uuid::parse_str(value).unwrap(), name.to_owned());
            }
        }
    }
    info!("not stuck?");
    (crcs, uuids)
}
