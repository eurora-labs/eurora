use std::collections::HashMap;

use crate::error::Result;
use crate::messages::BaseMessage;
use crate::utils::interactive_env::is_interactive_env;

pub trait BaseMessagePromptTemplate: Send + Sync {
    fn input_variables(&self) -> Vec<String>;

    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<BaseMessage>>;

    fn aformat_messages(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<BaseMessage>>> + Send + '_>>
    {
        let result = self.format_messages(kwargs);
        Box::pin(async move { result })
    }

    fn pretty_repr(&self, html: bool) -> String;

    fn pretty_print(&self) {
        println!("{}", self.pretty_repr(is_interactive_env()));
    }
}

pub fn get_msg_title_repr(title: &str, bold: bool) -> String {
    let padded = format!(" {} ", title);
    let sep_len = (80_usize).saturating_sub(padded.len()) / 2;
    let sep: String = "=".repeat(sep_len);
    let second_sep = if padded.len() % 2 == 0 {
        sep.clone()
    } else {
        format!("{}=", sep)
    };

    if bold {
        format!("{}\x1b[1m{}\x1b[0m{}", sep, padded, second_sep)
    } else {
        format!("{}{}{}", sep, padded, second_sep)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_msg_title_repr() {
        let title = get_msg_title_repr("Test", false);
        assert!(title.contains("Test"));
        assert!(title.contains("="));
    }

    #[test]
    fn test_get_msg_title_repr_bold() {
        let title = get_msg_title_repr("Test", true);
        assert!(title.contains("Test"));
        assert!(title.contains("\x1b[1m"));
        assert!(title.contains("\x1b[0m"));
    }
}
