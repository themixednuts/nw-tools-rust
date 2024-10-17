use clap::{Parser, ValueEnum};

use crate::traits::IArgs;

#[derive(Debug, Parser)]
pub struct DistributionConfig {
    #[arg(long, default_value = "bytes")]
    pub distribution: DistributionFormat,
}

impl<'a> IArgs<'a> for DistributionConfig {
    type Value = ();
    fn configure(&mut self, _: Self::Value) -> std::io::Result<()> {
        todo!()
    }
}

#[derive(ValueEnum, Debug, Clone, Default, PartialEq, Eq)]
pub enum DistributionFormat {
    #[default]
    BYTES,
    // XML,
    MINI,
    PRETTY,
    // CSV,
    // YAML,
}
