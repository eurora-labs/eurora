use std::sync::Arc;

use async_trait::async_trait;
use tiktoken_rs::CoreBPE;

use crate::documents::{BaseDocumentTransformer, Document};
use crate::text_splitters::{
    LengthFunction, TextSplitter, TextSplitterConfig, Tokenizer, split_text_on_tokens,
};
use std::collections::HashMap;

/// Resolves a tiktoken encoding name string to the corresponding `tiktoken_rs::tokenizer::Tokenizer` enum variant.
fn encoding_name_to_tokenizer(name: &str) -> Option<tiktoken_rs::tokenizer::Tokenizer> {
    match name {
        "gpt2" => Some(tiktoken_rs::tokenizer::Tokenizer::Gpt2),
        "r50k_base" => Some(tiktoken_rs::tokenizer::Tokenizer::R50kBase),
        "p50k_base" => Some(tiktoken_rs::tokenizer::Tokenizer::P50kBase),
        "p50k_edit" => Some(tiktoken_rs::tokenizer::Tokenizer::P50kEdit),
        "cl100k_base" => Some(tiktoken_rs::tokenizer::Tokenizer::Cl100kBase),
        "o200k_base" => Some(tiktoken_rs::tokenizer::Tokenizer::O200kBase),
        _ => None,
    }
}

/// Resolves a tiktoken `CoreBPE` from either a model name or an encoding name.
///
/// If `model_name` is provided, uses `tiktoken_rs::get_bpe_from_model`.
/// Otherwise, maps `encoding_name` to the corresponding tokenizer variant.
/// Defaults to `"gpt2"` if neither is provided.
pub fn resolve_tiktoken_bpe(
    encoding_name: Option<&str>,
    model_name: Option<&str>,
) -> Result<CoreBPE, Box<dyn std::error::Error + Send + Sync>> {
    if let Some(model) = model_name {
        tiktoken_rs::get_bpe_from_model(model).map_err(
            |e| -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(crate::Error::ValidationError(e.to_string()))
            },
        )
    } else {
        let name = encoding_name.unwrap_or("gpt2");
        let tokenizer = encoding_name_to_tokenizer(name).ok_or_else(|| {
            crate::Error::ValidationError(format!("Unknown tiktoken encoding: {}", name))
        })?;
        tiktoken_rs::get_bpe_from_tokenizer(tokenizer).map_err(
            |e| -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(crate::Error::ValidationError(e.to_string()))
            },
        )
    }
}

/// Creates a tiktoken-based length function that counts tokens instead of characters.
///
/// This is the Rust equivalent of Python's `TextSplitter.from_tiktoken_encoder` pattern.
/// The returned function can be used as `length_function` in `TextSplitterConfig` for any splitter type.
pub fn tiktoken_length_function(
    encoding_name: Option<&str>,
    model_name: Option<&str>,
) -> Result<LengthFunction, Box<dyn std::error::Error + Send + Sync>> {
    let bpe = Arc::new(resolve_tiktoken_bpe(encoding_name, model_name)?);
    Ok(Arc::new(move |text: &str| -> usize {
        bpe.encode_ordinary(text).len()
    }))
}

/// Splitting text to tokens using tiktoken model tokenizer.
///
/// Equivalent to Python's `TokenTextSplitter` class.
pub struct TokenTextSplitter {
    config: TextSplitterConfig,
    tokenizer: Arc<CoreBPE>,
}

impl TokenTextSplitter {
    /// Create a new `TokenTextSplitter` with the given configuration and tiktoken encoding.
    ///
    /// - `encoding_name`: Name of the tiktoken encoding (e.g., "gpt2", "cl100k_base"). Defaults to "gpt2".
    /// - `model_name`: If provided, resolves the encoding from the model name (e.g., "gpt-3.5-turbo").
    /// - `config`: Text splitter configuration. `chunk_size` controls tokens per chunk, `chunk_overlap` controls token overlap.
    pub fn new(
        encoding_name: Option<&str>,
        model_name: Option<&str>,
        config: TextSplitterConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let bpe = resolve_tiktoken_bpe(encoding_name, model_name)?;
        Ok(Self {
            config,
            tokenizer: Arc::new(bpe),
        })
    }

    /// Create a `TokenTextSplitter` from a tiktoken encoder, matching Python's
    /// `TokenTextSplitter.from_tiktoken_encoder()` classmethod.
    ///
    /// This sets the config's `length_function` to count tokens using tiktoken,
    /// and uses the same encoding for the internal tokenizer.
    pub fn from_tiktoken_encoder(
        encoding_name: Option<&str>,
        model_name: Option<&str>,
        mut config: TextSplitterConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let bpe = Arc::new(resolve_tiktoken_bpe(encoding_name, model_name)?);
        let bpe_for_length = bpe.clone();
        config.length_function =
            Arc::new(move |text: &str| -> usize { bpe_for_length.encode_ordinary(text).len() });
        Ok(Self {
            config,
            tokenizer: bpe,
        })
    }

    /// Returns a reference to the internal `CoreBPE` tokenizer.
    pub fn tokenizer(&self) -> &CoreBPE {
        &self.tokenizer
    }
}

#[async_trait]
impl TextSplitter for TokenTextSplitter {
    fn config(&self) -> &TextSplitterConfig {
        &self.config
    }

    fn split_text(
        &self,
        text: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let tokenizer_ref = self.tokenizer.clone();
        let tokenizer_ref2 = self.tokenizer.clone();

        let tokenizer = Tokenizer {
            chunk_overlap: self.config.chunk_overlap,
            tokens_per_chunk: self.config.chunk_size,
            encode: Box::new(move |text: &str| {
                tokenizer_ref
                    .encode_ordinary(text)
                    .into_iter()
                    .map(|rank| rank as i64)
                    .collect()
            }),
            decode: Box::new(move |ids: &[i64]| {
                let ranks: Vec<u32> = ids.iter().map(|&id| id as u32).collect();
                tokenizer_ref2.decode(ranks).unwrap_or_default()
            }),
        };

        split_text_on_tokens(text, &tokenizer)
    }
}

#[async_trait]
impl BaseDocumentTransformer for TokenTextSplitter {
    fn transform_documents(
        &self,
        documents: Vec<Document>,
        _kwargs: HashMap<String, serde_json::Value>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        self.split_documents(&documents)
    }
}
