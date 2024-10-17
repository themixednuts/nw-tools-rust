use clap::{Parser, ValueEnum};

use crate::traits::IArgs;

#[derive(Debug, Parser)]
pub struct LuaConfig {
    #[arg(long, default_value = "bytes")]
    pub distribution: LuaFormat,
}

impl<'a> IArgs<'a> for LuaConfig {
    type Value = ();
    fn configure(&mut self, _: Self::Value) -> std::io::Result<()> {
        todo!()
    }
}

#[derive(ValueEnum, Debug, Clone, Default, PartialEq, Eq)]
pub enum LuaFormat {
    #[default]
    BYTES,
    LUA,
}
