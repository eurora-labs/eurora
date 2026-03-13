use std::collections::HashMap;
use std::path::Path;

use async_trait::async_trait;

use bon::bon;

use crate::error::{Error, Result};
use crate::messages::AnyMessage;
use crate::runnables::base::Runnable;
use crate::runnables::config::RunnableConfig;

use super::base::{BasePromptTemplate, merge_prompt_config};
use super::few_shot::ExampleSelectorClone;
use super::prompt::PromptTemplate;
use super::string::{PromptTemplateFormat, StringPromptTemplate};

#[derive(Debug, Clone)]
pub struct FewShotPromptWithTemplates {
    examples: Option<Vec<HashMap<String, String>>>,

    example_selector: Option<Box<dyn ExampleSelectorClone + Send + Sync>>,

    example_prompt: PromptTemplate,

    suffix: PromptTemplate,

    example_separator: String,

    prefix: Option<PromptTemplate>,

    template_format: PromptTemplateFormat,

    input_variables: Vec<String>,

    partial_variables: HashMap<String, String>,

    validate_template: bool,
}

#[bon]
impl FewShotPromptWithTemplates {
    #[builder]
    pub fn new(
        examples: Vec<HashMap<String, String>>,
        example_prompt: PromptTemplate,
        suffix: PromptTemplate,
        prefix: Option<PromptTemplate>,
        #[builder(default = "\n\n".to_string())] example_separator: String,
        #[builder(default)] template_format: PromptTemplateFormat,
        #[builder(default)] validate_template: bool,
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
            example_separator,
            prefix,
            template_format,
            input_variables,
            partial_variables: HashMap::new(),
            validate_template,
        };
        result.validate_template_variables()?;
        Ok(result)
    }

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

    fn get_examples(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, String>>> {
        super::few_shot::resolve_examples(
            self.examples.as_deref(),
            self.example_selector.as_deref(),
            kwargs,
        )
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
}

impl BasePromptTemplate for FewShotPromptWithTemplates {
    fn input_variables(&self) -> &[String] {
        &self.input_variables
    }

    fn partial_variables(&self) -> HashMap<String, String> {
        self.partial_variables.clone()
    }

    fn format(&self, kwargs: &HashMap<String, String>) -> Result<String> {
        let kwargs = self.merge_partial_and_user_variables(kwargs);

        let examples = self.get_examples(&kwargs)?;

        let example_strings: Result<Vec<_>> = examples
            .iter()
            .map(|example| StringPromptTemplate::format(&self.example_prompt, example))
            .collect();
        let example_strings = example_strings?;

        let prefix = if let Some(ref prefix_template) = self.prefix {
            StringPromptTemplate::format(prefix_template, &kwargs)?
        } else {
            String::new()
        };

        let suffix = StringPromptTemplate::format(&self.suffix, &kwargs)?;

        let mut pieces = vec![prefix];
        pieces.extend(example_strings);
        pieces.push(suffix);

        Ok(pieces
            .into_iter()
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join(&self.example_separator))
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
        Err(Error::InvalidConfig(
            "Saving few-shot prompts with templates is not currently supported".to_string(),
        ))
    }
}

#[async_trait]
impl Runnable for FewShotPromptWithTemplates {
    type Input = HashMap<String, String>;
    type Output = Vec<AnyMessage>;

    fn name(&self) -> Option<String> {
        Some("FewShotPromptWithTemplates".to_string())
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let config = merge_prompt_config(config, self.metadata(), self.tags());
        self.call_with_config(
            &|input, _config| {
                BasePromptTemplate::validate_input(self, &input)?;
                self.format_messages(&input)
            },
            input,
            config,
        )
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output> {
        self.invoke(input, config)
    }
}

impl StringPromptTemplate for FewShotPromptWithTemplates {
    fn input_variables(&self) -> &[String] {
        &self.input_variables
    }

    fn partial_variables(&self) -> HashMap<String, String> {
        self.partial_variables.clone()
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

        let few_shot = FewShotPromptWithTemplates::builder()
            .examples(examples)
            .example_prompt(example_prompt)
            .suffix(suffix)
            .prefix(prefix)
            .build()
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

        let few_shot = FewShotPromptWithTemplates::builder()
            .examples(examples)
            .example_prompt(example_prompt)
            .suffix(suffix)
            .build()
            .unwrap();

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

        let few_shot = FewShotPromptWithTemplates::builder()
            .examples(examples)
            .example_prompt(example_prompt)
            .suffix(suffix)
            .prefix(prefix)
            .build()
            .unwrap();

        let vars = BasePromptTemplate::input_variables(&few_shot);
        assert!(vars.contains(&"suffix_var".to_string()));
        assert!(vars.contains(&"prefix_var".to_string()));
    }

    #[test]
    fn test_shared_variable_between_prefix_and_suffix() {
        let examples = vec![];
        let example_prompt = PromptTemplate::from_template("{ex}").unwrap();
        let prefix = PromptTemplate::from_template("Context: {context}").unwrap();
        let suffix = PromptTemplate::from_template("Question about {context}: {question}").unwrap();

        let few_shot = FewShotPromptWithTemplates::builder()
            .examples(examples)
            .example_prompt(example_prompt)
            .suffix(suffix)
            .prefix(prefix)
            .build()
            .unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("context".to_string(), "science".to_string());
        kwargs.insert("question".to_string(), "why is sky blue?".to_string());

        let result = BasePromptTemplate::format(&few_shot, &kwargs).unwrap();
        assert!(result.contains("Context: science"));
        assert!(result.contains("Question about science: why is sky blue?"));
    }
}
