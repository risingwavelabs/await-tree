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

//! Procedural attributes for await-tree instrumentation.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Ident, ItemFn, Token};

/// Parse the attribute arguments to extract method calls and format args
#[derive(Default)]
struct InstrumentArgs {
    method_calls: Vec<Ident>,
    format_args: Option<proc_macro2::TokenStream>,
    boxed: bool,
}

impl syn::parse::Parse for InstrumentArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut method_calls = Vec::new();
        let mut format_args = None;
        let mut boxed = false;

        // Parse identifiers first (these will become method calls or special keywords)
        while input.peek(Ident) {
            // Look ahead to see if this looks like a method call identifier
            let fork = input.fork();
            let ident: Ident = fork.parse()?;

            // Check if the next token after the identifier is a comma or end
            // If it's something else (like a parenthesis or string), treat as format args
            if fork.peek(Token![,]) || fork.is_empty() {
                // This is a method call identifier or special keyword
                input.parse::<Ident>()?; // consume the identifier

                // Check for special "boxed" keyword
                if ident == "boxed" {
                    boxed = true;
                } else {
                    method_calls.push(ident);
                }

                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
            } else {
                // This looks like the start of format arguments
                break;
            }
        }

        // Parse remaining tokens as format arguments
        if !input.is_empty() {
            let remaining: proc_macro2::TokenStream = input.parse()?;
            format_args = Some(remaining);
        }

        Ok(InstrumentArgs {
            method_calls,
            format_args,
            boxed,
        })
    }
}

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
/// With attributes on the span:
///
/// ```rust,ignore
/// #[await_tree::instrument(long_running, verbose, "span_name({})", arg1)]
/// async fn foo(arg1: i32, arg2: String) {
///     // function body
/// }
/// ```
///
/// With the `boxed` keyword to `Box::pin` the function body before calling `instrument_await`,
/// which can help reducing the stack usage if you encounter stack overflow:
///
/// ```rust,ignore
/// #[await_tree::instrument(boxed, "span_name({})", arg1)]
/// async fn foo(arg1: i32, arg2: String) {
///     // function body
/// }
/// ```
///
/// The above will be expanded to:
///
/// ```rust,ignore
/// async fn foo(arg1: i32, arg2: String) {
///     let span = await_tree::span!("span_name({})", arg1).long_running().verbose();
///     let fut = async move {
///         // original function body
///     };
///     let fut = Box::pin(fut); // if `boxed` is specified
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

    // Parse the arguments
    let parsed_args = if args.is_empty() {
        InstrumentArgs::default()
    } else {
        match syn::parse::<InstrumentArgs>(args) {
            Ok(args) => args,
            Err(e) => return e.to_compile_error().into(),
        }
    };

    // Extract the span format arguments
    let span_args = if let Some(format_args) = parsed_args.format_args {
        quote! { #format_args }
    } else {
        // If no format arguments provided, use the function name as span
        let fn_name = &input_fn.sig.ident;
        quote! { stringify!(#fn_name) }
    };

    // Build span creation with method calls
    let mut span_creation = quote! { ::await_tree::span!(#span_args) };

    // Chain all method calls
    for method_name in parsed_args.method_calls {
        span_creation = quote! { #span_creation.#method_name() };
    }

    // Extract function components
    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;
    let fn_attrs = &input_fn.attrs;

    // Generate the instrumented function
    let boxed =
        (parsed_args.boxed).then(|| quote! { let __at_fut = ::std::boxed::Box::pin(__at_fut); });

    let result = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            use ::await_tree::SpanExt as _;
            let __at_span: ::await_tree::Span = #span_creation;
            let __at_fut = async move #fn_block;
            #boxed
            ::await_tree::InstrumentAwait::instrument_await(__at_fut, __at_span).await
        }
    };

    result.into()
}
