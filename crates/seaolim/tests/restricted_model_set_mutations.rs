//! A restricted set mutation keeps a flattened field while the library owns
//! the resolver, the transaction, and persistence. The cross-row rule lives
//! in an entity-scoped write-set hook that amends siblings before any row is
//! written; a block rolls the whole set back.

mod common;

use common::{probes, write_rows};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseTransaction, EntityTrait,
    IntoActiveModel, QueryFilter, QueryOrder,
};
use seaography::{
    async_graphql::{
        dynamic::{InputValue, ResolverContext, TypeRef},
        Request, Result,
    },
    Builder, BuilderContext, GuardAction, LifecycleHooks, OperationType,
};
use seaolim::{
    register_restricted_model_set_mutation, ComposedHooks, ComposedWriteSetHooks, ModelSetWrite,
    PreparedModelSet, RestrictedModelSetMutation, RestrictedMutationField, WriteSetHook,
    WriteSetRow,
};

/// Selects the parent's rows in the requested order. Ordering the set is a
/// selection concern; assigning the ranks is the cross-row rule below.
struct ReorderChildren;

struct DeleteChildren;

#[sea_orm::prelude::async_trait::async_trait]
impl RestrictedModelSetMutation<write_rows::Entity, write_rows::ActiveModel> for ReorderChildren {
    async fn prepare(
        &self,
        ctx: &ResolverContext<'_>,
        transaction: &DatabaseTransaction,
    ) -> Result<PreparedModelSet<write_rows::ActiveModel, write_rows::Model>> {
        let parent_id = ctx.args.try_get("parent_id")?.i64()?;
        let ordered: Vec<i64> = ctx
            .args
            .try_get("ordered_ids")?
            .list()?
            .iter()
            .map(|value| value.i64())
            .collect::<Result<_>>()?;
        let mut rows = write_rows::Entity::find()
            .filter(write_rows::Column::ParentId.eq(parent_id))
            .all(transaction)
            .await?
            .into_iter()
            .map(|row| (row.id, row))
            .collect::<std::collections::HashMap<_, _>>();
        if rows.len() != ordered.len() {
            return Err(seaolim::mutation_error(
                "ordered_ids must be exactly this parent's rows.",
            ));
        }
        let writes = ordered
            .into_iter()
            .map(|id| {
                let model = rows
                    .remove(&id)
                    .ok_or_else(|| seaolim::mutation_error("unknown row in ordered_ids"))?;
                Ok(ModelSetWrite::Update {
                    active_model: model.clone().into_active_model(),
                    model,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(PreparedModelSet::new(writes, ()))
    }
}

#[sea_orm::prelude::async_trait::async_trait]
impl RestrictedModelSetMutation<write_rows::Entity, write_rows::ActiveModel> for DeleteChildren {
    async fn prepare(
        &self,
        ctx: &ResolverContext<'_>,
        transaction: &DatabaseTransaction,
    ) -> Result<PreparedModelSet<write_rows::ActiveModel, write_rows::Model>> {
        let parent_id = ctx.args.try_get("parent_id")?.i64()?;
        let writes = write_rows::Entity::find()
            .filter(write_rows::Column::ParentId.eq(parent_id))
            .all(transaction)
            .await?
            .into_iter()
            .map(|model| ModelSetWrite::Delete {
                active_model: model.clone().into_active_model(),
                model,
            })
            .collect();
        Ok(PreparedModelSet::new(writes, ()))
    }
}

/// The cross-row rule: a row's revision is its position in the complete set.
/// No per-row hook can state this, because no row knows its own index.
struct RevisionIsPosition;

#[sea_orm::prelude::async_trait::async_trait]
impl WriteSetHook for RevisionIsPosition {
    async fn before_write_set(
        &self,
        _ctx: &ResolverContext,
        _entity: &str,
        _action: OperationType,
        _transaction: &DatabaseTransaction,
        rows: &mut [WriteSetRow<'_>],
    ) -> GuardAction {
        for (position, row) in rows.iter_mut().enumerate() {
            let Some(active_model) = row.active_model_mut::<write_rows::ActiveModel>() else {
                return GuardAction::Block(Some("unexpected entity in write set".to_owned()));
            };
            active_model.revision = Set(position as i32);
        }
        GuardAction::Allow
    }
}

struct RejectEveryReorder;

#[sea_orm::prelude::async_trait::async_trait]
impl WriteSetHook for RejectEveryReorder {
    async fn before_write_set(
        &self,
        _ctx: &ResolverContext,
        _entity: &str,
        _action: OperationType,
        _transaction: &DatabaseTransaction,
        _rows: &mut [WriteSetRow<'_>],
    ) -> GuardAction {
        GuardAction::Block(Some("reorder refused".to_owned()))
    }
}

async fn schema_with(
    hooks: ComposedWriteSetHooks,
) -> (
    seaography::async_graphql::dynamic::Schema,
    sea_orm::DatabaseConnection,
) {
    static CONTEXT: std::sync::LazyLock<BuilderContext> = std::sync::LazyLock::new(|| {
        let mut context = BuilderContext::default();
        context.hooks = LifecycleHooks::new(ComposedHooks::default());
        context
    });
    let database = common::database_with_write_rows().await;
    for (id, revision) in [(1, 9), (2, 9), (3, 9)] {
        write_rows::ActiveModel {
            id: Set(id),
            parent_id: Set(1),
            active: Set(true),
            revision: Set(revision),
            stamp: Set(None),
        }
        .insert(&database)
        .await
        .expect("seed write row");
    }
    let mut builder = Builder::new(&CONTEXT, database.clone());
    seaography::register_entity!(builder, write_rows, mutation: false);
    register_restricted_model_set_mutation::<write_rows::Entity, write_rows::ActiveModel, _>(
        &mut builder,
        RestrictedMutationField::new("reorder_children", OperationType::Update)
            .argument(InputValue::new(
                "parent_id",
                TypeRef::named_nn(TypeRef::INT),
            ))
            .argument(InputValue::new(
                "ordered_ids",
                TypeRef::named_nn_list_nn(TypeRef::INT),
            )),
        ReorderChildren,
    );
    register_restricted_model_set_mutation::<write_rows::Entity, write_rows::ActiveModel, _>(
        &mut builder,
        RestrictedMutationField::new("delete_children", OperationType::Delete)
            .argument(InputValue::new(
                "parent_id",
                TypeRef::named_nn(TypeRef::INT),
            ))
            .returns_boolean(),
        DeleteChildren,
    );
    let schema = builder
        .schema_builder()
        .data(database.clone())
        .data(hooks)
        .finish()
        .expect("build restricted set schema");
    (schema, database)
}

const REORDER: &str =
    r#"mutation { reorder_children(parent_id: 1, ordered_ids: [3, 1, 2]) { id revision } }"#;

#[tokio::test]
async fn set_hook_amends_siblings_and_the_field_keeps_its_flat_shape() {
    let (schema, database) = schema_with(
        ComposedWriteSetHooks::default().add_for::<write_rows::Entity>(RevisionIsPosition),
    )
    .await;

    assert!(schema
        .sdl()
        .contains("reorder_children(parent_id: Int!, ordered_ids: [Int!]!): [WriteRows!]!"));

    let response = schema.execute(Request::new(REORDER)).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);

    let persisted = write_rows::Entity::find()
        .order_by_asc(write_rows::Column::Id)
        .all(&database)
        .await
        .expect("read reordered rows")
        .into_iter()
        .map(|row| (row.id, row.revision))
        .collect::<Vec<_>>();
    assert_eq!(persisted, vec![(1, 1), (2, 2), (3, 0)]);
    assert_eq!(
        response.data.to_string(),
        r#"{reorder_children: [{id: 3, revision: 0}, {id: 1, revision: 1}, {id: 2, revision: 2}]}"#
    );
}

#[tokio::test]
async fn a_set_delete_can_preserve_a_boolean_success_contract() {
    let (schema, database) = schema_with(ComposedWriteSetHooks::default()).await;

    assert!(schema
        .sdl()
        .contains("delete_children(parent_id: Int!): Boolean!"));

    let response = schema
        .execute(Request::new("mutation { delete_children(parent_id: 1) }"))
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert_eq!(response.data.to_string(), "{delete_children: true}");
    assert!(write_rows::Entity::find()
        .all(&database)
        .await
        .expect("read rows after delete")
        .is_empty());
}

#[tokio::test]
async fn a_blocking_set_hook_rolls_the_whole_set_back() {
    let (schema, database) = schema_with(
        ComposedWriteSetHooks::default().add_for::<write_rows::Entity>(RejectEveryReorder),
    )
    .await;

    let response = schema.execute(Request::new(REORDER)).await;

    assert!(response.errors[0].message.contains("reorder refused"));
    assert!(write_rows::Entity::find()
        .all(&database)
        .await
        .expect("read rows after block")
        .into_iter()
        .all(|row| row.revision == 9));
}

#[tokio::test]
async fn a_rule_registered_for_another_entity_never_sees_this_write_set() {
    struct FailIfCalled;

    #[sea_orm::prelude::async_trait::async_trait]
    impl WriteSetHook for FailIfCalled {
        async fn before_write_set(
            &self,
            _ctx: &ResolverContext,
            _entity: &str,
            _action: OperationType,
            _transaction: &DatabaseTransaction,
            _rows: &mut [WriteSetRow<'_>],
        ) -> GuardAction {
            GuardAction::Block(Some("foreign rule ran".to_owned()))
        }
    }

    let (schema, _database) = schema_with(
        ComposedWriteSetHooks::default()
            .add_for::<probes::Entity>(FailIfCalled)
            .add_for::<write_rows::Entity>(RevisionIsPosition),
    )
    .await;

    let response = schema.execute(Request::new(REORDER)).await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
}
