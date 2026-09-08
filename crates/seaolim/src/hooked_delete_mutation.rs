//! A local, SDL-identical replacement for Seaography's generated delete
//! mutation that closes the rc.9 delete write-gap without forking.
//!
//! Differences from upstream `EntityDeleteMutationBuilder` (rc.9):
//!
//! - Rows are fetched and deleted one by one inside a transaction instead
//!   of via one `delete_many`, so each row is visible before it dies.
//! - `before_active_model_save` is invoked per row with
//!   `OperationType::Delete` — an action value upstream never sends to the
//!   save hook — letting one composed hook set veto individual deletions.
//!   A `Block` aborts the whole transaction.
//! - After all per-row hooks allow the mutation, seaolim invokes its
//!   set-level hook once with every fetched row and deletion ActiveModel.
//!   Upstream has no equivalent hook.
//! - SeaORM's `ActiveModelBehavior::before_delete`/`after_delete` run per
//!   row as a consequence of using `ActiveModel::delete`.
//!
//! Guard, filter, watch behavior and the `Int` rows-affected result mirror
//! upstream. The SDL is pinned identical by `tests/hooked_mutations.rs`.

use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, QueryTrait,
    TransactionTrait,
};
use seaography::{
    async_graphql,
    async_graphql::dynamic::{Field, FieldFuture, InputValue, TypeRef},
    get_filter_conditions, guard_error, Builder, BuilderContext, DatabaseContext,
    EntityDeleteMutationBuilder, EntityObjectBuilder, FilterInputBuilder, GuardAction,
    OperationType, UserContext,
};

use crate::write_set_hooks::{run_write_set_hooks, WriteSetRow};

pub struct HookedDeleteMutationBuilder {
    pub context: &'static BuilderContext,
}

impl HookedDeleteMutationBuilder {
    pub fn to_field<T, A>(&self) -> Field
    where
        T: EntityTrait,
        <T as EntityTrait>::Model: Sync + IntoActiveModel<A>,
        A: ActiveModelTrait<Entity = T> + sea_orm::ActiveModelBehavior + Send + 'static,
    {
        let entity_filter_input_builder = FilterInputBuilder {
            context: self.context,
        };
        let entity_object_builder = EntityObjectBuilder {
            context: self.context,
        };
        let object_name: String = entity_object_builder.type_name::<T>();
        let object_name_ = object_name.clone();

        let context = self.context;
        let hooks = &self.context.hooks;

        Field::new(
            EntityDeleteMutationBuilder { context }.type_name::<T>(),
            TypeRef::named_nn(TypeRef::INT),
            move |ctx| {
                let object_name = object_name.clone();
                FieldFuture::new(async move {
                    if let GuardAction::Block(reason) =
                        hooks.entity_guard(&ctx, &object_name, OperationType::Delete)
                    {
                        return Err(guard_error(reason, "Entity guard triggered."));
                    }

                    let db = ctx
                        .data::<DatabaseConnection>()?
                        .restricted(ctx.data_opt::<UserContext>())?;
                    let transaction = db.begin().await?;

                    let filters = ctx.args.get(&context.entity_delete_mutation.filter_field);
                    let filter_condition = get_filter_conditions::<T>(context, filters)?;

                    let entity_filter =
                        hooks.entity_filter(&ctx, &object_name, OperationType::Delete);

                    let models: Vec<T::Model> = T::find()
                        .apply_if(entity_filter, |query, filter| query.filter(filter))
                        .filter(filter_condition)
                        .all(&transaction)
                        .await?;

                    let mut writes = Vec::with_capacity(models.len());
                    for model in models {
                        let mut row: A = model.clone().into_active_model();
                        if let GuardAction::Block(reason) = hooks.before_active_model_save(
                            &ctx,
                            &object_name,
                            OperationType::Delete,
                            &mut row,
                        ) {
                            return Err(guard_error(reason, "Save hook blocked the delete."));
                        }
                        writes.push((model, row));
                    }

                    let mut write_set: Vec<_> = writes
                        .iter_mut()
                        .map(|(old_model, active_model)| WriteSetRow::Delete {
                            old_model,
                            active_model,
                        })
                        .collect();
                    if let GuardAction::Block(reason) = run_write_set_hooks::<T>(
                        &ctx,
                        &object_name,
                        OperationType::Delete,
                        &transaction,
                        &mut write_set,
                    )
                    .await
                    {
                        return Err(guard_error(reason, "Write-set hook blocked the delete."));
                    }
                    drop(write_set);

                    let mut rows_affected: u64 = 0;
                    for (_, row) in writes {
                        rows_affected += row.delete(&transaction).await?.rows_affected;
                    }

                    transaction.commit().await?;

                    hooks
                        .entity_watch(&ctx, &object_name, OperationType::Delete)
                        .await;

                    Ok(Some(async_graphql::Value::from(rows_affected)))
                })
            },
        )
        .argument(InputValue::new(
            &context.entity_delete_mutation.filter_field,
            TypeRef::named(entity_filter_input_builder.type_name(&object_name_)),
        ))
    }
}

/// Register the hooked delete mutation for an entity registered with
/// `mutation: false`.
pub fn register_hooked_delete<T, A>(builder: &mut Builder)
where
    T: EntityTrait,
    <T as EntityTrait>::Model: Sync + IntoActiveModel<A>,
    A: ActiveModelTrait<Entity = T> + sea_orm::ActiveModelBehavior + Send + 'static,
{
    let context = builder.context;
    builder
        .mutations
        .push(HookedDeleteMutationBuilder { context }.to_field::<T, A>());
}
