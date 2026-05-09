use std::fmt;

pub(crate) trait ResultExt<T> {
    fn ctx(self, msg: &str) -> Result<T, String>;
}

impl<T, E: fmt::Display> ResultExt<T> for Result<T, E> {
    fn ctx(self, msg: &str) -> Result<T, String> {
        self.map_err(|e| format!("{msg}: {e}"))
    }
}
