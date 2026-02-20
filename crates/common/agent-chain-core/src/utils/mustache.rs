use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum MustacheError {
    SyntaxError(String),
    UnclosedTag {
        line: usize,
    },
    UnclosedSection {
        tag: String,
        line: usize,
    },
    MismatchedSection {
        expected: String,
        got: String,
        line: usize,
    },
    EmptyTag {
        line: usize,
    },
}

impl std::fmt::Display for MustacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MustacheError::SyntaxError(msg) => write!(f, "Syntax error: {}", msg),
            MustacheError::UnclosedTag { line } => write!(f, "Unclosed tag at line {}", line),
            MustacheError::UnclosedSection { tag, line } => {
                write!(f, "Unclosed section '{}' opened at line {}", tag, line)
            }
            MustacheError::MismatchedSection {
                expected,
                got,
                line,
            } => {
                write!(
                    f,
                    "Mismatched section at line {}: expected '{}', got '{}'",
                    line, expected, got
                )
            }
            MustacheError::EmptyTag { line } => write!(f, "Empty tag at line {}", line),
        }
    }
}

impl std::error::Error for MustacheError {}

#[derive(Debug, Clone)]
pub enum MustacheValue {
    String(String),
    Bool(bool),
    Int(i64),
    Float(f64),
    List(Vec<MustacheValue>),
    Map(HashMap<String, MustacheValue>),
    Null,
}

impl MustacheValue {
    pub fn is_truthy(&self) -> bool {
        match self {
            MustacheValue::String(s) => !s.is_empty(),
            MustacheValue::Bool(b) => *b,
            MustacheValue::Int(i) => *i != 0,
            MustacheValue::Float(f) => *f != 0.0,
            MustacheValue::List(l) => !l.is_empty(),
            MustacheValue::Map(_) => true,
            MustacheValue::Null => false,
        }
    }

    pub fn to_output_string(&self) -> String {
        match self {
            MustacheValue::String(s) => s.clone(),
            MustacheValue::Bool(b) => b.to_string(),
            MustacheValue::Int(i) => i.to_string(),
            MustacheValue::Float(f) => f.to_string(),
            MustacheValue::List(_) => String::new(),
            MustacheValue::Map(_) => String::new(),
            MustacheValue::Null => String::new(),
        }
    }
}

impl From<String> for MustacheValue {
    fn from(s: String) -> Self {
        MustacheValue::String(s)
    }
}

impl From<&str> for MustacheValue {
    fn from(s: &str) -> Self {
        MustacheValue::String(s.to_string())
    }
}

impl From<bool> for MustacheValue {
    fn from(b: bool) -> Self {
        MustacheValue::Bool(b)
    }
}

impl From<i64> for MustacheValue {
    fn from(i: i64) -> Self {
        MustacheValue::Int(i)
    }
}

impl From<i32> for MustacheValue {
    fn from(i: i32) -> Self {
        MustacheValue::Int(i as i64)
    }
}

impl From<f64> for MustacheValue {
    fn from(f: f64) -> Self {
        MustacheValue::Float(f)
    }
}

impl From<Vec<MustacheValue>> for MustacheValue {
    fn from(v: Vec<MustacheValue>) -> Self {
        MustacheValue::List(v)
    }
}

impl From<HashMap<String, MustacheValue>> for MustacheValue {
    fn from(m: HashMap<String, MustacheValue>) -> Self {
        MustacheValue::Map(m)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum TokenType {
    Literal,
    Variable,
    NoEscape,
    Section,
    InvertedSection,
    End,
    Partial,
    Comment,
}

#[derive(Debug, Clone)]
struct Token {
    token_type: TokenType,
    key: String,
}

fn tokenize(template: &str, l_del: &str, r_del: &str) -> Result<Vec<Token>, MustacheError> {
    let mut tokens = Vec::new();
    let mut current_line = 1;
    let mut open_sections: Vec<(String, usize)> = Vec::new();
    let mut remaining = template;

    while !remaining.is_empty() {
        if let Some(pos) = remaining.find(l_del) {
            if pos > 0 {
                let literal = &remaining[..pos];
                current_line += literal.matches('\n').count();
                tokens.push(Token {
                    token_type: TokenType::Literal,
                    key: literal.to_string(),
                });
            }
            remaining = &remaining[pos + l_del.len()..];

            if let Some(end_pos) = remaining.find(r_del) {
                let tag = &remaining[..end_pos];
                remaining = &remaining[end_pos + r_del.len()..];

                if tag.is_empty() {
                    return Err(MustacheError::EmptyTag { line: current_line });
                }

                let first_char = tag.chars().next().expect("checked non-empty above");
                let (token_type, key) = match first_char {
                    '!' => (TokenType::Comment, tag[1..].trim().to_string()),
                    '#' => {
                        let key = tag[1..].trim().to_string();
                        open_sections.push((key.clone(), current_line));
                        (TokenType::Section, key)
                    }
                    '^' => {
                        let key = tag[1..].trim().to_string();
                        open_sections.push((key.clone(), current_line));
                        (TokenType::InvertedSection, key)
                    }
                    '/' => {
                        let key = tag[1..].trim().to_string();
                        if let Some((expected, _)) = open_sections.pop()
                            && expected != key
                        {
                            return Err(MustacheError::MismatchedSection {
                                expected,
                                got: key,
                                line: current_line,
                            });
                        }
                        (TokenType::End, key)
                    }
                    '>' => (TokenType::Partial, tag[1..].trim().to_string()),
                    '&' => (TokenType::NoEscape, tag[1..].trim().to_string()),
                    '{' => {
                        let tag = tag[1..].trim();
                        let tag = if let Some(stripped) = tag.strip_suffix('}') {
                            stripped.trim()
                        } else {
                            if remaining.starts_with('}') {
                                remaining = &remaining[1..];
                            }
                            tag
                        };
                        (TokenType::NoEscape, tag.to_string())
                    }
                    _ => (TokenType::Variable, tag.trim().to_string()),
                };

                if token_type != TokenType::Comment {
                    tokens.push(Token { token_type, key });
                }
            } else {
                return Err(MustacheError::UnclosedTag { line: current_line });
            }
        } else {
            if !remaining.is_empty() {
                tokens.push(Token {
                    token_type: TokenType::Literal,
                    key: remaining.to_string(),
                });
            }
            break;
        }
    }

    if let Some((tag, line)) = open_sections.pop() {
        return Err(MustacheError::UnclosedSection { tag, line });
    }

    Ok(tokens)
}

fn html_escape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            _ => result.push(c),
        }
    }
    result
}

fn get_key(key: &str, scopes: &[&MustacheValue]) -> MustacheValue {
    if key == "." {
        return scopes
            .first()
            .map(|v| (*v).clone())
            .unwrap_or(MustacheValue::Null);
    }

    for scope in scopes {
        if let MustacheValue::Map(map) = scope {
            let parts: Vec<&str> = key.split('.').collect();
            let mut current: Option<&MustacheValue> = map.get(parts[0]);

            for part in parts.iter().skip(1) {
                if let Some(MustacheValue::Map(m)) = current {
                    current = m.get(*part);
                } else {
                    current = None;
                    break;
                }
            }

            if let Some(value) = current {
                return value.clone();
            }
        }
    }

    MustacheValue::Null
}

pub fn render(
    template: &str,
    data: &MustacheValue,
    partials: Option<&HashMap<String, String>>,
) -> Result<String, MustacheError> {
    render_with_delimiters(template, data, partials, "{{", "}}")
}

pub fn render_with_delimiters(
    template: &str,
    data: &MustacheValue,
    partials: Option<&HashMap<String, String>>,
    l_del: &str,
    r_del: &str,
) -> Result<String, MustacheError> {
    let tokens = tokenize(template, l_del, r_del)?;
    let scopes = vec![data];
    render_tokens(&tokens, &scopes, partials, l_del, r_del)
}

fn render_tokens(
    tokens: &[Token],
    scopes: &[&MustacheValue],
    partials: Option<&HashMap<String, String>>,
    l_del: &str,
    r_del: &str,
) -> Result<String, MustacheError> {
    let mut output = String::new();
    let mut i = 0;

    while i < tokens.len() {
        let token = &tokens[i];

        match token.token_type {
            TokenType::Literal => {
                output.push_str(&token.key);
            }
            TokenType::Variable => {
                let value = get_key(&token.key, scopes);
                output.push_str(&html_escape(&value.to_output_string()));
            }
            TokenType::NoEscape => {
                let value = get_key(&token.key, scopes);
                output.push_str(&value.to_output_string());
            }
            TokenType::Section => {
                let value = get_key(&token.key, scopes);
                let end_index = find_section_end(tokens, i, &token.key);

                if value.is_truthy() {
                    let section_tokens = &tokens[i + 1..end_index];
                    match &value {
                        MustacheValue::List(items) => {
                            for item in items {
                                let mut new_scopes = vec![item];
                                new_scopes.extend(scopes.iter());
                                output.push_str(&render_tokens(
                                    section_tokens,
                                    &new_scopes,
                                    partials,
                                    l_del,
                                    r_del,
                                )?);
                            }
                        }
                        _ => {
                            let mut new_scopes = vec![&value];
                            new_scopes.extend(scopes.iter());
                            output.push_str(&render_tokens(
                                section_tokens,
                                &new_scopes,
                                partials,
                                l_del,
                                r_del,
                            )?);
                        }
                    }
                }

                i = end_index;
            }
            TokenType::InvertedSection => {
                let value = get_key(&token.key, scopes);
                let end_index = find_section_end(tokens, i, &token.key);

                if !value.is_truthy() {
                    let section_tokens = &tokens[i + 1..end_index];
                    output.push_str(&render_tokens(
                        section_tokens,
                        scopes,
                        partials,
                        l_del,
                        r_del,
                    )?);
                }

                i = end_index;
            }
            TokenType::Partial => {
                if let Some(partials_map) = partials
                    && let Some(partial_template) = partials_map.get(&token.key)
                {
                    output.push_str(&render_with_delimiters(
                        partial_template,
                        scopes[0],
                        partials,
                        l_del,
                        r_del,
                    )?);
                }
            }
            TokenType::End | TokenType::Comment => {}
        }

        i += 1;
    }

    Ok(output)
}

fn find_section_end(tokens: &[Token], start: usize, key: &str) -> usize {
    let mut depth = 1;
    for (i, token) in tokens.iter().enumerate().skip(start + 1) {
        match &token.token_type {
            TokenType::Section | TokenType::InvertedSection if token.key == key => {
                depth += 1;
            }
            TokenType::End if token.key == key => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    tokens.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_data(pairs: &[(&str, MustacheValue)]) -> MustacheValue {
        let mut map = HashMap::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v.clone());
        }
        MustacheValue::Map(map)
    }

    #[test]
    fn test_simple_variable() {
        let data = make_data(&[("name", "World".into())]);
        let result = render("Hello, {{name}}!", &data, None).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_html_escape() {
        let data = make_data(&[("html", "<b>bold</b>".into())]);
        let result = render("{{html}}", &data, None).unwrap();
        assert_eq!(result, "&lt;b&gt;bold&lt;/b&gt;");
    }

    #[test]
    fn test_no_escape() {
        let data = make_data(&[("html", "<b>bold</b>".into())]);
        let result = render("{{{html}}}", &data, None).unwrap();
        assert_eq!(result, "<b>bold</b>");
    }

    #[test]
    fn test_section() {
        let data = make_data(&[("show", true.into())]);
        let result = render("{{#show}}Shown{{/show}}", &data, None).unwrap();
        assert_eq!(result, "Shown");
    }

    #[test]
    fn test_section_false() {
        let data = make_data(&[("show", false.into())]);
        let result = render("{{#show}}Hidden{{/show}}", &data, None).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_inverted_section() {
        let data = make_data(&[("show", false.into())]);
        let result = render("{{^show}}Shown{{/show}}", &data, None).unwrap();
        assert_eq!(result, "Shown");
    }

    #[test]
    fn test_list() {
        let items = [
            make_data(&[("name", "Alice".into())]),
            make_data(&[("name", "Bob".into())]),
        ];
        let data = make_data(&[(
            "items",
            MustacheValue::List(vec![items[0].clone(), items[1].clone()]),
        )]);
        let result = render("{{#items}}{{name}} {{/items}}", &data, None).unwrap();
        assert_eq!(result, "Alice Bob ");
    }

    #[test]
    fn test_dot_notation() {
        let person = make_data(&[("name", "John".into())]);
        let data = make_data(&[("person", person)]);
        let result = render("{{person.name}}", &data, None).unwrap();
        assert_eq!(result, "John");
    }

    #[test]
    fn test_partial() {
        let data = make_data(&[("name", "World".into())]);
        let mut partials = HashMap::new();
        partials.insert("greeting".to_string(), "Hello, {{name}}!".to_string());
        let result = render("{{>greeting}}", &data, Some(&partials)).unwrap();
        assert_eq!(result, "Hello, World!");
    }
}
