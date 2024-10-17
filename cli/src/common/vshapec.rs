use clap::{Parser, ValueEnum};

use crate::traits::IArgs;

#[derive(Debug, Parser)]
pub struct VShapeConfig {
    #[arg(long, default_value = "bytes")]
    pub vshapec: VShapeFormat,
}

impl<'a> IArgs<'a> for VShapeConfig {
    type Value = ();
    fn configure(&mut self, _: Self::Value) -> std::io::Result<()> {
        todo!()
    }
}

#[derive(ValueEnum, Debug, Clone, Default, PartialEq, Eq)]
pub enum VShapeFormat {
    #[default]
    BYTES,
    // XML,
    MINI,
    PRETTY,
    // CSV,
    YAML,
}
