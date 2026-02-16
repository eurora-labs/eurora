use crate::prompts::StringPromptTemplate;
use std::collections::HashMap;

use serde_json::Value;

use crate::Result;
use crate::example_selectors::BaseExampleSelector;
use crate::prompts::PromptTemplate;

fn get_length_based(text: &str) -> usize {
    text.split(['\n', ' ']).count()
}

/// Select examples based on length.
pub struct LengthBasedExampleSelector {
    /// A list of the examples that the prompt template expects.
    pub examples: Vec<HashMap<String, String>>,
    /// Prompt template used to format the examples.
    pub example_prompt: PromptTemplate,
    /// Function to measure prompt length. Defaults to word count.
    pub get_text_length: fn(&str) -> usize,
    /// Max length for the prompt, beyond which examples are cut.
    pub max_length: usize,
    /// Length of each example.
    example_text_lengths: Vec<usize>,
}

impl LengthBasedExampleSelector {
    pub fn new(
        examples: Vec<HashMap<String, String>>,
        example_prompt: PromptTemplate,
        max_length: usize,
    ) -> Result<Self> {
        let get_text_length = get_length_based;
        let example_text_lengths: Vec<usize> = examples
            .iter()
            .map(|example| {
                let formatted = example_prompt.format(example)?;
                Ok(get_text_length(&formatted))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            examples,
            example_prompt,
            get_text_length,
            max_length,
            example_text_lengths,
        })
    }

    pub fn with_text_length_fn(mut self, func: fn(&str) -> usize) -> Result<Self> {
        self.get_text_length = func;
        self.example_text_lengths = self
            .examples
            .iter()
            .map(|example| {
                let formatted = self.example_prompt.format(example)?;
                Ok((self.get_text_length)(&formatted))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(self)
    }
}

#[async_trait::async_trait]
impl BaseExampleSelector for LengthBasedExampleSelector {
    fn add_example(&mut self, example: HashMap<String, String>) -> Result<Option<String>> {
        let formatted = self.example_prompt.format(&example)?;
        self.example_text_lengths
            .push((self.get_text_length)(&formatted));
        self.examples.push(example);
        Ok(None)
    }

    fn select_examples(
        &self,
        input_variables: HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, Value>>> {
        let inputs: String = input_variables
            .values()
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        let mut remaining_length =
            self.max_length as isize - (self.get_text_length)(&inputs) as isize;
        let mut i = 0;
        let mut selected = Vec::new();

        while remaining_length > 0 && i < self.examples.len() {
            let example_length = self.example_text_lengths.get(i).copied().unwrap_or(0) as isize;
            let new_length = remaining_length - example_length;
            if new_length < 0 {
                break;
            }
            selected.push(
                self.examples[i]
                    .iter()
                    .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                    .collect(),
            );
            remaining_length = new_length;
            i += 1;
        }

        Ok(selected)
    }
}
