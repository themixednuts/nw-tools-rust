use std::{
    collections::HashMap,
    io::{BufReader, Read},
};

use dashmap::DashMap;
// use quick_xml::DeError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
#[serde(rename = "resources")]
pub struct Localization {
    #[serde(rename = "@xmlns:xsi")]
    xmlns: Option<String>,
    #[serde(default)]
    string: Vec<KeyValue>,
}

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
#[serde(rename = "string")]
pub struct KeyValue {
    #[serde(rename = "@key")]
    key: Option<String>,
    #[serde(rename = "@comment")]
    comment: Option<String>,
    #[serde(rename = "@rel_version")]
    rel_version: Option<String>,
    #[serde(rename = "@speaker")]
    speaker: Option<String>,
    #[serde(rename = "@name")]
    name: Option<String>,
    #[serde(rename = "@VO_Status")]
    vo_status: Option<String>,
    #[serde(rename = "@dialogue-next")]
    dialogue_next: Option<String>,
    #[serde(rename = "@xsi:nil")]
    xsi_nil: Option<bool>,
    #[serde(rename = "$value", default)]
    value: Option<String>,
}

impl From<Localization> for DashMap<String, Option<String>> {
    fn from(value: Localization) -> Self {
        value
            .string
            .iter()
            .map(|s| {
                (
                    s.key.to_owned().map(|s| s.to_lowercase()),
                    s.value.to_owned(),
                )
            })
            .filter_map(|(k, v)| k.map(|k| (k, v)))
            .collect::<DashMap<_, _>>()
    }
}

impl From<Localization> for HashMap<String, Option<String>> {
    fn from(value: Localization) -> Self {
        value
            .string
            .iter()
            .map(|s| (s.key.to_owned(), s.value.to_owned()))
            .filter_map(|(k, v)| k.map(|k| (k, v)))
            .collect::<HashMap<_, _>>()
    }
}

impl<R: Read> From<R> for Localization {
    fn from(value: R) -> Self {
        quick_xml::de::from_reader(BufReader::new(value)).unwrap()
    }
}

// impl<R: Read> TryFrom<R> for Localization {
//     type Error = DeError;
//     fn try_from(value: R) -> Result<Self, Self::Error> {
//         quick_xml::de::from_reader(BufReader::new(value))
//     }
// }
