//! Image prompt template for multimodal models.
//!
//! This module provides image prompt templates for multimodal models,
//! mirroring `langchain_core.prompts.image` in Python.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use async_trait::async_trait;

use crate::error::{Error, Result};
use crate::prompt_values::StringPromptValue;
use crate::runnables::base::Runnable;
use crate::runnables::config::{RunnableConfig, ensure_config};

use super::base::{BasePromptTemplate, FormatOutputType};
use super::string::{PromptTemplateFormat, format_template, get_template_variables};

/// Image URL structure for multimodal prompts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageURL {
    /// The URL of the image.
    pub url: String,

    /// The detail level for the image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl ImageURL {
    /// Create a new ImageURL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            detail: None,
        }
    }

    /// Create a new ImageURL with a detail level.
    pub fn with_detail(url: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            detail: Some(detail.into()),
        }
    }
}

/// Template for image prompts in multimodal models.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::prompts::ImagePromptTemplate;
/// use std::collections::HashMap;
///
/// let template = ImagePromptTemplate::new(
///     HashMap::from([("url".to_string(), "{image_url}".to_string())]),
///     vec!["image_url".to_string()],
/// ).unwrap();
///
/// let mut kwargs = HashMap::new();
/// kwargs.insert("image_url".to_string(), "https://example.com/image.jpg".to_string());
///
/// let result = template.format(&kwargs).unwrap();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagePromptTemplate {
    /// Template for the image URL.
    pub template: HashMap<String, String>,

    /// Input variables for the template.
    pub input_variables: Vec<String>,

    /// The format of the prompt template.
    #[serde(default)]
    pub template_format: PromptTemplateFormat,

    /// Partial variables that are pre-filled.
    #[serde(default)]
    pub partial_variables: HashMap<String, String>,
}

impl ImagePromptTemplate {
    /// Create a new ImagePromptTemplate.
    ///
    /// # Arguments
    ///
    /// * `template` - A map containing the template configuration (url, detail, etc.)
    /// * `input_variables` - The input variables used in the template.
    ///
    /// # Returns
    ///
    /// A new ImagePromptTemplate, or an error if validation fails.
    pub fn new(template: HashMap<String, String>, input_variables: Vec<String>) -> Result<Self> {
        let reserved = ["url", "path", "detail"];
        let overlap: Vec<_> = input_variables
            .iter()
            .filter(|v| reserved.contains(&v.as_str()))
            .collect();

        if !overlap.is_empty() {
            return Err(Error::InvalidConfig(format!(
                "input_variables for the image template cannot contain \
                 any of 'url', 'path', or 'detail'. Found: {:?}",
                overlap
            )));
        }

        Ok(Self {
            template,
            input_variables,
            template_format: PromptTemplateFormat::FString,
            partial_variables: HashMap::new(),
        })
    }

    /// Create a new ImagePromptTemplate with just a URL template.
    pub fn from_url_template(url_template: impl Into<String>) -> Result<Self> {
        let url_template = url_template.into();
        let input_variables = get_template_variables(&url_template, PromptTemplateFormat::FString)?;

        let mut template = HashMap::new();
        template.insert("url".to_string(), url_template);

        Self::new(template, input_variables)
    }

    /// Set the template format.
    pub fn with_format(mut self, format: PromptTemplateFormat) -> Self {
        self.template_format = format;
        self
    }

    /// Format the template into an ImageURL.
    pub fn format_image(&self, kwargs: &HashMap<String, String>) -> Result<ImageURL> {
        let mut merged_kwargs = self.partial_variables.clone();
        merged_kwargs.extend(kwargs.iter().map(|(k, v)| (k.clone(), v.clone())));

        let mut formatted = HashMap::new();

        for (key, value) in &self.template {
            if key == "path" {
                return Err(Error::InvalidConfig(
                    "Loading images from 'path' has been removed for security reasons. \
                     Please specify images by 'url'."
                        .to_string(),
                ));
            }

            let formatted_value = format_template(value, self.template_format, &merged_kwargs)?;
            formatted.insert(key.clone(), formatted_value);
        }

        let url = merged_kwargs
            .get("url")
            .cloned()
            .or_else(|| formatted.get("url").cloned())
            .ok_or_else(|| Error::InvalidConfig("Must provide url.".to_string()))?;

        let detail = merged_kwargs
            .get("detail")
            .cloned()
            .or_else(|| formatted.get("detail").cloned());

        Ok(ImageURL { url, detail })
    }
}

impl BasePromptTemplate for ImagePromptTemplate {
    fn input_variables(&self) -> &[String] {
        &self.input_variables
    }

    fn format(&self, kwargs: &HashMap<String, String>) -> Result<FormatOutputType> {
        let image_url = self.format_image(kwargs)?;
        serde_json::to_string(&image_url).map_err(|e| Error::Other(e.to_string()))
    }

    fn partial(&self, kwargs: HashMap<String, String>) -> Result<Box<dyn BasePromptTemplate>> {
        let new_vars: Vec<_> = self
            .input_variables
            .iter()
            .filter(|v| !kwargs.contains_key(*v))
            .cloned()
            .collect();

        let mut new_partial = self.partial_variables.clone();
        new_partial.extend(kwargs);

        Ok(Box::new(Self {
            template: self.template.clone(),
            input_variables: new_vars,
            template_format: self.template_format,
            partial_variables: new_partial,
        }))
    }

    fn prompt_type(&self) -> &str {
        "image-prompt"
    }

    fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "_type": self.prompt_type(),
            "template": self.template,
            "input_variables": self.input_variables,
            "template_format": self.template_format,
        })
    }
}

#[async_trait]
impl Runnable for ImagePromptTemplate {
    type Input = HashMap<String, String>;
    type Output = StringPromptValue;

    fn name(&self) -> Option<String> {
        Some("ImagePromptTemplate".to_string())
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let _config = ensure_config(config);
        BasePromptTemplate::validate_input(self, &input)?;
        let text = BasePromptTemplate::format(self, &input)?;
        Ok(StringPromptValue::new(text))
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output> {
        self.invoke(input, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_url() {
        let url = ImageURL::new("https://example.com/image.jpg");
        assert_eq!(url.url, "https://example.com/image.jpg");
        assert!(url.detail.is_none());

        let url_with_detail = ImageURL::with_detail("https://example.com/image.jpg", "high");
        assert_eq!(url_with_detail.detail, Some("high".to_string()));
    }

    #[test]
    fn test_from_url_template() {
        let template = ImagePromptTemplate::from_url_template("{image_url}").unwrap();
        assert_eq!(template.input_variables, vec!["image_url"]);
    }

    #[test]
    fn test_format_image() {
        let template = ImagePromptTemplate::from_url_template("{image_url}").unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert(
            "image_url".to_string(),
            "https://example.com/image.jpg".to_string(),
        );

        let result = template.format_image(&kwargs).unwrap();
        assert_eq!(result.url, "https://example.com/image.jpg");
    }

    #[test]
    fn test_invalid_input_variables() {
        let mut template = HashMap::new();
        template.insert("url".to_string(), "{url}".to_string());

        let result = ImagePromptTemplate::new(template, vec!["url".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_path_rejected() {
        let mut template = HashMap::new();
        template.insert("path".to_string(), "/some/path".to_string());

        let prompt = ImagePromptTemplate {
            template,
            input_variables: Vec::new(),
            template_format: PromptTemplateFormat::FString,
            partial_variables: HashMap::new(),
        };

        let result = prompt.format_image(&HashMap::new());
        assert!(result.is_err());
    }
}
