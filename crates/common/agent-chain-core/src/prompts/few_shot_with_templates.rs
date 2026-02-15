//! Prompt template that contains few shot examples with templates for prefix and suffix.
//!
//! This module provides few-shot prompt templates that use templates for prefix and suffix,
//! mirroring `langchain_core.prompts.few_shot_with_templates` in Python.

use std::collections::HashMap;
use std::path::Path;

use crate::error::{Error, Result};

use super::base::{BasePromptTemplate, FormatOutputType};
use super::few_shot::ExampleSelectorClone;
use super::prompt::PromptTemplate;
use super::string::{PromptTemplateFormat, StringPromptTemplate, format_template};

/// Prompt template that contains few shot examples with templated prefix and suffix.
///
/// Unlike `FewShotPromptTemplate`, this uses `StringPromptTemplate` instances
/// for the prefix and suffix, allowing them to also contain template variables.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::prompts::{FewShotPromptWithTemplates, PromptTemplate};
/// use std::collections::HashMap;
///
/// let examples = vec![
///     HashMap::from([
///         ("input".to_string(), "2+2".to_string()),
///         ("output".to_string(), "4".to_string()),
///     ]),
/// ];
///
/// let example_prompt = PromptTemplate::from_template("Q: {input}\nA: {output}").unwrap();
/// let prefix = PromptTemplate::from_template("You are {role}.").unwrap();
/// let suffix = PromptTemplate::from_template("Q: {question}\nA:").unwrap();
///
/// let few_shot = FewShotPromptWithTemplates::new(
///     examples,
///     example_prompt,
///     suffix,
///     Some(prefix),
/// ).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct FewShotPromptWithTemplates {
    /// Examples to format into the prompt.
    examples: Option<Vec<HashMap<String, String>>>,

    /// ExampleSelector to choose the examples to format into the prompt.
    example_selector: Option<Box<dyn ExampleSelectorClone + Send + Sync>>,

    /// PromptTemplate used to format an individual example.
    example_prompt: PromptTemplate,

    /// A StringPromptTemplate to put after the examples.
    suffix: PromptTemplate,

    /// String separator used to join the prefix, the examples, and suffix.
    example_separator: String,

    /// A StringPromptTemplate to put before the examples.
    prefix: Option<PromptTemplate>,

    /// The format of the prompt template.
    template_format: PromptTemplateFormat,

    /// Input variables for this prompt.
    input_variables: Vec<String>,

    /// Partial variables for this prompt.
    partial_variables: HashMap<String, String>,

    /// Whether to validate the template.
    validate_template: bool,
}

impl FewShotPromptWithTemplates {
    /// Create a new FewShotPromptWithTemplates with examples.
    pub fn new(
        examples: Vec<HashMap<String, String>>,
        example_prompt: PromptTemplate,
        suffix: PromptTemplate,
        prefix: Option<PromptTemplate>,
    ) -> Result<Self> {
        let mut input_variables = std::collections::HashSet::new();

        for var in &suffix.input_variables {
            input_variables.insert(var.clone());
        }

        if let Some(ref p) = prefix {
            for var in &p.input_variables {
                input_variables.insert(var.clone());
            }
        }

        let mut input_variables: Vec<_> = input_variables.into_iter().collect();
        input_variables.sort();

        let mut result = Self {
            examples: Some(examples),
            example_selector: None,
            example_prompt,
            suffix,
            example_separator: "\n\n".to_string(),
            prefix,
            template_format: PromptTemplateFormat::FString,
            input_variables,
            partial_variables: HashMap::new(),
            validate_template: false,
        };
        result.validate_template_variables()?;
        Ok(result)
    }

    /// Create a new FewShotPromptWithTemplates with an example selector.
    pub fn with_selector(
        selector: impl ExampleSelectorClone + 'static,
        example_prompt: PromptTemplate,
        suffix: PromptTemplate,
        prefix: Option<PromptTemplate>,
    ) -> Result<Self> {
        let mut input_variables = std::collections::HashSet::new();

        for var in &suffix.input_variables {
            input_variables.insert(var.clone());
        }

        if let Some(ref p) = prefix {
            for var in &p.input_variables {
                input_variables.insert(var.clone());
            }
        }

        let mut input_variables: Vec<_> = input_variables.into_iter().collect();
        input_variables.sort();

        let mut result = Self {
            examples: None,
            example_selector: Some(Box::new(selector)),
            example_prompt,
            suffix,
            example_separator: "\n\n".to_string(),
            prefix,
            template_format: PromptTemplateFormat::FString,
            input_variables,
            partial_variables: HashMap::new(),
            validate_template: false,
        };
        result.validate_template_variables()?;
        Ok(result)
    }

    /// Set the example separator.
    pub fn with_separator(mut self, separator: impl Into<String>) -> Self {
        self.example_separator = separator.into();
        self
    }

    /// Set the template format.
    pub fn with_format(mut self, format: PromptTemplateFormat) -> Self {
        self.template_format = format;
        self
    }

    /// Get examples based on kwargs.
    fn get_examples(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, String>>> {
        if let Some(ref examples) = self.examples {
            Ok(examples.clone())
        } else if let Some(ref selector) = self.example_selector {
            Ok(selector.select_examples(kwargs))
        } else {
            Err(Error::InvalidConfig(
                "One of 'examples' and 'example_selector' should be provided".to_string(),
            ))
        }
    }

    /// Async get examples based on kwargs.
    #[allow(dead_code)]
    async fn aget_examples(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, String>>> {
        if let Some(ref examples) = self.examples {
            Ok(examples.clone())
        } else if let Some(ref selector) = self.example_selector {
            Ok(selector.aselect_examples(kwargs).await)
        } else {
            Err(Error::InvalidConfig(
                "One of 'examples' and 'example_selector' should be provided".to_string(),
            ))
        }
    }

    fn validate_template_variables(&mut self) -> Result<()> {
        if self.validate_template {
            let input_set: std::collections::HashSet<_> =
                self.input_variables.iter().cloned().collect();
            let mut expected: std::collections::HashSet<_> =
                self.suffix.input_variables.iter().cloned().collect();
            expected.extend(self.partial_variables.keys().cloned());
            if let Some(ref p) = self.prefix {
                expected.extend(p.input_variables.iter().cloned());
            }
            let missing: Vec<_> = expected.difference(&input_set).cloned().collect();
            if !missing.is_empty() {
                return Err(Error::InvalidConfig(format!(
                    "Got input_variables={:?}, but based on prefix/suffix expected {:?}",
                    self.input_variables, expected
                )));
            }
        } else {
            let mut vars: std::collections::HashSet<_> =
                self.suffix.input_variables.iter().cloned().collect();
            if let Some(ref p) = self.prefix {
                vars.extend(p.input_variables.iter().cloned());
            }
            for k in self.partial_variables.keys() {
                vars.remove(k);
            }
            let mut sorted: Vec<_> = vars.into_iter().collect();
            sorted.sort();
            self.input_variables = sorted;
        }
        Ok(())
    }

    fn merge_partial_and_user_variables(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut merged = self.partial_variables.clone();
        merged.extend(kwargs.clone());
        merged
    }
}

impl BasePromptTemplate for FewShotPromptWithTemplates {
    fn input_variables(&self) -> &[String] {
        &self.input_variables
    }

    fn partial_variables(&self) -> &HashMap<String, String> {
        &self.partial_variables
    }

    fn format(&self, kwargs: &HashMap<String, String>) -> Result<FormatOutputType> {
        let mut kwargs = self.merge_partial_and_user_variables(kwargs);

        let examples = self.get_examples(&kwargs)?;

        let example_strings: Result<Vec<_>> = examples
            .iter()
            .map(|example| StringPromptTemplate::format(&self.example_prompt, example))
            .collect();
        let example_strings = example_strings?;

        let prefix = if let Some(ref prefix_template) = self.prefix {
            let prefix_vars: HashMap<_, _> = kwargs
                .iter()
                .filter(|(k, _)| prefix_template.input_variables.contains(k))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            for k in prefix_vars.keys() {
                kwargs.remove(k);
            }

            StringPromptTemplate::format(prefix_template, &prefix_vars)?
        } else {
            String::new()
        };

        let suffix_vars: HashMap<_, _> = kwargs
            .iter()
            .filter(|(k, _)| self.suffix.input_variables.contains(k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for k in suffix_vars.keys() {
            kwargs.remove(k);
        }

        let suffix = StringPromptTemplate::format(&self.suffix, &suffix_vars)?;

        let mut pieces = vec![prefix];
        pieces.extend(example_strings);
        pieces.push(suffix);

        let template = pieces
            .into_iter()
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join(&self.example_separator);

        format_template(&template, self.template_format, &kwargs)
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
            examples: self.examples.clone(),
            example_selector: self.example_selector.clone(),
            example_prompt: self.example_prompt.clone(),
            suffix: self.suffix.clone(),
            example_separator: self.example_separator.clone(),
            prefix: self.prefix.clone(),
            template_format: self.template_format,
            input_variables: new_vars,
            partial_variables: new_partials,
            validate_template: self.validate_template,
        }))
    }

    fn prompt_type(&self) -> &str {
        "few_shot_with_templates"
    }

    fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "_type": self.prompt_type(),
            "examples": self.examples,
            "example_separator": self.example_separator,
            "template_format": self.template_format,
        })
    }

    fn save(&self, _file_path: &Path) -> Result<()> {
        if self.example_selector.is_some() {
            return Err(Error::InvalidConfig(
                "Saving an example selector is not currently supported".to_string(),
            ));
        }
        // Note: Cannot call default save implementation due to recursion.
        // The save functionality for few-shot prompts with templates is not fully supported.
        Err(Error::InvalidConfig(
            "Saving few-shot prompts with templates is not currently supported".to_string(),
        ))
    }
}

impl StringPromptTemplate for FewShotPromptWithTemplates {
    fn input_variables(&self) -> &[String] {
        &self.input_variables
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

    fn pretty_repr(&self, _html: bool) -> String {
        format!(
            "FewShotPromptWithTemplates(prefix={:?}, suffix={:?}, examples={:?})",
            self.prefix.as_ref().map(|p| &p.template),
            self.suffix.template,
            self.examples
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_few_shot_with_templates() {
        let examples = vec![HashMap::from([
            ("input".to_string(), "2+2".to_string()),
            ("output".to_string(), "4".to_string()),
        ])];

        let example_prompt = PromptTemplate::from_template("Q: {input}\nA: {output}").unwrap();
        let suffix = PromptTemplate::from_template("Q: {question}\nA:").unwrap();
        let prefix = PromptTemplate::from_template("You are a {role}.").unwrap();

        let few_shot =
            FewShotPromptWithTemplates::new(examples, example_prompt, suffix, Some(prefix))
                .unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("role".to_string(), "math tutor".to_string());
        kwargs.insert("question".to_string(), "2+4".to_string());

        let result = BasePromptTemplate::format(&few_shot, &kwargs).unwrap();

        assert!(result.contains("You are a math tutor."));
        assert!(result.contains("Q: 2+2"));
        assert!(result.contains("A: 4"));
        assert!(result.contains("Q: 2+4"));
    }

    #[test]
    fn test_few_shot_without_prefix() {
        let examples = vec![HashMap::from([
            ("x".to_string(), "1".to_string()),
            ("y".to_string(), "2".to_string()),
        ])];

        let example_prompt = PromptTemplate::from_template("{x} + {y}").unwrap();
        let suffix = PromptTemplate::from_template("{a} + {b} = ?").unwrap();

        let few_shot =
            FewShotPromptWithTemplates::new(examples, example_prompt, suffix, None).unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("a".to_string(), "3".to_string());
        kwargs.insert("b".to_string(), "4".to_string());

        let result = BasePromptTemplate::format(&few_shot, &kwargs).unwrap();

        assert!(result.contains("1 + 2"));
        assert!(result.contains("3 + 4 = ?"));
    }

    #[test]
    fn test_input_variables_inference() {
        let examples = vec![];
        let example_prompt = PromptTemplate::from_template("{ex}").unwrap();
        let suffix = PromptTemplate::from_template("{suffix_var}").unwrap();
        let prefix = PromptTemplate::from_template("{prefix_var}").unwrap();

        let few_shot =
            FewShotPromptWithTemplates::new(examples, example_prompt, suffix, Some(prefix))
                .unwrap();

        let vars = BasePromptTemplate::input_variables(&few_shot);
        assert!(vars.contains(&"suffix_var".to_string()));
        assert!(vars.contains(&"prefix_var".to_string()));
    }
}
