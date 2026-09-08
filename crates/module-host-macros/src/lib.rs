//! The two attribute macros a Module uses to declare custom operations.
//!
//! Both exist for one reason: everything they expand to is written with
//! absolute `::module_host` and `::seaography` paths, so a Module's own
//! source never has to name a layer beneath the seam. Seaography's stock
//! `#[CustomFields]` and `#[derive(CustomOutputType)]` expand to bare
//! `seaography::` and `async_graphql::` paths, which only resolve when the
//! author imports them — which is exactly what
//! `scripts/check-module-imports.mjs` rejects.
//!
//! They also fix two conventions the stock macros leave to the author: root
//! field names, argument names, and output field names are camel-cased to
//! match the generated Seaography contract, and the resolver's context
//! argument is a narrowed [`ModuleCtx`](../module_host/struct.ModuleCtx.html)
//! rather than the full `async_graphql::Context`.

mod custom_fields;
mod naming;
mod output;

use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemImpl, ItemStruct};

/// Declare an impl block's async functions as custom root fields.
///
/// Every function becomes one field: its name is camel-cased, its first
/// argument must be `&ModuleCtx<'_>`, its remaining arguments become
/// camel-cased GraphQL arguments, and it must return `Result<T>` where `T`
/// is a scalar or an `#[output]` struct. Which root the fields land on —
/// Query, Mutation, or the escape hatch — is decided where the type is
/// registered in `module_def()`, not here.
#[proc_macro_attribute]
pub fn custom_fields(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);
    custom_fields::expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Declare a struct as a custom GraphQL output object.
///
/// Field names are camel-cased; field types must be scalars. The struct's
/// own name is the GraphQL type name, so it carries the Module's prefix or
/// composition rejects it.
#[proc_macro_attribute]
pub fn output(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    output::expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
