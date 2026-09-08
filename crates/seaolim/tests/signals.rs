//! Signals deliver the saved row on create — and, when the hooked builders
//! serve update/delete, on those paths too (the row as written for update,
//! the row about to die for delete).

mod common;

use std::sync::{Arc, LazyLock, Mutex};

use common::probes;
use sea_orm::{ActiveValue, DatabaseTransaction, Set};
use seaography::{
    async_graphql::{dynamic::ResolverContext, Request},
    Builder, BuilderContext, GuardAction, LifecycleHooks, OperationType,
};

use seaolim::{
    register_hooked_create_one, register_hooked_delete, register_hooked_update, ComposedHooks,
    ComposedWriteSetHooks, SignalBuffer, Signals, WriteSetHook, WriteSetRow,
};

type SeenEvents = Arc<Mutex<Vec<(OperationType, Option<String>)>>>;

static SEEN: LazyLock<SeenEvents> = LazyLock::new(SeenEvents::default);

#[allow(clippy::field_reassign_with_default)]
static CONTEXT: LazyLock<BuilderContext> = LazyLock::new(|| {
    let seen = SEEN.clone();
    let signals =
        Signals::default().on("Probes", move |action, row: Option<probes::ActiveModel>| {
            let seen = seen.clone();
            async move {
                let value = row.and_then(|row| match row.value {
                    ActiveValue::Set(value) | ActiveValue::Unchanged(value) => Some(value),
                    _ => None,
                });
                seen.lock().expect("events poisoned").push((action, value));
            }
        });
    let mut context = BuilderContext::default();
    context.hooks = LifecycleHooks::new(ComposedHooks::default().add(signals));
    context
});

struct AmendSignalPayload;

#[sea_orm::prelude::async_trait::async_trait]
impl WriteSetHook for AmendSignalPayload {
    async fn before_write_set(
        &self,
        _ctx: &ResolverContext,
        _entity: &str,
        action: OperationType,
        _transaction: &DatabaseTransaction,
        rows: &mut [WriteSetRow<'_>],
    ) -> GuardAction {
        if action == OperationType::Update {
            for row in rows {
                if let Some(active) = row.active_model_mut::<probes::ActiveModel>() {
                    if matches!(&active.value, ActiveValue::Set(value) if value == "before-set-hook")
                    {
                        active.value = Set("after-set-hook".to_owned());
                    }
                }
            }
        }
        GuardAction::Allow
    }
}

fn request(query: &str) -> Request {
    Request::new(query).data(SignalBuffer::default())
}

#[tokio::test]
async fn signals_carry_the_row_on_create_update_and_delete() {
    let database = common::database_with_probes().await;
    let mut builder = Builder::new(&CONTEXT, database.clone());
    seaography::register_entity!(builder, probes, mutation: false);
    register_hooked_create_one::<probes::Entity, probes::ActiveModel>(&mut builder);
    register_hooked_update::<probes::Entity, probes::ActiveModel>(&mut builder);
    register_hooked_delete::<probes::Entity, probes::ActiveModel>(&mut builder);
    let schema = builder
        .schema_builder()
        .data(database)
        .data(ComposedWriteSetHooks::default().add_for::<probes::Entity>(AmendSignalPayload))
        .finish()
        .expect("build signals schema");

    for mutation in [
        r#"mutation { probesCreateOne(data: { value: "hello" }) { id } }"#,
        r#"mutation { probesUpdate(data: { value: "renamed" }, filter: { id: { eq: 1 } }) { id } }"#,
        r#"mutation { probesUpdate(data: { value: "before-set-hook" }, filter: { id: { eq: 1 } }) { id } }"#,
        r#"mutation { probesDelete(filter: { id: { eq: 1 } }) }"#,
    ] {
        let response = schema.execute(request(mutation)).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
    }

    let seen = SEEN.lock().expect("events poisoned").clone();
    assert_eq!(
        seen,
        vec![
            (OperationType::Create, Some("hello".to_owned())),
            (OperationType::Update, Some("renamed".to_owned())),
            (OperationType::Update, Some("after-set-hook".to_owned())),
            (OperationType::Delete, Some("after-set-hook".to_owned())),
        ]
    );
}
