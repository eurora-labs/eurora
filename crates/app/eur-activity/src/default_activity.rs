use crate::{ActivityAsset, ActivityStrategy};
use anyhow::Result;
use async_trait::async_trait;

pub struct DefaultStrategy {
    pub name: String,
    pub process_name: String,
    pub icon: String,
}

impl DefaultStrategy {
    pub fn new(name: String, icon: String, process_name: String) -> Result<Self> {
        Ok(Self {
            name,
            process_name,
            icon,
        })
    }
}
#[async_trait]
impl ActivityStrategy for DefaultStrategy {
    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_icon(&self) -> &String {
        &self.icon
    }

    fn get_process_name(&self) -> &String {
        &self.process_name
    }

    async fn retrieve_assets(&mut self) -> Result<Vec<Box<dyn ActivityAsset>>> {
        Ok(vec![])
    }
    fn gather_state(&self) -> String {
        String::new()
    }
}
