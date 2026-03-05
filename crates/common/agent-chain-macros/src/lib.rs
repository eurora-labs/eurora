//! Procedural macros for agent-chain.
//!
//! Provides the `#[tool]` attribute macro that mirrors Python's `@tool` decorator
//! from langchain_core. It converts a Rust function into a struct implementing
//! the `BaseTool` trait, enabling LLM agents to invoke it via function calling.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::{
    Expr, ExprLit, FnArg, GenericArgument, ItemFn, Lit, Meta, MetaNameValue, Pat, PathArguments,
    ReturnType, Token, Type, parse_macro_input,
};

/// Marks a function as a tool that can be used by an LLM.
///
/// This macro generates a module containing a struct that implements the `BaseTool` trait,
/// allowing the function to be invoked by an AI model via function calling.
///
/// # Attributes
///
/// - `name = "custom_name"` — Override the tool name (defaults to function name)
/// - `description = "..."` — Override the description (defaults to doc comment, then "Tool: {name}")
/// - `return_direct = true` — Signal the agent to stop after this tool returns
/// - `response_format = "content_and_artifact"` — Set response format
///
/// # Return Types
///
/// Tools can return either a plain value or a `Result`:
///
/// - `fn foo() -> String` — Always succeeds, result is serialized
/// - `fn foo() -> Result<String>` — Errors propagate through `BaseTool`'s error handling
///   (matching Python's `ToolException` behavior)
///
/// When returning `Result`, errors are propagated to `BaseTool::run`/`arun` which applies
/// `handle_tool_error` and `handle_validation_error` policies — just like Python's `@tool`
/// propagates exceptions through `BaseTool.run()`.
///
/// # Examples
///
/// ```ignore
/// use agent_chain_core::tools::tool;
///
/// /// Multiply two numbers together.
/// #[tool]
/// fn multiply(a: i64, b: i64) -> i64 {
///     a * b
/// }
///
/// /// Search the web (errors propagate to handle_tool_error).
/// #[tool]
/// async fn search(query: String) -> Result<String> {
///     let response = reqwest::get(&query).await
///         .map_err(|e| Error::ToolException(format!("Request failed: {e}")))?;
///     Ok(response.text().await.unwrap_or_default())
/// }
///
/// /// Greet someone with an optional greeting.
/// #[tool]
/// fn greet(name: String, greeting: Option<String>) -> String {
///     let greeting = greeting.unwrap_or_else(|| "Hello".to_string());
///     format!("{}, {}!", greeting, name)
/// }
/// ```
#[proc_macro_attribute]
pub fn tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let attrs = parse_tool_attrs(attr);

    let fn_name = &input.sig.ident;
    let fn_name_str = fn_name.to_string();
    let mod_name = format_ident!("{}", fn_name);
    let struct_name = format_ident!("{}Tool", to_pascal_case(&fn_name_str));
    let is_async = input.sig.asyncness.is_some();

    let fn_body = &input.block;
    let fn_vis = &input.vis;
    let fn_return_type = &input.sig.output;

    let doc_comment = extract_doc_comment(&input);

    let tool_name = attrs.name.unwrap_or_else(|| fn_name_str.clone());

    let tool_description = attrs.description.unwrap_or_else(|| {
        doc_comment
            .clone()
            .unwrap_or_else(|| format!("Tool: {}", tool_name))
    });

    let return_direct = attrs.return_direct;

    let response_format_expr = match attrs.response_format.as_deref() {
        Some("content_and_artifact") => {
            quote! { ::agent_chain::_core::tools::ResponseFormat::ContentAndArtifact }
        }
        _ => quote! { ::agent_chain::_core::tools::ResponseFormat::Content },
    };

    // Extract parameters
    let params: Vec<ParamInfo> = input
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg
                && let Pat::Ident(pat_ident) = pat_type.pat.as_ref()
            {
                let param_name = pat_ident.ident.clone();
                let param_type = pat_type.ty.as_ref().clone();
                let is_option = is_option_type(&param_type);
                let inner_type = if is_option {
                    extract_option_inner(&param_type)
                } else {
                    None
                };
                return Some(ParamInfo {
                    name: param_name,
                    ty: param_type,
                    is_option,
                    inner_type,
                });
            }
            None
        })
        .collect();

    let param_names: Vec<_> = params.iter().map(|p| &p.name).collect();
    let param_types: Vec<_> = params.iter().map(|p| &p.ty).collect();

    // JSON schema properties
    let schema_properties: Vec<_> = params
        .iter()
        .map(|p| {
            let name_str = p.name.to_string();
            let type_json = if p.is_option {
                let inner = p.inner_type.as_ref().unwrap_or(&p.ty);
                get_json_schema(inner)
            } else {
                get_json_schema(&p.ty)
            };
            quote! { (#name_str.to_string(), #type_json) }
        })
        .collect();

    let required_params: Vec<_> = params
        .iter()
        .filter(|p| !p.is_option)
        .map(|p| {
            let name_str = p.name.to_string();
            quote! { #name_str.to_string() }
        })
        .collect();

    // Detect if return type is Result<T>
    let returns_result = is_result_type(fn_return_type);

    let actual_return_type = match fn_return_type {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, ty) => quote! { #ty },
    };

    // --- invoke_tool_call param extraction (returns ToolMessage on error) ---
    let invoke_param_extractions: Vec<_> = params
        .iter()
        .map(|p| {
            let name = &p.name;
            let name_str = p.name.to_string();
            let ty = &p.ty;

            if p.is_option {
                quote! {
                    let #name: #ty = match args.get(#name_str) {
                        Some(v) if !v.is_null() => {
                            match serde_json::from_value(v.clone()) {
                                Ok(val) => val,
                                Err(e) => return ::agent_chain::_core::messages::ToolMessage::builder()
                                    .content(format!("Failed to parse parameter '{}': {}", #name_str, e))
                                    .tool_call_id(tool_call.id.clone().unwrap_or_default())
                                    .status(::agent_chain::_core::messages::ToolStatus::Error)
                                    .build()
                                    .into(),
                            }
                        }
                        _ => None,
                    };
                }
            } else {
                quote! {
                    let #name: #ty = match args.get(#name_str) {
                        Some(v) => match serde_json::from_value(v.clone()) {
                            Ok(val) => val,
                            Err(e) => return ::agent_chain::_core::messages::ToolMessage::builder()
                                .content(format!("Failed to parse parameter '{}': {}", #name_str, e))
                                .tool_call_id(tool_call.id.clone().unwrap_or_default())
                                .status(::agent_chain::_core::messages::ToolStatus::Error)
                                .build()
                                .into(),
                        },
                        None => return ::agent_chain::_core::messages::ToolMessage::builder()
                            .content(format!("Missing required parameter '{}'", #name_str))
                            .tool_call_id(tool_call.id.clone().unwrap_or_default())
                            .status(::agent_chain::_core::messages::ToolStatus::Error)
                            .build()
                            .into(),
                    };
                }
            }
        })
        .collect();

    // --- tool_run/tool_arun param extraction (returns Err on failure) ---
    let result_param_extractions: Vec<_> = params
        .iter()
        .map(|p| {
            let name = &p.name;
            let name_str = p.name.to_string();
            let ty = &p.ty;

            if p.is_option {
                quote! {
                    let #name: #ty = match args.get(#name_str) {
                        Some(v) if !v.is_null() => serde_json::from_value(v.clone())
                            .map_err(|e| ::agent_chain::_core::error::Error::ValidationError(
                                format!("Failed to parse parameter '{}': {}", #name_str, e)
                            ))?,
                        _ => None,
                    };
                }
            } else {
                quote! {
                    let #name: #ty = match args.get(#name_str) {
                        Some(v) => serde_json::from_value(v.clone())
                            .map_err(|e| ::agent_chain::_core::error::Error::ValidationError(
                                format!("Failed to parse parameter '{}': {}", #name_str, e)
                            ))?,
                        None => return Err(::agent_chain::_core::error::Error::ValidationError(
                            format!("Missing required parameter '{}'", #name_str)
                        )),
                    };
                }
            }
        })
        .collect();

    // --- Function call generation ---
    let fn_call_async = if is_async {
        quote! {
            async fn __tool_fn(#(#param_names: #param_types),*) #fn_return_type
                #fn_body
            __tool_fn(#(#param_names),*).await
        }
    } else {
        quote! {
            fn __tool_fn(#(#param_names: #param_types),*) #fn_return_type
                #fn_body
            __tool_fn(#(#param_names),*)
        }
    };

    let fn_call_sync = quote! {
        fn __tool_fn(#(#param_names: #param_types),*) #fn_return_type
            #fn_body
        __tool_fn(#(#param_names),*)
    };

    let extract_args = quote! {
        let args = match input {
            ::agent_chain::_core::tools::ToolInput::Dict(d) => d,
            ::agent_chain::_core::tools::ToolInput::String(s) => {
                match serde_json::from_str::<serde_json::Value>(&s) {
                    Ok(serde_json::Value::Object(obj)) => obj.into_iter().collect(),
                    _ => {
                        let mut map = HashMap::new();
                        map.insert("input".to_string(), serde_json::Value::String(s));
                        map
                    }
                }
            }
            ::agent_chain::_core::tools::ToolInput::ToolCall(tc) => {
                tc.args.as_object()
                    .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default()
            }
        };
    };

    // --- Result handling differs based on whether the function returns Result<T> or T ---

    // For tool_arun: serialize the successful value into ToolOutput
    let arun_result_handling = if returns_result {
        quote! {
            let result: #actual_return_type = { #fn_call_async };
            let value = result?;
            let result_str = serde_json::to_string(&value)
                .unwrap_or_else(|_| format!("{:?}", value));
            Ok(::agent_chain::_core::tools::ToolOutput::String(result_str))
        }
    } else {
        quote! {
            let result: #actual_return_type = { #fn_call_async };
            let result_str = serde_json::to_string(&result)
                .unwrap_or_else(|_| format!("{:?}", result));
            Ok(::agent_chain::_core::tools::ToolOutput::String(result_str))
        }
    };

    // For tool_run (sync)
    let tool_run_body = if is_async {
        quote! {
            Err(::agent_chain::_core::error::Error::ToolInvocation(
                "This is an async tool. Use async invoke instead.".to_string()
            ))
        }
    } else if returns_result {
        quote! {
            #extract_args
            #(#result_param_extractions)*
            let result: #actual_return_type = { #fn_call_sync };
            let value = result?;
            let result_str = serde_json::to_string(&value)
                .unwrap_or_else(|_| format!("{:?}", value));
            Ok(::agent_chain::_core::tools::ToolOutput::String(result_str))
        }
    } else {
        quote! {
            #extract_args
            #(#result_param_extractions)*
            let result: #actual_return_type = { #fn_call_sync };
            let result_str = serde_json::to_string(&result)
                .unwrap_or_else(|_| format!("{:?}", result));
            Ok(::agent_chain::_core::tools::ToolOutput::String(result_str))
        }
    };

    // For invoke_tool_call: return error ToolMessage on Err
    let invoke_result_handling = if returns_result {
        quote! {
            let result: #actual_return_type = { #fn_call_async };
            match result {
                Ok(value) => {
                    let result_str = serde_json::to_string(&value)
                        .unwrap_or_else(|_| format!("{:?}", value));
                    ::agent_chain::_core::messages::ToolMessage::builder()
                        .content(result_str)
                        .tool_call_id(tool_call.id.unwrap_or_default())
                        .build()
                        .into()
                }
                Err(e) => {
                    ::agent_chain::_core::messages::ToolMessage::builder()
                        .content(e.to_string())
                        .tool_call_id(tool_call.id.unwrap_or_default())
                        .status(::agent_chain::_core::messages::ToolStatus::Error)
                        .build()
                        .into()
                }
            }
        }
    } else {
        quote! {
            let result: #actual_return_type = { #fn_call_async };
            let result_str = serde_json::to_string(&result)
                .unwrap_or_else(|_| format!("{:?}", result));
            ::agent_chain::_core::messages::ToolMessage::builder()
                .content(result_str)
                .tool_call_id(tool_call.id.unwrap_or_default())
                .build()
                .into()
        }
    };

    let expanded = quote! {
        #fn_vis mod #mod_name {
            use super::*;
            use std::collections::HashMap;

            pub struct #struct_name {
                args_schema: ::agent_chain::_core::tools::ArgsSchema,
            }

            impl #struct_name {
                pub fn new() -> Self {
                    let properties: HashMap<String, serde_json::Value> = [
                        #(#schema_properties),*
                    ].into_iter().collect();

                    let required: Vec<String> = vec![
                        #(#required_params),*
                    ];

                    let schema = serde_json::json!({
                        "type": "object",
                        "properties": properties,
                        "required": required
                    });

                    Self {
                        args_schema: ::agent_chain::_core::tools::ArgsSchema::JsonSchema(schema),
                    }
                }
            }

            impl Default for #struct_name {
                fn default() -> Self {
                    Self::new()
                }
            }

            impl std::fmt::Debug for #struct_name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.debug_struct(#tool_name).finish()
                }
            }

            #[::agent_chain::_core::async_trait]
            impl ::agent_chain::_core::tools::BaseTool for #struct_name {
                fn name(&self) -> &str {
                    #tool_name
                }

                fn description(&self) -> &str {
                    #tool_description
                }

                fn args_schema(&self) -> Option<&::agent_chain::_core::tools::ArgsSchema> {
                    Some(&self.args_schema)
                }

                fn return_direct(&self) -> bool {
                    #return_direct
                }

                fn response_format(&self) -> ::agent_chain::_core::tools::ResponseFormat {
                    #response_format_expr
                }

                fn tool_run(
                    &self,
                    input: ::agent_chain::_core::tools::ToolInput,
                    _run_manager: Option<&::agent_chain::_core::callbacks::manager::CallbackManagerForToolRun>,
                    _config: &::agent_chain::_core::runnables::RunnableConfig,
                ) -> ::agent_chain::_core::error::Result<::agent_chain::_core::tools::ToolOutput> {
                    #tool_run_body
                }

                async fn tool_arun(
                    &self,
                    input: ::agent_chain::_core::tools::ToolInput,
                    _run_manager: Option<&::agent_chain::_core::callbacks::manager::CallbackManagerForToolRun>,
                    _config: &::agent_chain::_core::runnables::RunnableConfig,
                ) -> ::agent_chain::_core::error::Result<::agent_chain::_core::tools::ToolOutput> {
                    #extract_args
                    #(#result_param_extractions)*
                    #arun_result_handling
                }

                fn definition(&self) -> ::agent_chain::_core::tools::ToolDefinition {
                    ::agent_chain::_core::tools::ToolDefinition {
                        name: #tool_name.to_string(),
                        description: #tool_description.to_string(),
                        parameters: self.args_schema.to_json_schema(),
                    }
                }

                async fn invoke_tool_call(
                    &self,
                    tool_call: ::agent_chain::_core::messages::ToolCall,
                ) -> ::agent_chain::_core::messages::BaseMessage {
                    let args = &tool_call.args;
                    let args: HashMap<String, serde_json::Value> = args.as_object()
                        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                        .unwrap_or_default();

                    #(#invoke_param_extractions)*
                    #invoke_result_handling
                }
            }

            /// Create a new instance of this tool.
            pub fn tool() -> #struct_name {
                #struct_name::new()
            }
        }
    };

    TokenStream::from(expanded)
}

struct ParamInfo {
    name: syn::Ident,
    ty: Type,
    is_option: bool,
    inner_type: Option<Type>,
}

#[derive(Default)]
struct ToolAttrs {
    name: Option<String>,
    description: Option<String>,
    return_direct: bool,
    response_format: Option<String>,
}

fn parse_tool_attrs(attr: TokenStream) -> ToolAttrs {
    let mut result = ToolAttrs::default();

    if attr.is_empty() {
        return result;
    }

    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let parsed = match syn::parse::Parser::parse(parser, attr) {
        Ok(p) => p,
        Err(_) => return result,
    };

    for meta in parsed {
        match &meta {
            Meta::NameValue(MetaNameValue {
                path,
                value:
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(lit_str),
                        ..
                    }),
                ..
            }) => {
                if path.is_ident("name") {
                    result.name = Some(lit_str.value());
                } else if path.is_ident("description") {
                    result.description = Some(lit_str.value());
                } else if path.is_ident("response_format") {
                    result.response_format = Some(lit_str.value());
                }
            }
            Meta::NameValue(MetaNameValue {
                path,
                value:
                    Expr::Lit(ExprLit {
                        lit: Lit::Bool(lit_bool),
                        ..
                    }),
                ..
            }) => {
                if path.is_ident("return_direct") {
                    result.return_direct = lit_bool.value();
                }
            }
            _ => {}
        }
    }

    result
}

fn extract_doc_comment(func: &ItemFn) -> Option<String> {
    let doc_lines: Vec<String> = func
        .attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc")
                && let Meta::NameValue(nv) = &attr.meta
                && let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = &nv.value
            {
                return Some(s.value());
            }
            None
        })
        .collect();

    if doc_lines.is_empty() {
        return None;
    }

    let combined = doc_lines
        .iter()
        .map(|line| line.strip_prefix(' ').unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n");

    let trimmed = combined.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Check if the return type is `Result<T>` or `Result<T, E>`.
fn is_result_type(ret: &ReturnType) -> bool {
    if let ReturnType::Type(_, ty) = ret {
        return type_is_result(ty);
    }
    false
}

fn type_is_result(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "Result";
    }
    false
}

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "Option";
    }
    false
}

fn extract_option_inner(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "Option"
        && let PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner.clone());
    }
    None
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

fn get_json_schema(ty: &Type) -> proc_macro2::TokenStream {
    let type_str = quote!(#ty).to_string().replace(' ', "");

    if type_str.starts_with("Vec<") {
        if let Some(inner) = extract_generic_inner(ty) {
            let inner_schema = get_json_schema(&inner);
            return quote! {
                serde_json::json!({ "type": "array", "items": #inner_schema })
            };
        }
        return quote! { serde_json::json!({ "type": "array" }) };
    }

    if type_str.starts_with("HashMap<") {
        if let Some(value_type) = extract_hashmap_value_type(ty) {
            let value_schema = get_json_schema(&value_type);
            return quote! {
                serde_json::json!({ "type": "object", "additionalProperties": #value_schema })
            };
        }
        return quote! { serde_json::json!({ "type": "object" }) };
    }

    if type_str.starts_with("Option<")
        && let Some(inner) = extract_option_inner(ty)
    {
        return get_json_schema(&inner);
    }

    let json_type = match type_str.as_str() {
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128"
        | "usize" => "integer",
        "f32" | "f64" => "number",
        "bool" => "boolean",
        "String" | "&str" | "&'staticstr" => "string",
        _ => "string",
    };

    quote! { serde_json::json!({ "type": #json_type }) }
}

fn extract_generic_inner(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && let PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner.clone());
    }
    None
}

fn extract_hashmap_value_type(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && let PathArguments::AngleBracketed(args) = &segment.arguments
    {
        let type_args: Vec<_> = args
            .args
            .iter()
            .filter_map(|a| {
                if let GenericArgument::Type(t) = a {
                    Some(t.clone())
                } else {
                    None
                }
            })
            .collect();
        if type_args.len() == 2 {
            return Some(type_args[1].clone());
        }
    }
    None
}
