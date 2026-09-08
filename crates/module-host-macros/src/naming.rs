use heck::ToLowerCamelCase;
use syn::Ident;

/// The GraphQL name for a Rust identifier. Generated Seaography root fields,
/// arguments, and columns are camel-cased, so custom ones are too — a Module
/// whose SDL mixed both conventions would read as two contracts.
pub fn graphql_name(ident: &Ident) -> String {
    ident.to_string().to_lower_camel_case()
}
