//! The hooked update/delete builders must be SDL-identical to upstream's
//! generated bundle (drift pin) and must run the save hook on the paths
//! upstream skips.

mod common;

use std::any::Any;
use std::sync::LazyLock;

use common::probes;
use sea_orm::{ActiveValue, ColumnTrait, Condition, Set};
use seaography::{
    async_graphql::dynamic::ResolverContext, Builder, BuilderContext, GuardAction, LifecycleHooks,
    LifecycleHooksInterface, OperationType,
};

use seaolim::{
    register_generated_mutations, register_hooked_create_one, register_hooked_delete,
    register_hooked_update, ComposedHooks, GeneratedMutations,
};

/// Stamps updates and vetoes deleting rows whose value is "keep".
struct WriteRules;

impl LifecycleHooksInterface for WriteRules {
    fn before_active_model_save(
        &self,
        _ctx: &ResolverContext,
        _entity: &str,
        action: OperationType,
        active_model: &mut dyn Any,
    ) -> GuardAction {
        let Some(row) = active_model.downcast_mut::<probes::ActiveModel>() else {
            return GuardAction::Allow;
        };
        match action {
            OperationType::Update => {
                row.stamp = Set(Some("updated-by-hook".to_owned()));
                GuardAction::Allow
            }
            OperationType::Delete => {
                if matches!(&row.value, ActiveValue::Unchanged(value) if value == "keep") {
                    GuardAction::Block(Some("row is protected".to_owned()))
                } else {
                    GuardAction::Allow
                }
            }
            _ => GuardAction::Allow,
        }
    }
}

/// Scopes creates: rows with value "forbidden" are outside the caller's
/// visible set, so creating one must fail.
struct CreateScope;

impl LifecycleHooksInterface for CreateScope {
    fn entity_filter(
        &self,
        _ctx: &ResolverContext,
        _entity: &str,
        action: OperationType,
    ) -> Option<Condition> {
        (action == OperationType::Create)
            .then(|| Condition::all().add(probes::Column::Value.ne("forbidden")))
    }
}

#[allow(clippy::field_reassign_with_default)]
fn context_with(hooks: LifecycleHooks) -> BuilderContext {
    let mut context = BuilderContext::default();
    context.hooks = hooks;
    context
}

static PLAIN: LazyLock<BuilderContext> = LazyLock::new(BuilderContext::default);
static RULED: LazyLock<BuilderContext> = LazyLock::new(|| {
    context_with(LifecycleHooks::new(
        ComposedHooks::default().add(WriteRules),
    ))
});
static SCOPED: LazyLock<BuilderContext> = LazyLock::new(|| {
    context_with(LifecycleHooks::new(
        ComposedHooks::default().add(CreateScope),
    ))
});

/// Full bundle, with create-one, update, and delete served by the hooked
/// builders and create-batch by the generated one.
async fn hooked_schema(
    context: &'static BuilderContext,
) -> seaography::async_graphql::dynamic::Schema {
    let database = common::database_with_probes().await;
    let mut builder = Builder::new(context, database.clone());
    seaography::register_entity!(builder, probes, mutation: false);
    register_hooked_create_one::<probes::Entity, probes::ActiveModel>(&mut builder);
    register_generated_mutations::<probes::Entity, probes::ActiveModel>(
        &mut builder,
        GeneratedMutations {
            create_one: false,
            create_batch: true,
            update: false,
            delete: false,
        },
    );
    register_hooked_update::<probes::Entity, probes::ActiveModel>(&mut builder);
    register_hooked_delete::<probes::Entity, probes::ActiveModel>(&mut builder);
    builder
        .schema_builder()
        .data(database)
        .finish()
        .expect("build hooked schema")
}

#[tokio::test]
async fn hooked_bundle_is_sdl_identical_to_native_bundle() {
    assert_eq!(
        hooked_schema(&PLAIN).await.sdl(),
        common::probes_schema(&PLAIN).await.sdl()
    );
}

#[tokio::test]
async fn update_runs_the_save_hook_per_row() {
    let schema = hooked_schema(&RULED).await;

    let response = schema
        .execute(r#"mutation { probesCreateOne(data: { value: "a" }) { id } }"#)
        .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);

    let response = schema
        .execute(
            r#"mutation {
                probesUpdate(data: { value: "b" }, filter: { id: { eq: 1 } }) { value stamp }
            }"#,
        )
        .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let data = serde_json::to_value(response.data).expect("serialize");
    assert_eq!(
        data["probesUpdate"],
        serde_json::json!([{ "value": "b", "stamp": "updated-by-hook" }])
    );
}

#[tokio::test]
async fn create_enforces_the_entity_filter() {
    let schema = hooked_schema(&SCOPED).await;

    let allowed = schema
        .execute(r#"mutation { probesCreateOne(data: { value: "fine" }) { value } }"#)
        .await;
    assert!(allowed.errors.is_empty(), "{:?}", allowed.errors);

    let rejected = schema
        .execute(r#"mutation { probesCreateOne(data: { value: "forbidden" }) { value } }"#)
        .await;
    assert_eq!(rejected.errors.len(), 1);
    assert!(rejected.errors[0]
        .message
        .contains("outside the caller's scope"));

    let rows = schema
        .execute(r#"query { probes { nodes { value } } }"#)
        .await;
    let data = serde_json::to_value(rows.data).expect("serialize");
    assert_eq!(
        data["probes"]["nodes"],
        serde_json::json!([{ "value": "fine" }]),
        "rejected create must not persist"
    );
}

#[tokio::test]
async fn delete_lets_a_hook_veto_individual_rows() {
    let schema = hooked_schema(&RULED).await;

    for value in ["keep", "discard"] {
        let response = schema
            .execute(format!(
                r#"mutation {{ probesCreateOne(data: {{ value: "{value}" }}) {{ id }} }}"#
            ))
            .await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
    }

    let vetoed = schema
        .execute(r#"mutation { probesDelete(filter: { value: { eq: "keep" } }) }"#)
        .await;
    assert_eq!(vetoed.errors.len(), 1);
    assert!(vetoed.errors[0].message.contains("row is protected"));

    let allowed = schema
        .execute(r#"mutation { probesDelete(filter: { value: { eq: "discard" } }) }"#)
        .await;
    assert!(allowed.errors.is_empty(), "{:?}", allowed.errors);
    let data = serde_json::to_value(allowed.data).expect("serialize");
    assert_eq!(data["probesDelete"], 1);

    let remaining = schema
        .execute(r#"query { probes { nodes { value } } }"#)
        .await;
    let data = serde_json::to_value(remaining.data).expect("serialize");
    assert_eq!(
        data["probes"]["nodes"],
        serde_json::json!([{ "value": "keep" }])
    );
}
