use crate::config::EuroraConfig;
use crate::error::EuroraError;

#[derive(Debug, Clone)]
pub struct EuroraProvider {
    config: EuroraConfig,
}

impl EuroraProvider {
    pub fn new(config: EuroraConfig) -> Result<Self, EuroraError> {
        Ok(Self { config })
    }
}
