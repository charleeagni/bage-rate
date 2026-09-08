use seaography::async_graphql::parser::{
    parse_schema,
    types::{TypeKind, TypeSystemDefinition},
};

use crate::error::ComposeError;

/// Seaography scaffolding present in every built schema regardless of which
/// Modules registered, so owned by no Module. Derived from the pinned
/// seaography 2.0.0-rc.9 sources: the pagination plumbing (`src/inputs`,
/// `src/outputs`), the ordering enum (`src/enumerations/order_by_enum.rs`),
/// the fixed filter primitives (`src/builder_context/filter_types_map.rs`),
/// and the unconditionally registered `Json` scalar.
const SCAFFOLDING_TYPES: &[&str] = &[
    "CursorInput",
    "OffsetInput",
    "PageInput",
    "PaginationInput",
    "PageInfo",
    "PaginationInfo",
    "OrderByEnum",
    "BooleanFilterInput",
    "FloatFilterInput",
    "IdentityFilterInput",
    "IntegerFilterInput",
    "JsonFilterInput",
    "StringFilterInput",
    "TextFilterInput",
    "BooleanArrayFilterInput",
    "FloatArrayFilterInput",
    "IdArrayFilterInput",
    "IntegerArrayFilterInput",
    "StringArrayFilterInput",
    "TextArrayFilterInput",
    "Json",
];

/// Root objects are shared plumbing; their fields are what Modules own.
const ROOT_TYPES: &[&str] = &["Query", "Mutation", "Subscription"];

/// Walk the schema's SDL and require every non-scaffolding type name and
/// every root field to carry the Module's name as a prefix.
pub fn check(module: &'static str, sdl: &str) -> Result<(), ComposeError> {
    let document =
        parse_schema(sdl).map_err(|error| ComposeError::Schema(error.to_string().into()))?;

    for definition in document.definitions {
        let TypeSystemDefinition::Type(type_definition) = definition else {
            continue;
        };
        let type_definition = type_definition.node;
        let name = type_definition.name.node.as_str();

        if ROOT_TYPES.contains(&name) {
            let TypeKind::Object(object) = type_definition.kind else {
                continue;
            };
            for field in object.fields {
                let field_name = field.node.name.node.as_str();
                if !owned_by(module, field_name) {
                    return Err(ComposeError::UnownedRootField {
                        module,
                        root: name.to_owned(),
                        field: field_name.to_owned(),
                    });
                }
            }
        } else if !SCAFFOLDING_TYPES.contains(&name) && !owned_by(module, name) {
            return Err(ComposeError::UnownedType {
                module,
                type_name: name.to_owned(),
            });
        }
    }
    Ok(())
}

// Case conventions differ per position (PascalCase types, camelCase root
// fields, snake_case module names), so ownership compares case-insensitively
// with underscores removed.
fn owned_by(module: &str, name: &str) -> bool {
    normalized(name).starts_with(&normalized(module))
}

fn normalized(name: &str) -> String {
    name.chars()
        .filter(|character| *character != '_')
        .flat_map(char::to_lowercase)
        .collect()
}
