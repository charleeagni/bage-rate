//! Restricted, model-shaped mutations that write a complete row set.
//!
//! [`crate::restricted_model_mutation`] serves a flattened field that writes
//! one row. A transactional reorder, or a delete that reassigns its
//! referents, writes several rows whose invariant only holds over the whole
//! set. This module is the set-shaped sibling: application code selects and
//! locks the rows, and seaolim owns the resolver, the transaction, the
//! per-row save hooks, the set-level [`WriteSetHook`](crate::WriteSetHook)
//! pass, persistence, the entity watch, and the returned list.
//!
//! Cross-row amendment belongs in the write-set hook, not in `prepare`: the
//! hook is the composed, entity-scoped seam an application reads to learn
//! what rule governs a set. `prepare` answers only "which rows does this
//! field write, in what order".
//!
//! The field returns a non-null list of the entity object by default.
//! [`RestrictedMutationField::returns_boolean`] projects any non-empty write
//! set to `true`, which preserves existing success-flag delete contracts.
//! [`RestrictedMutationField::nullable`] does not apply here.

use sea_orm::{
    entity::prelude::async_trait, ActiveModelTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, IntoActiveModel, TransactionTrait,
};
use seaography::{
    async_graphql::{
        dynamic::{Field, FieldFuture, FieldValue, ResolverContext, TypeRef},
        Result,
    },
    guard_error, Builder, DatabaseContext, EntityObjectBuilder, GuardAction, UserContext,
};

use crate::restricted_model_mutation::{
    enforce_create_filter, run_save_hook, RestrictedMutationField, WritePermit,
};
use crate::write_set_hooks::{run_write_set_hooks, WriteSetRow};

/// One member of a restricted write set.
///
/// Update and delete carry the fetched row beside the ActiveModel so the
/// write-set hook sees old and proposed values, exactly as the hooked
/// builders present them.
pub enum ModelSetWrite<A, M> {
    Insert(A),
    Update { model: M, active_model: A },
    Delete { model: M, active_model: A },
}

impl<A, M> ModelSetWrite<A, M> {
    fn active_model_mut(&mut self) -> &mut A {
        match self {
            Self::Insert(active_model)
            | Self::Update { active_model, .. }
            | Self::Delete { active_model, .. } => active_model,
        }
    }

    fn as_write_set_row(&mut self) -> WriteSetRow<'_>
    where
        A: Send + 'static,
        M: Send + Sync + 'static,
    {
        match self {
            Self::Insert(active_model) => WriteSetRow::Create { active_model },
            Self::Update {
                model,
                active_model,
            } => WriteSetRow::Update {
                old_model: model,
                active_model,
            },
            Self::Delete {
                model,
                active_model,
            } => WriteSetRow::Delete {
                old_model: model,
                active_model,
            },
        }
    }
}

/// The rows one restricted set mutation writes, plus the permit serializing
/// its read/compare/write window.
///
/// Row order is the persistence order and the order of the returned list.
pub struct PreparedModelSet<A, M> {
    pub writes: Vec<ModelSetWrite<A, M>>,
    pub permit: Box<dyn WritePermit>,
}

impl<A, M> PreparedModelSet<A, M> {
    pub fn new(writes: Vec<ModelSetWrite<A, M>>, permit: impl WritePermit + 'static) -> Self {
        Self {
            writes,
            permit: Box::new(permit),
        }
    }
}

/// Domain rule that selects the rows one restricted set mutation writes.
///
/// `prepare` runs inside the mutation transaction. It must acquire any lock
/// before reading rows through `transaction`, then return the proposed set.
#[async_trait::async_trait]
pub trait RestrictedModelSetMutation<T, A>: Send + Sync
where
    T: EntityTrait,
    A: ActiveModelTrait<Entity = T>,
{
    async fn prepare(
        &self,
        ctx: &ResolverContext<'_>,
        transaction: &DatabaseTransaction,
    ) -> Result<PreparedModelSet<A, T::Model>>;
}

/// Register one restricted set mutation without an application-authored
/// GraphQL resolver.
pub fn register_restricted_model_set_mutation<T, A, H>(
    builder: &mut Builder,
    declaration: RestrictedMutationField,
    hook: H,
) where
    T: EntityTrait + 'static,
    T::Model: Sync + IntoActiveModel<A>,
    A: ActiveModelTrait<Entity = T> + sea_orm::ActiveModelBehavior + Send + 'static,
    H: RestrictedModelSetMutation<T, A> + 'static,
{
    let context = builder.context;
    let object_name = EntityObjectBuilder { context }.type_name::<T>();
    let output_type = if declaration.returns_boolean {
        TypeRef::named_nn(TypeRef::BOOLEAN)
    } else {
        TypeRef::named_nn_list_nn(&object_name)
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
            let PreparedModelSet { mut writes, permit } = hook.prepare(&ctx, &transaction).await?;

            for write in writes.iter_mut() {
                run_save_hook(hooks, &ctx, &object_name, action, write.active_model_mut())?;
            }

            let mut write_set: Vec<_> = writes
                .iter_mut()
                .map(ModelSetWrite::as_write_set_row)
                .collect();
            if let GuardAction::Block(reason) =
                run_write_set_hooks::<T>(&ctx, &object_name, action, &transaction, &mut write_set)
                    .await
            {
                return Err(guard_error(reason, "Write-set hook blocked the write."));
            }
            drop(write_set);

            let mut rows = Vec::with_capacity(writes.len());
            for write in writes {
                match write {
                    ModelSetWrite::Insert(active_model) => {
                        let model = active_model.insert(&transaction).await?;
                        enforce_create_filter::<T>(hooks, &ctx, &object_name, &transaction, &model)
                            .await?;
                        rows.push(model);
                    }
                    ModelSetWrite::Update { active_model, .. } => {
                        rows.push(active_model.update(&transaction).await?);
                    }
                    ModelSetWrite::Delete {
                        model,
                        active_model,
                    } => {
                        active_model.delete(&transaction).await?;
                        rows.push(model);
                    }
                }
            }

            transaction.commit().await?;
            if !rows.is_empty() {
                hooks.entity_watch(&ctx, &object_name, action).await;
            }
            permit.committed();

            if returns_boolean {
                Ok(Some(FieldValue::value(!rows.is_empty())))
            } else {
                Ok(Some(FieldValue::list(
                    rows.into_iter().map(FieldValue::owned_any),
                )))
            }
        })
    });
    for argument in declaration.arguments {
        field = field.argument(argument);
    }
    builder.mutations.push(field);
}
