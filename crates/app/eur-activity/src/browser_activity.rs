use crate::ActivityStrategy;

pub struct BrowserStrategy {
    name: String,
    icon: String,
    process_name: String,
}

impl BrowserStrategy {
    /// Create a new BrowserStrategy with the given name
    pub fn new(name: String) -> Self {
        Self {
            name,
            icon: String::new(),
            process_name: String::new(),
        }
    }
}

impl ActivityStrategy for BrowserStrategy {
    fn retrieve_assets(&self) -> anyhow::Result<Vec<Box<dyn crate::ActivityAsset>>> {
        // Implementation for retrieving assets
        Ok(vec![])
    }

    fn gather_state(&self) -> String {
        todo!()
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_icon(&self) -> &String {
        &self.icon
    }

    fn get_process_name(&self) -> &String {
        &self.process_name
    }
}
