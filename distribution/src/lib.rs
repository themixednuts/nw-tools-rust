#[derive(Debug, Default)]
pub struct Distribution {}

impl TryFrom<&[u8]> for Distribution {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self {})
    }
}
