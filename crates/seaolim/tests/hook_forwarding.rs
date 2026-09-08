//! Reproduces the Seaography 2.0.0-rc.9 composition bug and proves
//! `ComposedHooks` immune to it.
//!
//! The bug: `MultiLifecycleHooks` forwards four of the five
//! `LifecycleHooksInterface` methods but not `before_active_model_save`,
//! so a composed hook's save-time mutation silently never runs.

mod common;

use std::any::Any;
use std::sync::LazyLock;

use common::probes;
use sea_orm::Set;
use seaography::{
    async_graphql::dynamic::ResolverContext, BuilderContext, GuardAction, LifecycleHooks,
    LifecycleHooksInterface, MultiLifecycleHooks, OperationType,
};

use seaolim::ComposedHooks;

/// Stamps every probe row at save time. If composition forwards the save
/// hook, inserted rows carry `stamp = "stamped"`.
struct StampHook;

impl LifecycleHooksInterface for StampHook {
    fn before_active_model_save(
        &self,
        _ctx: &ResolverContext,
        _entity: &str,
        _action: OperationType,
        active_model: &mut dyn Any,
    ) -> GuardAction {
        if let Some(model) = active_model.downcast_mut::<probes::ActiveModel>() {
            model.stamp = Set(Some("stamped".to_owned()));
        }
        GuardAction::Allow
    }
}

/// Blocks every create at the entity guard, to show guard forwarding still
/// short-circuits when hooks are composed.
struct DenyCreateHook;

impl LifecycleHooksInterface for DenyCreateHook {
    fn entity_guard(
        &self,
        _ctx: &ResolverContext,
        _entity: &str,
        action: OperationType,
    ) -> GuardAction {
        if action == OperationType::Create {
            GuardAction::Block(Some("creates denied".to_owned()))
        } else {
            GuardAction::Allow
        }
    }
}

// BuilderContext is #[non_exhaustive], so assignment after default() is the
// only construction path.
#[allow(clippy::field_reassign_with_default)]
fn context_with(hooks: LifecycleHooks) -> BuilderContext {
    let mut context = BuilderContext::default();
    context.hooks = hooks;
    context
}

static UPSTREAM_MULTI: LazyLock<BuilderContext> = LazyLock::new(|| {
    context_with(LifecycleHooks::new(
        MultiLifecycleHooks::default().add(StampHook),
    ))
});

static COMPOSED: LazyLock<BuilderContext> =
    LazyLock::new(|| context_with(LifecycleHooks::new(ComposedHooks::default().add(StampHook))));

static COMPOSED_WITH_DENY: LazyLock<BuilderContext> = LazyLock::new(|| {
    context_with(LifecycleHooks::new(
        ComposedHooks::default().add(StampHook).add(DenyCreateHook),
    ))
});

const CREATE_ONE: &str = r#"mutation { probesCreateOne(data: { value: "x" }) { value stamp } }"#;

async fn create_one_stamp(context: &'static BuilderContext) -> serde_json::Value {
    let response = common::probes_schema(context)
        .await
        .execute(CREATE_ONE)
        .await;
    assert!(
        response.errors.is_empty(),
        "create failed: {:?}",
        response.errors
    );
    serde_json::to_value(response.data).expect("serialize response")["probesCreateOne"]["stamp"]
        .clone()
}

/// Documents the upstream bug. When a dependency bump makes this test fail,
/// the bug is fixed: prefer upstream composition again and retire this pin.
#[tokio::test]
async fn upstream_multi_hooks_silently_drop_the_save_hook() {
    assert_eq!(
        create_one_stamp(&UPSTREAM_MULTI).await,
        serde_json::Value::Null
    );
}

#[tokio::test]
async fn composed_hooks_forward_the_save_hook() {
    assert_eq!(create_one_stamp(&COMPOSED).await, "stamped");
}

#[tokio::test]
async fn composed_hooks_still_short_circuit_guards() {
    let response = common::probes_schema(&COMPOSED_WITH_DENY)
        .await
        .execute(CREATE_ONE)
        .await;
    assert_eq!(response.errors.len(), 1);
    assert!(
        response.errors[0].message.contains("creates denied"),
        "unexpected error: {}",
        response.errors[0].message
    );
}
