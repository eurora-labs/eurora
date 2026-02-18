//! Procedural macros for agent-graph.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn, Pat, ReturnType, parse_macro_input};

/// Marks an async function as a task.
///
/// Tasks are the building blocks of LangGraph workflows. They represent
/// individual units of work that can be executed and whose results can
/// be awaited. When called, tasks return a `TaskFuture` that can be:
/// - Awaited with `.await`
/// - Blocked on with `.result()`
///
/// This matches Python's langgraph functional API where `@task` decorated
/// functions return a `SyncAsyncFuture`.
///
/// # Example
///
/// ```ignore
/// use agent_graph::func::task;
///
/// #[task]
/// async fn process_data(input: String) -> String {
///     input.to_uppercase()
/// }
///
/// // Inside an entrypoint:
/// let future = process_data("hello".to_string());  // Returns TaskFuture
/// let result = future.await?;  // "HELLO"
/// // Or: let result = future.result()?;  // Blocking version
/// ```
#[proc_macro_attribute]
pub fn task(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let fn_block = &input.block;
    let _fn_inputs = &input.sig.inputs;
    let fn_return_type = &input.sig.output;

    let actual_return_type = match fn_return_type {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, ty) => quote! { #ty },
    };

    let params: Vec<_> = input
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg
                && let Pat::Ident(pat_ident) = pat_type.pat.as_ref()
            {
                let name = &pat_ident.ident;
                let ty = &pat_type.ty;
                return Some((name.clone(), ty.clone()));
            }
            None
        })
        .collect();

    let param_names: Vec<_> = params.iter().map(|(name, _)| name.clone()).collect();
    let param_types: Vec<_> = params.iter().map(|(_, ty)| ty.clone()).collect();

    let expanded = quote! {
        #fn_vis fn #fn_name(#(#param_names: #param_types),*) -> agent_graph::func::TaskFuture<#actual_return_type>
        where
            #actual_return_type: Send + 'static,
        {
            let (sender, receiver) = tokio::sync::oneshot::channel();

            tokio::spawn(async move {
                let result: #actual_return_type = {
                    #fn_block
                };
                let _ = sender.send(result);
            });

            agent_graph::func::TaskFuture::new(receiver)
        }
    };

    TokenStream::from(expanded)
}

/// Marks an async function as an entrypoint for a workflow.
///
/// The entrypoint decorator creates a workflow that can be streamed
/// or invoked. It generates a module with a `stream` function that
/// returns an async stream of results.
///
/// # Example
///
/// ```ignore
/// use agent_graph::func::entrypoint;
/// use agent_graph::stream::StreamMode;
///
/// #[entrypoint]
/// async fn my_workflow(input: String) -> String {
///     input.to_uppercase()
/// }
///
/// // Usage:
/// let stream = my_workflow::stream(input, StreamMode::Updates, ());
/// ```
#[proc_macro_attribute]
pub fn entrypoint(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_name_str = fn_name.to_string();
    let mod_name = format_ident!("{}", fn_name);
    let fn_vis = &input.vis;
    let fn_block = &input.block;
    let fn_inputs = &input.sig.inputs;
    let fn_return_type = &input.sig.output;

    let actual_return_type = match fn_return_type {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, ty) => quote! { #ty },
    };

    let first_param = input.sig.inputs.first();
    let (input_name, input_type) = if let Some(FnArg::Typed(pat_type)) = first_param {
        if let Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
            let name = &pat_ident.ident;
            let ty = &pat_type.ty;
            (quote! { #name }, quote! { #ty })
        } else {
            (quote! { input }, quote! { () })
        }
    } else {
        (quote! { input }, quote! { () })
    };

    let additional_params: Vec<_> = input
        .sig
        .inputs
        .iter()
        .skip(1)
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg
                && let Pat::Ident(pat_ident) = pat_type.pat.as_ref()
            {
                let name = &pat_ident.ident;
                let ty = &pat_type.ty;
                return Some((name.clone(), ty.clone()));
            }
            None
        })
        .collect();

    let additional_param_names: Vec<_> = additional_params
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    let additional_param_types: Vec<_> =
        additional_params.iter().map(|(_, ty)| ty.clone()).collect();

    let context_type = if additional_params.is_empty() {
        quote! { () }
    } else if additional_params.len() == 1 {
        let ty = &additional_param_types[0];
        quote! { #ty }
    } else {
        quote! { (#(#additional_param_types),*) }
    };

    let context_extraction = if additional_params.is_empty() {
        quote! {}
    } else if additional_params.len() == 1 {
        let name = &additional_param_names[0];
        quote! { let #name = context; }
    } else {
        let extractions: Vec<_> = additional_param_names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let idx = syn::Index::from(i);
                quote! { let #name = context.#idx.clone(); }
            })
            .collect();
        quote! { #(#extractions)* }
    };

    let expanded = quote! {
        #fn_vis mod #mod_name {
            use super::*;
            use futures::stream::{self, Stream, StreamExt};
            use std::pin::Pin;

            /// Stream the workflow execution.
            pub fn stream(
                #input_name: #input_type,
                _mode: agent_graph::stream::StreamMode,
                context: #context_type,
            ) -> Pin<Box<dyn Stream<Item = agent_graph::stream::StreamChunk<#actual_return_type>> + Send>> {
                Box::pin(stream::once(async move {
                    #context_extraction

                    let result: #actual_return_type = {
                        let mut #input_name = #input_name;
                        #fn_block
                    };

                    agent_graph::stream::StreamChunk::new(#fn_name_str.to_string(), result)
                }))
            }

            /// Invoke the workflow and return the final result.
            pub async fn invoke(
                #fn_inputs
            ) -> #actual_return_type {
                #fn_block
            }
        }
    };

    TokenStream::from(expanded)
}
