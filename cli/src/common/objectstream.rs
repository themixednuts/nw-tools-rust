use clap::{Parser, ValueEnum};

use crate::traits::IArgs;

#[derive(Debug, Parser)]
pub struct ObjectStreamConfig {
    #[arg(long, default_value = "bytes")]
    pub objectstream: ObjectStreamFormat,
}

impl<'a> IArgs<'a> for ObjectStreamConfig {
    type Value = ();
    fn configure(&mut self, _: Self::Value) -> std::io::Result<()> {
        todo!()
    }
}

#[derive(ValueEnum, Debug, Clone, Default, PartialEq, Eq)]
pub enum ObjectStreamFormat {
    #[default]
    BYTES,
    XML,
    JSON,
    PRETTY,
    // CSV,
    // YAML,
}
