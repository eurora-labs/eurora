//! Procedural macros for agent-chain.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn, Pat, ReturnType, Type, parse_macro_input};

/// Marks a function as a tool that can be used by an LLM.
///
/// This macro generates a struct that implements the `Tool` trait,
/// allowing the function to be invoked by an AI model.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::tools::tool;
///
/// #[tool]
/// fn multiply(a: i64, b: i64) -> i64 {
///     a * b
/// }
///
/// // Creates a tool instance
/// let tool = multiply::tool();
/// ```
#[proc_macro_attribute]
pub fn tool(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_name_str = fn_name.to_string();
    let mod_name = format_ident!("{}", fn_name);
    let struct_name = format_ident!("{}Tool", to_pascal_case(&fn_name_str));

    let fn_body = &input.block;
    let fn_vis = &input.vis;
    let _fn_asyncness = &input.sig.asyncness;
    let fn_return_type = &input.sig.output;

    // Extract parameters
    let params: Vec<_> = input
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg
                && let Pat::Ident(pat_ident) = pat_type.pat.as_ref()
            {
                let param_name = &pat_ident.ident;
                let param_type = &pat_type.ty;
                return Some((param_name.clone(), param_type.clone()));
            }
            None
        })
        .collect();

    let param_names: Vec<_> = params.iter().map(|(name, _)| name.clone()).collect();
    let param_types: Vec<_> = params.iter().map(|(_, ty)| ty.clone()).collect();
    let param_names_str: Vec<_> = params.iter().map(|(name, _)| name.to_string()).collect();

    // Generate JSON schema properties for parameters
    let schema_properties: Vec<_> = params
        .iter()
        .map(|(name, ty)| {
            let name_str = name.to_string();
            let type_str = get_json_type(ty);
            quote! {
                (#name_str.to_string(), serde_json::json!({ "type": #type_str }))
            }
        })
        .collect();

    // Get the return type without the `-> `
    let actual_return_type = match fn_return_type {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, ty) => quote! { #ty },
    };

    let expanded = quote! {
        #fn_vis mod #mod_name {
            use super::*;
            use std::collections::HashMap;
            use serde_json;

            /// The tool implementation struct
            pub struct #struct_name;

            impl #struct_name {
                /// Create a new instance of this tool
                pub fn new() -> Self {
                    Self
                }
            }

            impl Default for #struct_name {
                fn default() -> Self {
                    Self::new()
                }
            }

            impl std::fmt::Debug for #struct_name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.debug_struct(stringify!(#struct_name)).finish()
                }
            }

            #[::agent_chain::_core::async_trait]
            impl ::agent_chain::_core::tools::BaseTool for #struct_name {
                fn name(&self) -> &str {
                    #fn_name_str
                }

                fn description(&self) -> &str {
                    concat!("Tool: ", #fn_name_str)
                }

                fn args_schema(&self) -> Option<&::agent_chain::_core::tools::ArgsSchema> {
                    None
                }

                fn tool_run(
                    &self,
                    input: ::agent_chain::_core::tools::ToolInput,
                    _run_manager: Option<&::agent_chain::_core::callbacks::manager::CallbackManagerForToolRun>,
                    _config: &::agent_chain::_core::runnables::RunnableConfig,
                ) -> ::agent_chain::_core::error::Result<::agent_chain::_core::tools::ToolOutput> {
                    // For generated tools, we always use async version
                    Err(::agent_chain::_core::error::Error::NotImplemented("Use async invoke instead".into()))
                }

                fn definition(&self) -> ::agent_chain::_core::tools::ToolDefinition {
                    let properties: HashMap<String, serde_json::Value> = [
                        #(#schema_properties),*
                    ].into_iter().collect();

                    let required: Vec<String> = vec![
                        #(#param_names_str.to_string()),*
                    ];

                    ::agent_chain::_core::tools::ToolDefinition {
                        name: #fn_name_str.to_string(),
                        description: concat!("Tool: ", #fn_name_str).to_string(),
                        parameters: serde_json::json!({
                            "type": "object",
                            "properties": properties,
                            "required": required
                        }),
                    }
                }

                async fn invoke_tool_call(&self, tool_call: ::agent_chain::_core::messages::ToolCall) -> ::agent_chain::_core::messages::BaseMessage {
                    let args = &tool_call.args;

                    #(
                        let #param_names: #param_types = serde_json::from_value(
                            args.get(#param_names_str).cloned().unwrap_or(serde_json::Value::Null)
                        ).expect(&format!("Failed to parse parameter '{}'", #param_names_str));
                    )*

                    let result: #actual_return_type = { #fn_body };

                    let result_str = serde_json::to_string(&result).unwrap_or_else(|_| format!("{:?}", result));

                    ::agent_chain::_core::messages::ToolMessage::builder()
                        .content(result_str)
                        .tool_call_id(tool_call.id.unwrap_or_default())
                        .build()
                        .into()
                }
            }

            /// Create a new instance of this tool
            pub fn tool() -> #struct_name {
                #struct_name::new()
            }
        }
    };

    TokenStream::from(expanded)
}

/// Convert snake_case to PascalCase
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

/// Get the JSON schema type for a Rust type
fn get_json_type(ty: &Type) -> &'static str {
    let type_str = quote!(#ty).to_string();
    match type_str.as_str() {
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128"
        | "usize" => "integer",
        "f32" | "f64" => "number",
        "bool" => "boolean",
        "String" | "& str" | "& 'static str" => "string",
        _ => "string", // Default to string for unknown types
    }
}
