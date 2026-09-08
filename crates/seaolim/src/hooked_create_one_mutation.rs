//! A local, SDL-identical replacement for Seaography's generated create-one
//! mutation that enforces `entity_filter` on the create path (upstream
//! issue #233: the filter is never consulted on Create).
//!
//! Differences from upstream `EntityCreateOneMutationBuilder` (rc.9):
//!
//! - The insert runs inside a transaction.
//! - After the per-row save hook, seaolim invokes its set-level hook with
//!   the complete one-row write set. Upstream has no equivalent hook.
//! - After the insert, the new row is re-selected through the
//!   `entity_filter(Create)` condition — row-level security's `WITH CHECK`
//!   shape. If the condition does not match the inserted row, the
//!   transaction rolls back and the mutation errors.
//!
//! Guard, field-guard, save-hook, and watch behavior mirrors upstream. The
//! SDL is pinned identical by `tests/hooked_mutations.rs`.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, IntoActiveModel,
    Iterable, ModelTrait, PrimaryKeyToColumn, QueryFilter, TransactionTrait,
};
use seaography::{
    async_graphql::dynamic::{Field, FieldFuture, FieldValue, InputValue, TypeRef},
    guard_error, prepare_active_model, Builder, BuilderContext, DatabaseContext,
    EntityCreateOneMutationBuilder, EntityInputBuilder, EntityObjectBuilder, GuardAction,
    OperationType, UserContext,
};

use crate::write_set_hooks::{run_write_set_hooks, WriteSetRow};

pub struct HookedCreateOneMutationBuilder {
    pub context: &'static BuilderContext,
}

impl HookedCreateOneMutationBuilder {
    pub fn to_field<T, A>(&self) -> Field
    where
        T: EntityTrait,
        <T as EntityTrait>::Model: Sync + IntoActiveModel<A>,
        A: ActiveModelTrait<Entity = T> + sea_orm::ActiveModelBehavior + Send + 'static,
    {
        let entity_input_builder = EntityInputBuilder {
            context: self.context,
        };
        let entity_object_builder = EntityObjectBuilder {
            context: self.context,
        };
        let object_name: String = entity_object_builder.type_name::<T>();

        let context = self.context;
        let hooks = &self.context.hooks;

        Field::new(
            EntityCreateOneMutationBuilder { context }.type_name::<T>(),
            TypeRef::named_nn(entity_object_builder.basic_type_name::<T>()),
            move |ctx| {
                let object_name = object_name.clone();
                FieldFuture::new(async move {
                    if let GuardAction::Block(reason) =
                        hooks.entity_guard(&ctx, &object_name, OperationType::Create)
                    {
                        return Err(guard_error(reason, "Entity guard triggered."));
                    }

                    let entity_input_builder = EntityInputBuilder { context };
                    let entity_object_builder = EntityObjectBuilder { context };
                    let value_accessor = ctx
                        .args
                        .try_get(&context.entity_create_one_mutation.data_field)?;
                    let input_object = &value_accessor.object()?;

                    for (column, _) in input_object.iter() {
                        if let GuardAction::Block(reason) =
                            hooks.field_guard(&ctx, &object_name, column, OperationType::Create)
                        {
                            return Err(guard_error(reason, "Field guard triggered."));
                        }
                    }

                    let db = ctx
                        .data::<DatabaseConnection>()?
                        .restricted(ctx.data_opt::<UserContext>())?;
                    let transaction = db.begin().await?;

                    let mut active_model = prepare_active_model::<T, A>(
                        &entity_input_builder,
                        &entity_object_builder,
                        input_object,
                    )?;

                    if let GuardAction::Block(reason) = hooks.before_active_model_save(
                        &ctx,
                        &object_name,
                        OperationType::Create,
                        &mut active_model,
                    ) {
                        return Err(guard_error(reason, "Blocked by before_active_model_save."));
                    }

                    let mut write_set = [WriteSetRow::Create {
                        active_model: &mut active_model,
                    }];
                    if let GuardAction::Block(reason) = run_write_set_hooks::<T>(
                        &ctx,
                        &object_name,
                        OperationType::Create,
                        &transaction,
                        &mut write_set,
                    )
                    .await
                    {
                        return Err(guard_error(reason, "Write-set hook blocked the create."));
                    }

                    let result = active_model.insert(&transaction).await?;

                    if let Some(filter) =
                        hooks.entity_filter(&ctx, &object_name, OperationType::Create)
                    {
                        let mut primary_key = Condition::all();
                        for key in <T::PrimaryKey as Iterable>::iter() {
                            let column = key.into_column();
                            primary_key = primary_key.add(column.eq(result.get(column)));
                        }
                        let in_scope = T::find()
                            .filter(primary_key)
                            .filter(filter)
                            .one(&transaction)
                            .await?
                            .is_some();
                        if !in_scope {
                            return Err(guard_error(
                                Some("created row is outside the caller's scope".to_owned()),
                                "Entity filter rejected the create.",
                            ));
                        }
                    }

                    transaction.commit().await?;

                    hooks
                        .entity_watch(&ctx, &object_name, OperationType::Create)
                        .await;

                    Ok(Some(FieldValue::owned_any(result)))
                })
            },
        )
        .argument(InputValue::new(
            &context.entity_create_one_mutation.data_field,
            TypeRef::named_nn(entity_input_builder.insert_type_name::<T>()),
        ))
    }
}

/// Register the hooked create-one mutation for an entity registered with
/// `mutation: false`. Output/input support types are added only if a prior
/// registrar has not already pushed them.
pub fn register_hooked_create_one<T, A>(builder: &mut Builder)
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
        EntityInputBuilder { context }.insert_input_object::<T>(),
    );
    builder
        .mutations
        .push(HookedCreateOneMutationBuilder { context }.to_field::<T, A>());
}
