//! A local, SDL-identical replacement for Seaography's generated update
//! mutation that closes the rc.9 update write-gap without forking.
//!
//! Differences from upstream `EntityUpdateMutationBuilder` (rc.9):
//!
//! - Rows are updated one by one instead of via one `update_many`, so each
//!   row's save runs with full context: the patch is folded into the
//!   fetched row's `ActiveModel` (old values `Unchanged`, patched values
//!   `Set`), giving hooks a DRF `perform_update`-grade view.
//! - `before_active_model_save` is invoked per row with
//!   `OperationType::Update` — the call upstream never makes. A `Block`
//!   aborts the transaction.
//! - After all per-row hooks allow the mutation, seaolim invokes its
//!   set-level hook once with every old row and proposed ActiveModel.
//!   Upstream has no equivalent hook.
//! - SeaORM's `ActiveModelBehavior::before_save`/`after_save` run per row
//!   as a consequence of using `ActiveModel::update`.
//!
//! Guard, filter, and watch behavior mirrors upstream. The SDL is pinned
//! identical by `tests/hooked_mutations.rs`.

use sea_orm::{
    ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait, IntoActiveModel, Iterable,
    QueryFilter, QueryTrait, TransactionTrait,
};
use seaography::{
    async_graphql::dynamic::{Field, FieldFuture, FieldValue, InputValue, TypeRef},
    get_filter_conditions, guard_error, prepare_active_model, Builder, BuilderContext,
    DatabaseContext, EntityInputBuilder, EntityObjectBuilder, EntityUpdateMutationBuilder,
    FilterInputBuilder, GuardAction, OperationType, UserContext,
};

use crate::write_set_hooks::{run_write_set_hooks, WriteSetRow};

pub struct HookedUpdateMutationBuilder {
    pub context: &'static BuilderContext,
}

impl HookedUpdateMutationBuilder {
    pub fn to_field<T, A>(&self) -> Field
    where
        T: EntityTrait,
        <T as EntityTrait>::Model: Sync + IntoActiveModel<A>,
        A: ActiveModelTrait<Entity = T> + sea_orm::ActiveModelBehavior + Send + 'static,
    {
        let entity_input_builder = EntityInputBuilder {
            context: self.context,
        };
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
            EntityUpdateMutationBuilder { context }.type_name::<T>(),
            TypeRef::named_nn_list_nn(entity_object_builder.basic_type_name::<T>()),
            move |ctx| {
                let object_name = object_name.clone();
                FieldFuture::new(async move {
                    if let GuardAction::Block(reason) =
                        hooks.entity_guard(&ctx, &object_name, OperationType::Update)
                    {
                        return Err(guard_error(reason, "Entity guard triggered."));
                    }

                    let db = ctx
                        .data::<DatabaseConnection>()?
                        .restricted(ctx.data_opt::<UserContext>())?;
                    let transaction = db.begin().await?;

                    let entity_input_builder = EntityInputBuilder { context };
                    let entity_object_builder = EntityObjectBuilder { context };

                    let filters = ctx.args.get(&context.entity_update_mutation.filter_field);
                    let filter_condition = get_filter_conditions::<T>(context, filters)?;

                    let value_accessor = ctx
                        .args
                        .try_get(&context.entity_update_mutation.data_field)?;
                    let input_object = &value_accessor.object()?;

                    for (column, _) in input_object.iter() {
                        if let GuardAction::Block(reason) =
                            hooks.field_guard(&ctx, &object_name, column, OperationType::Update)
                        {
                            return Err(guard_error(reason, "Field guard triggered."));
                        }
                    }

                    let patch = prepare_active_model::<T, A>(
                        &entity_input_builder,
                        &entity_object_builder,
                        input_object,
                    )?;

                    let entity_filter =
                        hooks.entity_filter(&ctx, &object_name, OperationType::Update);

                    let models: Vec<T::Model> = T::find()
                        .apply_if(entity_filter, |query, filter| query.filter(filter))
                        .filter(filter_condition)
                        .all(&transaction)
                        .await?;

                    let mut writes = Vec::with_capacity(models.len());
                    for model in models {
                        let mut row: A = model.clone().into_active_model();
                        for column in T::Column::iter() {
                            if let ActiveValue::Set(value) = patch.get(column) {
                                row.set(column, value);
                            }
                        }
                        if let GuardAction::Block(reason) = hooks.before_active_model_save(
                            &ctx,
                            &object_name,
                            OperationType::Update,
                            &mut row,
                        ) {
                            return Err(guard_error(reason, "Save hook blocked the update."));
                        }
                        writes.push((model, row));
                    }

                    let mut write_set: Vec<_> = writes
                        .iter_mut()
                        .map(|(old_model, active_model)| WriteSetRow::Update {
                            old_model,
                            active_model,
                        })
                        .collect();
                    if let GuardAction::Block(reason) = run_write_set_hooks::<T>(
                        &ctx,
                        &object_name,
                        OperationType::Update,
                        &transaction,
                        &mut write_set,
                    )
                    .await
                    {
                        return Err(guard_error(reason, "Write-set hook blocked the update."));
                    }
                    drop(write_set);

                    let mut updated = Vec::with_capacity(writes.len());
                    for (_, row) in writes {
                        updated.push(row.update(&transaction).await?);
                    }

                    transaction.commit().await?;

                    hooks
                        .entity_watch(&ctx, &object_name, OperationType::Update)
                        .await;

                    Ok(Some(FieldValue::list(
                        updated.into_iter().map(FieldValue::owned_any),
                    )))
                })
            },
        )
        .argument(InputValue::new(
            &context.entity_update_mutation.data_field,
            TypeRef::named_nn(entity_input_builder.update_type_name::<T>()),
        ))
        .argument(InputValue::new(
            &context.entity_update_mutation.filter_field,
            TypeRef::named(entity_filter_input_builder.type_name(&object_name_)),
        ))
    }
}

/// Register the hooked update mutation for an entity registered with
/// `mutation: false`. Output/input support types are added only if a prior
/// registrar has not already pushed them.
pub fn register_hooked_update<T, A>(builder: &mut Builder)
where
    T: EntityTrait,
    <T as EntityTrait>::Model: Sync + IntoActiveModel<A>,
    A: ActiveModelTrait<Entity = T> + sea_orm::ActiveModelBehavior + Send + 'static,
{
    let context = builder.context;
    crate::generated_mutations::push_output_once(
        builder,
        EntityObjectBuilder { context }.to_basic_object::<T>(),
    );
    crate::generated_mutations::push_input_once(
        builder,
        EntityInputBuilder { context }.update_input_object::<T>(),
    );
    builder
        .mutations
        .push(HookedUpdateMutationBuilder { context }.to_field::<T, A>());
}
