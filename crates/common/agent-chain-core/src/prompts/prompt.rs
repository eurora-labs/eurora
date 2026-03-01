use std::collections::HashMap;
use std::path::Path;

use bon::bon;
use serde::{Deserialize, Serialize};

use async_trait::async_trait;

use crate::error::{Error, Result};
use crate::prompt_values::StringPromptValue;
use crate::runnables::base::Runnable;
use crate::runnables::config::{RunnableConfig, ensure_config};
use crate::utils::input::get_colored_text;

use super::base::{BasePromptTemplate, FormatOutputType};
use super::string::{
    PromptTemplateFormat, StringPromptTemplate, check_valid_template, format_template,
    get_template_variables,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub template: String,

    pub input_variables: Vec<String>,

    #[serde(default)]
    pub optional_variables: Vec<String>,

    #[serde(default)]
    pub template_format: PromptTemplateFormat,

    #[serde(default)]
    pub validate_template: bool,

    #[serde(default)]
    pub partial_variables: HashMap<String, String>,

    #[serde(default)]
    pub metadata: Option<HashMap<String, serde_json::Value>>,

    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[bon]
impl PromptTemplate {
    #[builder]
    pub fn new(
        template: impl Into<String>,
        #[builder(default)] template_format: PromptTemplateFormat,
        #[builder(default)] partial_variables: HashMap<String, String>,
        metadata: Option<HashMap<String, serde_json::Value>>,
        tags: Option<Vec<String>>,
        #[builder(default)] validate_template: bool,
    ) -> Result<Self> {
        let template = template.into();
        let all_variables = get_template_variables(&template, template_format)?;

        let input_variables: Vec<_> = all_variables
            .into_iter()
            .filter(|v| !partial_variables.contains_key(v))
            .collect();

        let prompt = Self {
            template,
            input_variables,
            optional_variables: Vec::new(),
            template_format,
            validate_template,
            partial_variables,
            metadata,
            tags,
        };

        prompt.validate()?;
        Ok(prompt)
    }

    pub fn from_template(template: impl Into<String>) -> Result<Self> {
        Self::builder().template(template).build()
    }

    pub fn from_template_with_format(
        template: impl Into<String>,
        template_format: PromptTemplateFormat,
    ) -> Result<Self> {
        Self::builder().template(template).template_format(template_format).build()
    }

    pub fn from_template_with_partials(
        template: impl Into<String>,
        template_format: PromptTemplateFormat,
        partial_variables: HashMap<String, String>,
    ) -> Result<Self> {
        Self::builder()
            .template(template)
            .template_format(template_format)
            .partial_variables(partial_variables)
            .build()
    }

    pub fn from_file(template_file: impl AsRef<Path>) -> Result<Self> {
        let template = std::fs::read_to_string(template_file)?;
        Self::from_template(template)
    }

    pub fn from_file_with_format(
        template_file: impl AsRef<Path>,
        template_format: PromptTemplateFormat,
    ) -> Result<Self> {
        let template = std::fs::read_to_string(template_file)?;
        Self::from_template_with_format(template, template_format)
    }

    pub fn from_examples(
        examples: &[String],
        suffix: &str,
        input_variables: Vec<String>,
        example_separator: Option<&str>,
        prefix: Option<&str>,
    ) -> Result<Self> {
        let example_separator = example_separator.unwrap_or("\n\n");
        let prefix = prefix.unwrap_or("");

        let mut pieces = vec![prefix.to_string()];
        pieces.extend(examples.iter().cloned());
        pieces.push(suffix.to_string());

        let template = pieces.join(example_separator);

        Ok(Self {
            template,
            input_variables,
            optional_variables: Vec::new(),
            template_format: PromptTemplateFormat::FString,
            validate_template: false,
            partial_variables: HashMap::new(),
            metadata: None,
            tags: None,
        })
    }

    fn validate(&self) -> Result<()> {
        if self.validate_template {
            if self.template_format == PromptTemplateFormat::Mustache {
                return Err(Error::InvalidConfig(
                    "Mustache templates cannot be validated.".to_string(),
                ));
            }

            let all_inputs: Vec<_> = self
                .input_variables
                .iter()
                .chain(self.partial_variables.keys())
                .cloned()
                .collect();

            check_valid_template(&self.template, self.template_format, &all_inputs)?;
        }

        Ok(())
    }


}

impl BasePromptTemplate for PromptTemplate {
    fn input_variables(&self) -> &[String] {
        &self.input_variables
    }

    fn optional_variables(&self) -> &[String] {
        &self.optional_variables
    }

    fn partial_variables(&self) -> &HashMap<String, String> {
        &self.partial_variables
    }

    fn metadata(&self) -> Option<&HashMap<String, serde_json::Value>> {
        self.metadata.as_ref()
    }

    fn tags(&self) -> Option<&[String]> {
        self.tags.as_deref()
    }

    fn format(&self, kwargs: &HashMap<String, String>) -> Result<FormatOutputType> {
        let merged = self.merge_partial_and_user_variables(kwargs);
        format_template(&self.template, self.template_format, &merged)
    }

    fn partial(&self, kwargs: HashMap<String, String>) -> Result<Box<dyn BasePromptTemplate>> {
        let new_vars: Vec<_> = self
            .input_variables
            .iter()
            .filter(|v| !kwargs.contains_key(*v))
            .cloned()
            .collect();

        let mut new_partials = self.partial_variables.clone();
        new_partials.extend(kwargs);

        Ok(Box::new(Self {
            template: self.template.clone(),
            input_variables: new_vars,
            optional_variables: self.optional_variables.clone(),
            template_format: self.template_format,
            validate_template: self.validate_template,
            partial_variables: new_partials,
            metadata: self.metadata.clone(),
            tags: self.tags.clone(),
        }))
    }

    fn prompt_type(&self) -> &str {
        "prompt"
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

impl StringPromptTemplate for PromptTemplate {
    fn input_variables(&self) -> &[String] {
        &self.input_variables
    }

    fn optional_variables(&self) -> &[String] {
        &self.optional_variables
    }

    fn partial_variables(&self) -> &HashMap<String, String> {
        &self.partial_variables
    }

    fn template_format(&self) -> PromptTemplateFormat {
        self.template_format
    }

    fn format(&self, kwargs: &HashMap<String, String>) -> Result<String> {
        BasePromptTemplate::format(self, kwargs)
    }

    fn pretty_repr(&self, html: bool) -> String {
        let dummy_vars: HashMap<_, _> = self
            .input_variables
            .iter()
            .map(|v| {
                let placeholder = format!("{{{}}}", v);
                if html {
                    (v.clone(), get_colored_text(&placeholder, "yellow"))
                } else {
                    (v.clone(), placeholder)
                }
            })
            .collect();

        match BasePromptTemplate::format(self, &dummy_vars) {
            Ok(s) => s,
            Err(_) => self.template.clone(),
        }
    }
}

impl std::ops::Add for PromptTemplate {
    type Output = Result<PromptTemplate>;

    fn add(self, other: Self) -> Self::Output {
        if self.template_format != other.template_format {
            return Err(Error::InvalidConfig(
                "Cannot add templates of different formats".to_string(),
            ));
        }

        let mut input_variables: std::collections::HashSet<_> =
            self.input_variables.into_iter().collect();
        input_variables.extend(other.input_variables);

        let template = format!("{}{}", self.template, other.template);
        let validate_template = self.validate_template && other.validate_template;

        let mut partial_variables = self.partial_variables;
        for (k, v) in other.partial_variables {
            if partial_variables.contains_key(&k) {
                return Err(Error::InvalidConfig(
                    "Cannot have same variable partialed twice.".to_string(),
                ));
            }
            partial_variables.insert(k, v);
        }

        Ok(PromptTemplate {
            template,
            input_variables: input_variables.into_iter().collect(),
            optional_variables: Vec::new(),
            template_format: self.template_format,
            validate_template,
            partial_variables,
            metadata: None,
            tags: None,
        })
    }
}

impl std::ops::Add<&str> for PromptTemplate {
    type Output = Result<PromptTemplate>;

    fn add(self, other: &str) -> Self::Output {
        let other_prompt = PromptTemplate::from_template_with_format(other, self.template_format)?;
        self + other_prompt
    }
}

#[async_trait]
impl Runnable for PromptTemplate {
    type Input = HashMap<String, String>;
    type Output = StringPromptValue;

    fn name(&self) -> Option<String> {
        Some("PromptTemplate".to_string())
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

use crate::load::Serializable;
use serde_json::Value;

impl Serializable for PromptTemplate {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "langchain".to_string(),
            "prompts".to_string(),
            "prompt".to_string(),
        ]
    }

    fn lc_attributes(&self) -> HashMap<String, Value> {
        let mut attrs = HashMap::new();
        attrs.insert(
            "template_format".to_string(),
            Value::String(self.template_format.as_str().to_string()),
        );
        attrs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_template() {
        let prompt = PromptTemplate::from_template("Hello, {name}!").unwrap();
        assert_eq!(prompt.input_variables, vec!["name"]);
        assert_eq!(prompt.template, "Hello, {name}!");
    }

    #[test]
    fn test_format() {
        let prompt = PromptTemplate::from_template("Hello, {name}!").unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), "World".to_string());

        let result = BasePromptTemplate::format(&prompt, &kwargs).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_format_multiple_variables() {
        let prompt = PromptTemplate::from_template("{greeting}, {name}!").unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("greeting".to_string(), "Hello".to_string());
        kwargs.insert("name".to_string(), "World".to_string());

        let result = BasePromptTemplate::format(&prompt, &kwargs).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_partial() {
        let prompt = PromptTemplate::from_template("{greeting}, {name}!").unwrap();

        let mut partial_vars = HashMap::new();
        partial_vars.insert("greeting".to_string(), "Hi".to_string());

        let partial_prompt = BasePromptTemplate::partial(&prompt, partial_vars).unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), "Alice".to_string());

        let result = partial_prompt.format(&kwargs).unwrap();
        assert_eq!(result, "Hi, Alice!");
    }

    #[test]
    fn test_from_examples() {
        let examples = vec!["Example 1".to_string(), "Example 2".to_string()];

        let prompt = PromptTemplate::from_examples(
            &examples,
            "What is {input}?",
            vec!["input".to_string()],
            Some("\n"),
            Some("Here are some examples:"),
        )
        .unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("input".to_string(), "2+2".to_string());

        let result = BasePromptTemplate::format(&prompt, &kwargs).unwrap();
        assert!(result.contains("Here are some examples:"));
        assert!(result.contains("Example 1"));
        assert!(result.contains("Example 2"));
        assert!(result.contains("What is 2+2?"));
    }

    #[test]
    fn test_add_prompts() {
        let prompt1 = PromptTemplate::from_template("Hello, {name}! ").unwrap();
        let prompt2 = PromptTemplate::from_template("How are you, {name}?").unwrap();

        let combined = (prompt1 + prompt2).unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), "Alice".to_string());

        let result = BasePromptTemplate::format(&combined, &kwargs).unwrap();
        assert_eq!(result, "Hello, Alice! How are you, Alice?");
    }

    #[test]
    fn test_mustache_format() {
        let prompt = PromptTemplate::from_template_with_format(
            "Hello, {{name}}!",
            PromptTemplateFormat::Mustache,
        )
        .unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), "World".to_string());

        let result = BasePromptTemplate::format(&prompt, &kwargs).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_pretty_repr() {
        let prompt = PromptTemplate::from_template("Hello, {name}!").unwrap();
        let repr = prompt.pretty_repr(false);
        assert_eq!(repr, "Hello, {name}!");
    }
}
