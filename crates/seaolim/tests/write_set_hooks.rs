//! Set hooks see one mutation's complete write set after row hooks and
//! before persistence. Blocks roll back; amendments are persisted.

mod common;

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock};

use common::{probes, write_rows};
use sea_orm::{ActiveValue, ConnectionTrait, DatabaseTransaction, EntityTrait, Schema, Set, Value};
use seaography::{
    async_graphql::dynamic::ResolverContext, Builder, BuilderContext, GuardAction, OperationType,
};
use seaolim::{
    register_hooked_create_one, register_hooked_delete, register_hooked_update,
    ComposedWriteSetHooks, WriteSetHook, WriteSetRow,
};

static CONTEXT: LazyLock<BuilderContext> = LazyLock::new(BuilderContext::default);

async fn schema_with(hooks: ComposedWriteSetHooks) -> seaography::async_graphql::dynamic::Schema {
    let database = common::database_with_write_rows().await;
    let mut builder = Builder::new(&CONTEXT, database.clone());
    seaography::register_entity!(builder, write_rows, mutation: false);
    register_hooked_create_one::<write_rows::Entity, write_rows::ActiveModel>(&mut builder);
    register_hooked_update::<write_rows::Entity, write_rows::ActiveModel>(&mut builder);
    register_hooked_delete::<write_rows::Entity, write_rows::ActiveModel>(&mut builder);
    builder
        .schema_builder()
        .data(database)
        .data(hooks)
        .finish()
        .expect("build write-set schema")
}

async fn schema_with_two_entities(
    hooks: ComposedWriteSetHooks,
) -> seaography::async_graphql::dynamic::Schema {
    let database = common::database_with_write_rows().await;
    let create =
        Schema::new(database.get_database_backend()).create_table_from_entity(probes::Entity);
    database
        .execute(&create)
        .await
        .expect("create probes table");
    let mut builder = Builder::new(&CONTEXT, database.clone());
    seaography::register_entity!(builder, write_rows, mutation: false);
    seaography::register_entity!(builder, probes, mutation: false);
    register_hooked_create_one::<write_rows::Entity, write_rows::ActiveModel>(&mut builder);
    register_hooked_create_one::<probes::Entity, probes::ActiveModel>(&mut builder);
    builder
        .schema_builder()
        .data(database)
        .data(hooks)
        .finish()
        .expect("build two-entity write-set schema")
}

fn active_value<T: Clone + Into<Value>>(value: &ActiveValue<T>) -> Option<T> {
    match value {
        ActiveValue::Set(value) | ActiveValue::Unchanged(value) => Some(value.clone()),
        ActiveValue::NotSet => None,
    }
}

struct OneActivePerParent;

#[sea_orm::prelude::async_trait::async_trait]
impl WriteSetHook for OneActivePerParent {
    async fn before_write_set(
        &self,
        _ctx: &ResolverContext,
        _entity: &str,
        action: OperationType,
        transaction: &DatabaseTransaction,
        rows: &mut [WriteSetRow<'_>],
    ) -> GuardAction {
        if action == OperationType::Delete {
            return GuardAction::Allow;
        }

        let current = write_rows::Entity::find()
            .all(transaction)
            .await
            .expect("load sibling rows");
        let mut proposed: HashMap<i64, (i64, bool)> = current
            .into_iter()
            .map(|row| (row.id, (row.parent_id, row.active)))
            .collect();
        let mut next_create_id = -1;

        for row in rows {
            let old_id = row.old_model::<write_rows::Model>().map(|old| old.id);
            let Some(active) = row.active_model_mut::<write_rows::ActiveModel>() else {
                continue;
            };
            let Some(parent_id) = active_value(&active.parent_id) else {
                continue;
            };
            let Some(is_active) = active_value(&active.active) else {
                continue;
            };
            let id = old_id.unwrap_or_else(|| {
                let id = next_create_id;
                next_create_id -= 1;
                id
            });
            proposed.insert(id, (parent_id, is_active));
        }

        let mut active_by_parent: HashMap<i64, usize> = HashMap::new();
        for (parent_id, is_active) in proposed.values() {
            if *is_active {
                *active_by_parent.entry(*parent_id).or_default() += 1;
            }
        }
        if active_by_parent.values().any(|count| *count > 1) {
            GuardAction::Block(Some("only one active row is allowed per parent".to_owned()))
        } else {
            GuardAction::Allow
        }
    }
}

struct RevisionGuard;

#[sea_orm::prelude::async_trait::async_trait]
impl WriteSetHook for RevisionGuard {
    async fn before_write_set(
        &self,
        _ctx: &ResolverContext,
        _entity: &str,
        action: OperationType,
        _transaction: &DatabaseTransaction,
        rows: &mut [WriteSetRow<'_>],
    ) -> GuardAction {
        if action != OperationType::Update {
            return GuardAction::Allow;
        }
        for row in rows {
            let Some(current) = row.old_model::<write_rows::Model>().map(|old| old.revision) else {
                continue;
            };
            let Some(active) = row.active_model_mut::<write_rows::ActiveModel>() else {
                continue;
            };
            if !matches!(active.revision, ActiveValue::Set(expected) if expected == current) {
                return GuardAction::Block(Some("stale revision".to_owned()));
            }
            active.revision = Set(current + 1);
        }
        GuardAction::Allow
    }
}

struct StampSet;

#[sea_orm::prelude::async_trait::async_trait]
impl WriteSetHook for StampSet {
    async fn before_write_set(
        &self,
        _ctx: &ResolverContext,
        _entity: &str,
        _action: OperationType,
        _transaction: &DatabaseTransaction,
        rows: &mut [WriteSetRow<'_>],
    ) -> GuardAction {
        for row in rows {
            if let Some(active) = row.active_model_mut::<write_rows::ActiveModel>() {
                active.stamp = Set(Some("amended-as-a-set".to_owned()));
            }
        }
        GuardAction::Allow
    }
}

struct CountThenAllow(Arc<AtomicUsize>);

#[sea_orm::prelude::async_trait::async_trait]
impl WriteSetHook for CountThenAllow {
    async fn before_write_set(
        &self,
        _ctx: &ResolverContext,
        _entity: &str,
        _action: OperationType,
        _transaction: &DatabaseTransaction,
        _rows: &mut [WriteSetRow<'_>],
    ) -> GuardAction {
        self.0.fetch_add(1, Ordering::SeqCst);
        GuardAction::Allow
    }
}

struct CountThenBlock(Arc<AtomicUsize>, &'static str);

#[sea_orm::prelude::async_trait::async_trait]
impl WriteSetHook for CountThenBlock {
    async fn before_write_set(
        &self,
        _ctx: &ResolverContext,
        _entity: &str,
        _action: OperationType,
        _transaction: &DatabaseTransaction,
        _rows: &mut [WriteSetRow<'_>],
    ) -> GuardAction {
        self.0.fetch_add(1, Ordering::SeqCst);
        GuardAction::Block(Some(self.1.to_owned()))
    }
}

struct ObserveDelete(Arc<AtomicUsize>);

#[sea_orm::prelude::async_trait::async_trait]
impl WriteSetHook for ObserveDelete {
    async fn before_write_set(
        &self,
        _ctx: &ResolverContext,
        _entity: &str,
        action: OperationType,
        _transaction: &DatabaseTransaction,
        rows: &mut [WriteSetRow<'_>],
    ) -> GuardAction {
        if action == OperationType::Delete
            && rows.iter().all(|row| {
                matches!(row, WriteSetRow::Delete { .. })
                    && row
                        .old_model::<write_rows::Model>()
                        .is_some_and(|old| old.parent_id == 42)
            })
        {
            self.0.store(rows.len(), Ordering::SeqCst);
        }
        GuardAction::Allow
    }
}

async fn create(
    schema: &seaography::async_graphql::dynamic::Schema,
    parent_id: i64,
    active: bool,
    revision: i32,
) -> seaography::async_graphql::Response {
    schema
        .execute(format!(
            r#"mutation {{ writeRowsCreateOne(data: {{ parentId: {parent_id}, active: {active}, revision: {revision} }}) {{ id }} }}"#
        ))
        .await
}

#[tokio::test]
async fn sibling_uniqueness_blocks_create_and_update_and_rolls_back() {
    let schema = schema_with(
        ComposedWriteSetHooks::default().add_for::<write_rows::Entity>(OneActivePerParent),
    )
    .await;
    assert!(create(&schema, 7, true, 1).await.errors.is_empty());

    let rejected_create = create(&schema, 7, true, 1).await;
    assert_eq!(rejected_create.errors.len(), 1);
    assert!(rejected_create.errors[0]
        .message
        .contains("only one active row is allowed per parent"));

    assert!(create(&schema, 7, false, 1).await.errors.is_empty());
    let rejected_update = schema
        .execute(
            r#"mutation { writeRowsUpdate(data: { active: true }, filter: { id: { eq: 2 } }) { id active } }"#,
        )
        .await;
    assert_eq!(rejected_update.errors.len(), 1);

    let response = schema
        .execute(r#"query { writeRows { nodes { id active } } }"#)
        .await;
    let data = serde_json::to_value(response.data).expect("serialize rows");
    assert_eq!(
        data["writeRows"]["nodes"],
        serde_json::json!([
            { "id": 1, "active": true },
            { "id": 2, "active": false }
        ]),
        "both rejected mutations must roll back"
    );
}

#[tokio::test]
async fn revision_guard_blocks_stale_and_advances_current_revision() {
    let schema =
        schema_with(ComposedWriteSetHooks::default().add_for::<write_rows::Entity>(RevisionGuard))
            .await;
    assert!(create(&schema, 1, false, 4).await.errors.is_empty());

    let stale = schema
        .execute(
            r#"mutation { writeRowsUpdate(data: { revision: 3 }, filter: { id: { eq: 1 } }) { revision } }"#,
        )
        .await;
    assert_eq!(stale.errors.len(), 1);
    assert!(stale.errors[0].message.contains("stale revision"));

    let current = schema
        .execute(
            r#"mutation { writeRowsUpdate(data: { revision: 4 }, filter: { id: { eq: 1 } }) { revision } }"#,
        )
        .await;
    assert!(current.errors.is_empty(), "{:?}", current.errors);
    let data = serde_json::to_value(current.data).expect("serialize update");
    assert_eq!(data["writeRowsUpdate"][0]["revision"], 5);
}

#[tokio::test]
async fn amendments_apply_to_every_row_in_the_set() {
    let schema =
        schema_with(ComposedWriteSetHooks::default().add_for::<write_rows::Entity>(StampSet)).await;
    assert!(create(&schema, 9, false, 1).await.errors.is_empty());
    assert!(create(&schema, 9, false, 1).await.errors.is_empty());

    let response = schema
        .execute(
            r#"mutation { writeRowsUpdate(data: { active: true }, filter: { parentId: { eq: 9 } }) { stamp } }"#,
        )
        .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let data = serde_json::to_value(response.data).expect("serialize update");
    assert_eq!(
        data["writeRowsUpdate"],
        serde_json::json!([
            { "stamp": "amended-as-a-set" },
            { "stamp": "amended-as-a-set" }
        ])
    );
}

#[tokio::test]
async fn composed_hooks_run_in_order_and_stop_at_the_first_block() {
    let first = Arc::new(AtomicUsize::new(0));
    let blocker = Arc::new(AtomicUsize::new(0));
    let skipped = Arc::new(AtomicUsize::new(0));
    let hooks = ComposedWriteSetHooks::default()
        .add_for::<write_rows::Entity>(CountThenAllow(first.clone()))
        .add_for::<write_rows::Entity>(CountThenBlock(blocker.clone(), "first block"))
        .add_for::<write_rows::Entity>(CountThenBlock(skipped.clone(), "later block"));
    let schema = schema_with(hooks).await;

    let response = create(&schema, 1, false, 1).await;
    assert_eq!(response.errors.len(), 1);
    assert!(response.errors[0].message.contains("first block"));
    assert_eq!(first.load(Ordering::SeqCst), 1);
    assert_eq!(blocker.load(Ordering::SeqCst), 1);
    assert_eq!(skipped.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn entity_scoped_hook_cannot_observe_or_block_another_entity() {
    let calls = Arc::new(AtomicUsize::new(0));
    let hooks = ComposedWriteSetHooks::default()
        .add_for::<write_rows::Entity>(CountThenBlock(calls.clone(), "write rows only"));
    let schema = schema_with_two_entities(hooks).await;

    let unrelated = schema
        .execute(r#"mutation { probesCreateOne(data: { value: "allowed" }) { id } }"#)
        .await;
    assert!(unrelated.errors.is_empty(), "{:?}", unrelated.errors);
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let scoped = create(&schema, 1, false, 1).await;
    assert_eq!(scoped.errors.len(), 1);
    assert!(scoped.errors[0].message.contains("write rows only"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn delete_write_set_exposes_fetched_old_rows() {
    let seen = Arc::new(AtomicUsize::new(0));
    let schema = schema_with(
        ComposedWriteSetHooks::default().add_for::<write_rows::Entity>(ObserveDelete(seen.clone())),
    )
    .await;
    assert!(create(&schema, 42, false, 1).await.errors.is_empty());
    assert!(create(&schema, 42, false, 1).await.errors.is_empty());

    let response = schema
        .execute(r#"mutation { writeRowsDelete(filter: { parentId: { eq: 42 } }) }"#)
        .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert_eq!(seen.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn write_set_hook_registration_does_not_change_sdl() {
    let plain = schema_with(ComposedWriteSetHooks::default()).await.sdl();
    let hooked =
        schema_with(ComposedWriteSetHooks::default().add_for::<write_rows::Entity>(StampSet))
            .await
            .sdl();
    assert_eq!(hooked, plain);
}
