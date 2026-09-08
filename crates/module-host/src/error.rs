use seaography::async_graphql::dynamic::SchemaError;

/// The error a resolver, hook, or transaction returns. It is the GraphQL
/// error type, so `?` works over `DbErr` and anything else that displays.
pub type Error = seaography::async_graphql::Error;

/// What a resolver, hook, or transaction body returns.
pub type Result<T> = std::result::Result<T, Error>;

/// Why a set of Modules failed to compose into a schema.
#[derive(Debug)]
pub enum ComposeError {
    /// The underlying dynamic schema failed to build.
    Schema(SchemaError),
    /// A Module registered a type whose name lacks the Module's prefix.
    UnownedType {
        module: &'static str,
        type_name: String,
    },
    /// A Module registered a Query, Mutation, or Subscription root field
    /// whose name lacks the Module's prefix.
    UnownedRootField {
        module: &'static str,
        root: String,
        field: String,
    },
}

impl std::fmt::Display for ComposeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schema(error) => write!(formatter, "schema failed to build: {error}"),
            Self::UnownedType { module, type_name } => write!(
                formatter,
                "module \"{module}\" registered type \"{type_name}\", \
                 which does not carry the module's name as its prefix"
            ),
            Self::UnownedRootField {
                module,
                root,
                field,
            } => write!(
                formatter,
                "module \"{module}\" registered {root} field \"{field}\", \
                 which does not carry the module's name as its prefix"
            ),
        }
    }
}

impl std::error::Error for ComposeError {}

impl From<SchemaError> for ComposeError {
    fn from(error: SchemaError) -> Self {
        Self::Schema(error)
    }
}
