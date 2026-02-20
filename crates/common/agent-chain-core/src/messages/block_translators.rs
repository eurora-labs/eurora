pub mod anthropic;
pub mod bedrock;
pub mod bedrock_converse;
pub mod google_genai;
pub mod google_vertexai;
pub mod groq;
pub mod langchain_v0;
pub mod openai;

use serde_json::Value;

pub type TranslatorFn = fn(&[Value], bool) -> Vec<Value>;

pub fn get_translator(provider: &str) -> Option<TranslatorFn> {
    match provider {
        "anthropic" => Some(anthropic::convert_to_standard_blocks),
        "bedrock" => Some(bedrock::convert_to_standard_blocks),
        "bedrock_converse" => Some(bedrock_converse::convert_to_standard_blocks),
        "google_genai" => Some(google_genai::convert_to_standard_blocks),
        "google_vertexai" => Some(google_vertexai::convert_to_standard_blocks),
        "groq" => Some(groq::convert_to_standard_blocks),
        "openai" => Some(openai::convert_to_standard_blocks),
        _ => None,
    }
}
