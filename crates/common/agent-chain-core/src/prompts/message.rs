//! Message prompt templates.
//!
//! This module provides the base trait for message prompt templates,
//! mirroring `langchain_core.prompts.message` in Python.

use std::collections::HashMap;

use crate::error::Result;
use crate::messages::BaseMessage;
use crate::utils::interactive_env::is_interactive_env;

/// Base trait for message prompt templates.
///
/// Message prompt templates format into a list of messages rather than a single string.
/// They are used in chat-based models where threads consist of multiple messages.
pub trait BaseMessagePromptTemplate: Send + Sync {
    /// Get the input variables for this template.
    ///
    /// Returns a list of variable names that are required to format this template.
    fn input_variables(&self) -> Vec<String>;

    /// Format messages from kwargs.
    ///
    /// # Arguments
    ///
    /// * `kwargs` - Keyword arguments to use for formatting.
    ///
    /// # Returns
    ///
    /// A list of formatted `BaseMessage` objects.
    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<BaseMessage>>;

    /// Async format messages from kwargs.
    ///
    /// Default implementation calls the sync version.
    ///
    /// # Arguments
    ///
    /// * `kwargs` - Keyword arguments to use for formatting.
    ///
    /// # Returns
    ///
    /// A list of formatted `BaseMessage` objects.
    fn aformat_messages(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<BaseMessage>>> + Send + '_>>
    {
        let result = self.format_messages(kwargs);
        Box::pin(async move { result })
    }

    /// Get a pretty representation of the template.
    ///
    /// # Arguments
    ///
    /// * `html` - Whether to format as HTML.
    ///
    /// # Returns
    ///
    /// A human-readable representation of the template.
    fn pretty_repr(&self, html: bool) -> String;

    /// Print a human-readable representation.
    fn pretty_print(&self) {
        println!("{}", self.pretty_repr(is_interactive_env()));
    }
}

/// Helper function to get a title representation for a message.
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
