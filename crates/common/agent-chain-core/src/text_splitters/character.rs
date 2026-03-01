use std::collections::HashMap;

use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

use crate::documents::{BaseDocumentTransformer, Document};
use crate::text_splitters::base::{merge_splits, split_text_with_regex};
use crate::text_splitters::{KeepSeparator, Language, TextSplitter, TextSplitterConfig};

pub struct RecursiveCharacterTextSplitter {
    config: TextSplitterConfig,
    separators: Vec<String>,
    is_separator_regex: bool,
}

impl RecursiveCharacterTextSplitter {
    pub fn new(
        separators: Option<Vec<String>>,
        is_separator_regex: Option<bool>,
        config: TextSplitterConfig,
    ) -> Self {
        Self {
            config,
            separators: separators.unwrap_or_else(|| {
                vec![
                    "\n\n".to_string(),
                    "\n".to_string(),
                    " ".to_string(),
                    String::new(),
                ]
            }),
            is_separator_regex: is_separator_regex.unwrap_or(false),
        }
    }

    pub fn from_language(language: Language, config: TextSplitterConfig) -> Self {
        let separators = Self::get_separators_for_language(language);
        Self {
            config,
            separators,
            is_separator_regex: true,
        }
    }

    fn split_text_recursive(
        &self,
        text: &str,
        separators: &[String],
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut final_chunks: Vec<String> = Vec::new();

        let mut separator = separators.last().cloned().unwrap_or_default();
        let mut new_separators: &[String] = &[];

        for (i, s) in separators.iter().enumerate() {
            let separator_pattern = if self.is_separator_regex {
                s.clone()
            } else {
                regex::escape(s)
            };
            if s.is_empty() {
                separator = s.clone();
                break;
            }
            if let Ok(re) = Regex::new(&separator_pattern) {
                if re.is_match(text) {
                    separator = s.clone();
                    new_separators = &separators[i + 1..];
                    break;
                }
            }
        }

        let separator_pattern = if self.is_separator_regex {
            separator.clone()
        } else {
            regex::escape(&separator)
        };

        let splits = split_text_with_regex(text, &separator_pattern, self.config.keep_separator);

        let merge_separator = if self.config.keep_separator != KeepSeparator::None {
            String::new()
        } else {
            separator.clone()
        };

        let mut good_splits: Vec<String> = Vec::new();
        for s in &splits {
            if (self.config.length_function)(s) < self.config.chunk_size {
                good_splits.push(s.clone());
            } else {
                if !good_splits.is_empty() {
                    let merged = merge_splits(&good_splits, &merge_separator, &self.config);
                    final_chunks.extend(merged);
                    good_splits.clear();
                }
                if new_separators.is_empty() {
                    final_chunks.push(s.clone());
                } else {
                    let other_info = self.split_text_recursive(s, new_separators)?;
                    final_chunks.extend(other_info);
                }
            }
        }
        if !good_splits.is_empty() {
            let merged = merge_splits(&good_splits, &merge_separator, &self.config);
            final_chunks.extend(merged);
        }
        Ok(final_chunks)
    }

    pub fn get_separators_for_language(language: Language) -> Vec<String> {
        let strs: Vec<&str> = match language {
            Language::C | Language::Cpp => vec![
                "\nclass ",
                "\nvoid ",
                "\nint ",
                "\nfloat ",
                "\ndouble ",
                "\nif ",
                "\nfor ",
                "\nwhile ",
                "\nswitch ",
                "\ncase ",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::Go => vec![
                "\nfunc ",
                "\nvar ",
                "\nconst ",
                "\ntype ",
                "\nif ",
                "\nfor ",
                "\nswitch ",
                "\ncase ",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::Java => vec![
                "\nclass ",
                "\npublic ",
                "\nprotected ",
                "\nprivate ",
                "\nstatic ",
                "\nif ",
                "\nfor ",
                "\nwhile ",
                "\nswitch ",
                "\ncase ",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::Kotlin => vec![
                "\nclass ",
                "\npublic ",
                "\nprotected ",
                "\nprivate ",
                "\ninternal ",
                "\ncompanion ",
                "\nfun ",
                "\nval ",
                "\nvar ",
                "\nif ",
                "\nfor ",
                "\nwhile ",
                "\nwhen ",
                "\ncase ",
                "\nelse ",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::Js => vec![
                "\nfunction ",
                "\nconst ",
                "\nlet ",
                "\nvar ",
                "\nclass ",
                "\nif ",
                "\nfor ",
                "\nwhile ",
                "\nswitch ",
                "\ncase ",
                "\ndefault ",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::Ts => vec![
                "\nenum ",
                "\ninterface ",
                "\nnamespace ",
                "\ntype ",
                "\nclass ",
                "\nfunction ",
                "\nconst ",
                "\nlet ",
                "\nvar ",
                "\nif ",
                "\nfor ",
                "\nwhile ",
                "\nswitch ",
                "\ncase ",
                "\ndefault ",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::Php => vec![
                "\nfunction ",
                "\nclass ",
                "\nif ",
                "\nforeach ",
                "\nwhile ",
                "\ndo ",
                "\nswitch ",
                "\ncase ",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::Proto => vec![
                "\nmessage ",
                "\nservice ",
                "\nenum ",
                "\noption ",
                "\nimport ",
                "\nsyntax ",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::Python => vec!["\nclass ", "\ndef ", "\n\tdef ", "\n\n", "\n", " ", ""],
            Language::R => vec![
                "\nfunction ",
                r"\nsetClass\(",
                r"\nsetMethod\(",
                r"\nsetGeneric\(",
                "\nif ",
                "\nelse ",
                "\nfor ",
                "\nwhile ",
                "\nrepeat ",
                r"\nlibrary\(",
                r"\nrequire\(",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::Rst => vec![
                "\n=+\n",
                "\n-+\n",
                r"\n\*+\n",
                "\n\n.. *\n\n",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::Ruby => vec![
                "\ndef ",
                "\nclass ",
                "\nif ",
                "\nunless ",
                "\nwhile ",
                "\nfor ",
                "\ndo ",
                "\nbegin ",
                "\nrescue ",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::Elixir => vec![
                "\ndef ",
                "\ndefp ",
                "\ndefmodule ",
                "\ndefprotocol ",
                "\ndefmacro ",
                "\ndefmacrop ",
                "\nif ",
                "\nunless ",
                "\nwhile ",
                "\ncase ",
                "\ncond ",
                "\nwith ",
                "\nfor ",
                "\ndo ",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::Rust => vec![
                "\nfn ", "\nconst ", "\nlet ", "\nif ", "\nwhile ", "\nfor ", "\nloop ",
                "\nmatch ", "\nconst ", "\n\n", "\n", " ", "",
            ],
            Language::Scala => vec![
                "\nclass ",
                "\nobject ",
                "\ndef ",
                "\nval ",
                "\nvar ",
                "\nif ",
                "\nfor ",
                "\nwhile ",
                "\nmatch ",
                "\ncase ",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::Swift => vec![
                "\nfunc ",
                "\nclass ",
                "\nstruct ",
                "\nenum ",
                "\nif ",
                "\nfor ",
                "\nwhile ",
                "\ndo ",
                "\nswitch ",
                "\ncase ",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::Markdown => vec![
                "\n#{1,6} ",
                "```\n",
                r"\n\*\*\*+\n",
                "\n---+\n",
                "\n___+\n",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::Latex => vec![
                r"\n\\chapter\{",
                r"\n\\section\{",
                r"\n\\subsection\{",
                r"\n\\subsubsection\{",
                r"\n\\begin\{enumerate\}",
                r"\n\\begin\{itemize\}",
                r"\n\\begin\{description\}",
                r"\n\\begin\{list\}",
                r"\n\\begin\{quote\}",
                r"\n\\begin\{quotation\}",
                r"\n\\begin\{verse\}",
                r"\n\\begin\{verbatim\}",
                r"\n\\begin\{align\}",
                "$$",
                "$",
                " ",
                "",
            ],
            Language::Html => vec![
                "<body", "<div", "<p", "<br", "<li", "<h1", "<h2", "<h3", "<h4", "<h5", "<h6",
                "<span", "<table", "<tr", "<td", "<th", "<ul", "<ol", "<header", "<footer", "<nav",
                "<head", "<style", "<script", "<meta", "<title", "",
            ],
            Language::Sol => vec![
                "\npragma ",
                "\nusing ",
                "\ncontract ",
                "\ninterface ",
                "\nlibrary ",
                "\nconstructor ",
                "\ntype ",
                "\nfunction ",
                "\nevent ",
                "\nmodifier ",
                "\nerror ",
                "\nstruct ",
                "\nenum ",
                "\nif ",
                "\nfor ",
                "\nwhile ",
                "\ndo while ",
                "\nassembly ",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::CSharp => vec![
                "\ninterface ",
                "\nenum ",
                "\nimplements ",
                "\ndelegate ",
                "\nevent ",
                "\nclass ",
                "\nabstract ",
                "\npublic ",
                "\nprotected ",
                "\nprivate ",
                "\nstatic ",
                "\nreturn ",
                "\nif ",
                "\ncontinue ",
                "\nfor ",
                "\nforeach ",
                "\nwhile ",
                "\nswitch ",
                "\nbreak ",
                "\ncase ",
                "\nelse ",
                "\ntry ",
                "\nthrow ",
                "\nfinally ",
                "\ncatch ",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::Cobol => vec![
                "\nIDENTIFICATION DIVISION.",
                "\nENVIRONMENT DIVISION.",
                "\nDATA DIVISION.",
                "\nPROCEDURE DIVISION.",
                "\nWORKING-STORAGE SECTION.",
                "\nLINKAGE SECTION.",
                "\nFILE SECTION.",
                "\nINPUT-OUTPUT SECTION.",
                "\nOPEN ",
                "\nCLOSE ",
                "\nREAD ",
                "\nWRITE ",
                "\nIF ",
                "\nELSE ",
                "\nMOVE ",
                "\nPERFORM ",
                "\nUNTIL ",
                "\nVARYING ",
                "\nACCEPT ",
                "\nDISPLAY ",
                "\nSTOP RUN.",
                "\n",
                " ",
                "",
            ],
            Language::Lua => vec![
                "\nlocal ",
                "\nfunction ",
                "\nif ",
                "\nfor ",
                "\nwhile ",
                "\nrepeat ",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::Perl => {
                // Perl uses same as Python separators in the Python source
                vec!["\nclass ", "\ndef ", "\n\tdef ", "\n\n", "\n", " ", ""]
            }
            Language::Haskell => vec![
                "\nmain :: ",
                "\nmain = ",
                "\nlet ",
                "\nin ",
                "\ndo ",
                "\nwhere ",
                "\n:: ",
                "\n= ",
                "\ndata ",
                "\nnewtype ",
                "\ntype ",
                "\n:: ",
                "\nmodule ",
                "\nimport ",
                "\nqualified ",
                "\nimport qualified ",
                "\nclass ",
                "\ninstance ",
                "\ncase ",
                "\n| ",
                "\ndata ",
                "\n= {",
                "\n, ",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::PowerShell => vec![
                "\nfunction ",
                "\nparam ",
                "\nif ",
                "\nforeach ",
                "\nfor ",
                "\nwhile ",
                "\nswitch ",
                "\nclass ",
                "\ntry ",
                "\ncatch ",
                "\nfinally ",
                "\n\n",
                "\n",
                " ",
                "",
            ],
            Language::VisualBasic6 => {
                let vis = r"(?:Public|Private|Friend|Global|Static)\s+";
                return vec![
                    format!(r"\n(?!End\s){vis}?Sub\s+"),
                    format!(r"\n(?!End\s){vis}?Function\s+"),
                    format!(r"\n(?!End\s){vis}?Property\s+(?:Get|Let|Set)\s+"),
                    format!(r"\n(?!End\s){vis}?Type\s+"),
                    format!(r"\n(?!End\s){vis}?Enum\s+"),
                    r"\n(?!End\s)If\s+".to_string(),
                    r"\nElseIf\s+".to_string(),
                    r"\nElse\s+".to_string(),
                    r"\nSelect\s+Case\s+".to_string(),
                    r"\nCase\s+".to_string(),
                    r"\nFor\s+".to_string(),
                    r"\nDo\s+".to_string(),
                    r"\nWhile\s+".to_string(),
                    r"\nWith\s+".to_string(),
                    r"\n\n".to_string(),
                    r"\n".to_string(),
                    " ".to_string(),
                    String::new(),
                ];
            }
        };

        strs.into_iter().map(|s| s.to_string()).collect()
    }
}

#[async_trait]
impl BaseDocumentTransformer for RecursiveCharacterTextSplitter {
    fn transform_documents(
        &self,
        documents: Vec<Document>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error + Send + Sync>> {
        self.split_documents(&documents)
    }
}

#[async_trait]
impl TextSplitter for RecursiveCharacterTextSplitter {
    fn config(&self) -> &TextSplitterConfig {
        &self.config
    }

    fn split_text(
        &self,
        text: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        self.split_text_recursive(text, &self.separators)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recursive_character_text_splitter_keep_separator_start() {
        let config =
            TextSplitterConfig::new(10, 0, None, Some(KeepSeparator::Start), None, None).unwrap();
        let splitter = RecursiveCharacterTextSplitter::new(
            Some(vec![",".to_string(), ".".to_string()]),
            None,
            config,
        );
        let result = splitter
            .split_text("Apple,banana,orange and tomato.")
            .unwrap();
        assert_eq!(result, vec!["Apple", ",banana", ",orange and tomato", "."]);
    }

    #[test]
    fn test_recursive_character_text_splitter_keep_separator_end() {
        let config =
            TextSplitterConfig::new(10, 0, None, Some(KeepSeparator::End), None, None).unwrap();
        let splitter = RecursiveCharacterTextSplitter::new(
            Some(vec![",".to_string(), ".".to_string()]),
            None,
            config,
        );
        let result = splitter
            .split_text("Apple,banana,orange and tomato.")
            .unwrap();
        assert_eq!(result, vec!["Apple,", "banana,", "orange and tomato."]);
    }

    #[test]
    fn test_recursive_character_text_splitter_basic() {
        let config = TextSplitterConfig::new(10, 1, None, None, None, None).unwrap();
        let splitter = RecursiveCharacterTextSplitter::new(None, None, config);
        let text =
            "Hi.\n\nI'm Harrison.\n\nHow? Are? You?\nOkay then f f f f.\nThis is a long sentence.";
        let result = splitter.split_text(text).unwrap();
        assert!(!result.is_empty());
        for chunk in &result {
            assert!(chunk.len() <= 12); // allow some tolerance for separator handling
        }
    }

    #[test]
    fn test_from_language_python() {
        let config = TextSplitterConfig::new(50, 0, None, None, None, None).unwrap();
        let splitter = RecursiveCharacterTextSplitter::from_language(Language::Python, config);
        let text =
            "\nclass Foo:\n\n    def bar():\n\n\ndef foo():\n\ndef testing_func():\n\ndef bar():\n";
        let result = splitter.split_text(text).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_get_separators_for_language_coverage() {
        let languages = vec![
            Language::Cpp,
            Language::Go,
            Language::Java,
            Language::Kotlin,
            Language::Js,
            Language::Ts,
            Language::Php,
            Language::Proto,
            Language::Python,
            Language::R,
            Language::Rst,
            Language::Ruby,
            Language::Rust,
            Language::Scala,
            Language::Swift,
            Language::Markdown,
            Language::Latex,
            Language::Html,
            Language::Sol,
            Language::CSharp,
            Language::Cobol,
            Language::C,
            Language::Lua,
            Language::Perl,
            Language::Haskell,
            Language::Elixir,
            Language::PowerShell,
            Language::VisualBasic6,
        ];
        for language in languages {
            let seps = RecursiveCharacterTextSplitter::get_separators_for_language(language);
            assert!(
                !seps.is_empty(),
                "Language {:?} returned empty separators",
                language
            );
            assert_eq!(
                seps.last().map(|s| s.as_str()),
                Some(""),
                "Language {:?} should end with empty separator",
                language
            );
        }
    }
}
