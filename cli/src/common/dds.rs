use clap::{Parser, ValueEnum};

use crate::traits::IArgs;

#[derive(Debug, Parser)]
pub struct DDSConfig {
    #[arg(long, default_value = "bytes")]
    pub dds: DDSFormat,
}

impl<'a> IArgs<'a> for DDSConfig {
    type Value = ();
    fn configure(&mut self, _: Self::Value) -> std::io::Result<()> {
        todo!()
    }
}

#[derive(ValueEnum, Debug, Clone, Default, PartialEq, Eq)]
pub enum DDSFormat {
    #[default]
    BYTES,
    PNG,
    JPEG,
    WEBP,
    FLAT,
}
