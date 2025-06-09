// Copyright 2025 RisingWave Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Procedural macros for await-tree instrumentation.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Instruments an async function with await-tree spans.
///
/// This attribute macro transforms an async function to automatically create
/// an await-tree span and instrument the function's execution.
///
/// # Usage
///
/// ```rust,ignore
/// #[await_tree::instrument("span_name({})", arg1)]
/// async fn foo(arg1: i32, arg2: String) {
///     // function body
/// }
/// ```
///
/// The above will be expanded to:
///
/// ```rust,ignore
/// async fn foo(arg1: i32, arg2: String) {
///     let span = await_tree::span!("span_name({})", arg1);
///     let fut = async move {
///         // original function body
///     };
///     fut.instrument_await(span).await
/// }
/// ```
///
/// # Arguments
///
/// The macro accepts format arguments similar to `format!` or `println!`:
/// - The first argument is the format string
/// - Subsequent arguments are the values to be formatted
///
/// The format arguments are passed directly to the `await_tree::span!` macro
/// without any parsing or modification.
#[proc_macro_attribute]
pub fn instrument(args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);

    // Validate that this is an async function
    if input_fn.sig.asyncness.is_none() {
        return syn::Error::new_spanned(
            &input_fn.sig.fn_token,
            "the `instrument` attribute can only be applied to async functions",
        )
        .to_compile_error()
        .into();
    }

    // Extract the span format arguments
    let span_args = if args.is_empty() {
        // If no arguments provided, use the function name as span
        let fn_name = &input_fn.sig.ident;
        quote! { stringify!(#fn_name) }
    } else {
        // Convert the raw token stream to tokens for the span! macro
        let args_tokens = proc_macro2::TokenStream::from(args);
        quote! { #args_tokens }
    };

    // Extract function components
    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;
    let fn_attrs = &input_fn.attrs;

    // Generate the instrumented function
    let result = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            let __at_span = ::await_tree::span!(#span_args);
            let __at_fut = async move #fn_block;
            ::await_tree::InstrumentAwait::instrument_await(__at_fut, __at_span).await
        }
    };

    result.into()
}
