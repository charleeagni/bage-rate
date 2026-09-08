//! Restricted, model-shaped mutations with framework-owned persistence.
//!
//! Generated mutations are the default. This builder is for an existing
//! flattened GraphQL contract that must bind one concrete identity or CAS
//! token while still using Seaography guards, lifecycle hooks, SeaORM row
//! behavior, and post-commit signals. Application code prepares a model write;
//! this module owns the resolver, transaction, persistence, and watch timing.

use sea_orm::{
    entity::prelude::async_trait, ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection,
    DatabaseTransaction, EntityTrait, IntoActiveModel, Iterable, ModelTrait, PrimaryKeyToColumn,
    QueryFilter, TransactionTrait,
};
use seaography::{
    async_graphql::{
        dynamic::{Field, FieldFuture, FieldValue, InputValue, ResolverContext, TypeRef},
        Error, Result,
    },
    guard_error, Builder, DatabaseContext, EntityObjectBuilder, GuardAction, OperationType,
    UserContext,
};

/// A lock or other owned capability held from preparation through commit and
/// post-commit signal dispatch.
pub trait WritePermit: Send {
    fn committed(self: Box<Self>) {}
}

impl WritePermit for () {}

/// The single model write prepared by a restricted mutation hook.
pub enum ModelWrite<A, M> {
    Insert(A),
    Update(A),
    Delete { model: M, active_model: A },
    Noop,
}

/// Prepared write plus the permit that serializes its read/compare/write
/// window. Dropping the permit before `committed` is the application's failure
/// cleanup seam.
pub struct PreparedModelWrite<A, M> {
    pub write: ModelWrite<A, M>,
    pub permit: Box<dyn WritePermit>,
}

impl<A, M> PreparedModelWrite<A, M> {
    pub fn new(write: ModelWrite<A, M>, permit: impl WritePermit + 'static) -> Self {
        Self {
            write,
            permit: Box::new(permit),
        }
    }
}

/// Domain rule for one restricted model mutation.
///
/// `prepare` runs inside the mutation transaction. It must acquire any lock
/// before reading rows through `transaction`, then return the proposed write.
#[async_trait::async_trait]
pub trait RestrictedModelMutation<T, A>: Send + Sync
where
    T: EntityTrait,
    A: ActiveModelTrait<Entity = T>,
{
    async fn prepare(
        &self,
        ctx: &ResolverContext<'_>,
        transaction: &DatabaseTransaction,
    ) -> Result<PreparedModelWrite<A, T::Model>>;
}

/// Schema declaration for one existing restricted mutation field.
pub struct RestrictedMutationField {
    pub name: String,
    pub action: OperationType,
    pub nullable: bool,
    pub arguments: Vec<InputValue>,
    pub hook_owns_authorization: bool,
    pub returns_boolean: bool,
}

impl RestrictedMutationField {
    pub fn new(name: impl Into<String>, action: OperationType) -> Self {
        Self {
            name: name.into(),
            action,
            nullable: false,
            arguments: Vec::new(),
            hook_owns_authorization: false,
            returns_boolean: false,
        }
    }

    pub fn nullable(mut self) -> Self {
        self.nullable = true;
        self
    }

    pub fn argument(mut self, argument: InputValue) -> Self {
        self.arguments.push(argument);
        self
    }

    /// Skip the generated entity guard when the restricted hook performs the
    /// concrete identity and caller authorization itself.
    pub fn hook_owns_authorization(mut self) -> Self {
        self.hook_owns_authorization = true;
        self
    }

    /// Return whether the prepared model write affected a row instead of
    /// returning the row itself. This preserves existing success-flag delete
    /// contracts without moving persistence back into an authored resolver.
    pub fn returns_boolean(mut self) -> Self {
        self.returns_boolean = true;
        self
    }
}

/// Register one restricted model mutation without an application-authored
/// GraphQL resolver.
pub fn register_restricted_model_mutation<T, A, H>(
    builder: &mut Builder,
    declaration: RestrictedMutationField,
    hook: H,
) where
    T: EntityTrait,
    T::Model: Sync + IntoActiveModel<A>,
    A: ActiveModelTrait<Entity = T> + sea_orm::ActiveModelBehavior + Send + 'static,
    H: RestrictedModelMutation<T, A> + 'static,
{
    let context = builder.context;
    let object_name = EntityObjectBuilder { context }.type_name::<T>();
    let output_type = if declaration.returns_boolean {
        TypeRef::named_nn(TypeRef::BOOLEAN)
    } else if declaration.nullable {
        TypeRef::named(&object_name)
    } else {
        TypeRef::named_nn(&object_name)
    };
    let action = declaration.action;
    let hook_owns_authorization = declaration.hook_owns_authorization;
    let returns_boolean = declaration.returns_boolean;
    let hook = std::sync::Arc::new(hook);
    let hooks = &context.hooks;
    let mut field = Field::new(declaration.name, output_type, move |ctx| {
        let object_name = object_name.clone();
        let hook = hook.clone();
        FieldFuture::new(async move {
            if !hook_owns_authorization {
                if let GuardAction::Block(reason) = hooks.entity_guard(&ctx, &object_name, action) {
                    return Err(guard_error(reason, "Entity guard triggered."));
                }
            }

            let database = ctx
                .data::<DatabaseConnection>()?
                .restricted(ctx.data_opt::<UserContext>())?;
            let transaction = database.begin().await?;
            let PreparedModelWrite { write, permit } = hook.prepare(&ctx, &transaction).await?;

            let result = match write {
                ModelWrite::Noop => None,
                ModelWrite::Insert(mut active_model) => {
                    run_save_hook(hooks, &ctx, &object_name, action, &mut active_model)?;
                    let model = active_model.insert(&transaction).await?;
                    enforce_create_filter::<T>(hooks, &ctx, &object_name, &transaction, &model)
                        .await?;
                    Some(model)
                }
                ModelWrite::Update(mut active_model) => {
                    run_save_hook(hooks, &ctx, &object_name, action, &mut active_model)?;
                    Some(active_model.update(&transaction).await?)
                }
                ModelWrite::Delete {
                    model,
                    mut active_model,
                } => {
                    run_save_hook(hooks, &ctx, &object_name, action, &mut active_model)?;
                    active_model.delete(&transaction).await?;
                    Some(model)
                }
            };

            transaction.commit().await?;
            if result.is_some() {
                hooks.entity_watch(&ctx, &object_name, action).await;
            }
            permit.committed();

            if returns_boolean {
                Ok(Some(FieldValue::value(result.is_some())))
            } else {
                Ok(result.map(FieldValue::owned_any))
            }
        })
    });
    for argument in declaration.arguments {
        field = field.argument(argument);
    }
    builder.mutations.push(field);
}

pub(crate) async fn enforce_create_filter<T: EntityTrait>(
    hooks: &seaography::LifecycleHooks,
    ctx: &ResolverContext<'_>,
    entity: &str,
    transaction: &DatabaseTransaction,
    model: &T::Model,
) -> Result<()> {
    let Some(filter) = hooks.entity_filter(ctx, entity, OperationType::Create) else {
        return Ok(());
    };
    let mut primary_key = Condition::all();
    for key in <T::PrimaryKey as Iterable>::iter() {
        let column = key.into_column();
        primary_key = primary_key.add(column.eq(model.get(column)));
    }
    let in_scope = T::find()
        .filter(primary_key)
        .filter(filter)
        .one(transaction)
        .await?
        .is_some();
    if in_scope {
        Ok(())
    } else {
        Err(guard_error(
            Some("created row is outside the caller's scope".to_owned()),
            "Entity filter rejected the create.",
        ))
    }
}

pub(crate) fn run_save_hook<A: 'static>(
    hooks: &seaography::LifecycleHooks,
    ctx: &ResolverContext<'_>,
    entity: &str,
    action: OperationType,
    active_model: &mut A,
) -> Result<()> {
    match hooks.before_active_model_save(ctx, entity, action, active_model) {
        GuardAction::Allow => Ok(()),
        GuardAction::Block(reason) => Err(guard_error(reason, "Save hook blocked the write.")),
    }
}

pub fn string_argument(name: impl Into<String>) -> InputValue {
    InputValue::new(name, TypeRef::named_nn(TypeRef::STRING))
}

pub fn mutation_error(message: impl Into<String>) -> Error {
    Error::new(message)
}
