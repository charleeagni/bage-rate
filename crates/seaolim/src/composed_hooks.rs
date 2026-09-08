//! The application-owned composer for Seaography lifecycle hooks.
//!
//! Seaography's `MultiLifecycleHooks` (2.0.0-rc.9, unchanged on upstream
//! `main` as of 2026-08-23) forwards only four of the five
//! `LifecycleHooksInterface` methods. `before_active_model_save` falls
//! through to the trait default, which allows the write untouched, so
//! composing hook sets there silently disables every child's save-time
//! mutation and validation. `tests/hook_forwarding.rs` reproduces this.
//!
//! This composer forwards every method and deliberately has no default
//! fallthroughs: when upstream adds a hook method, adopting it here must be
//! an explicit decision, not a silent no-op.

use std::any::Any;

use sea_orm::{entity::prelude::async_trait, Condition};
use seaography::{
    async_graphql::dynamic::ResolverContext, GuardAction, LifecycleHooksInterface, OperationType,
};

/// Hook sets run in registration order. Guards short-circuit on the first
/// `Block`; `before_active_model_save` hands each hook the model as mutated
/// by the hooks before it; filters are AND-combined.
#[derive(Default)]
pub struct ComposedHooks {
    hooks: Vec<Box<dyn LifecycleHooksInterface>>,
}

impl ComposedHooks {
    #[allow(clippy::should_implement_trait)]
    pub fn add<T: LifecycleHooksInterface + 'static>(mut self, hook: T) -> Self {
        self.hooks.push(Box::new(hook));
        self
    }
}

#[async_trait::async_trait]
impl LifecycleHooksInterface for ComposedHooks {
    fn entity_guard(
        &self,
        ctx: &ResolverContext,
        entity: &str,
        action: OperationType,
    ) -> GuardAction {
        for hook in &self.hooks {
            if let GuardAction::Block(reason) = hook.entity_guard(ctx, entity, action) {
                return GuardAction::Block(reason);
            }
        }
        GuardAction::Allow
    }

    async fn entity_watch(&self, ctx: &ResolverContext, entity: &str, action: OperationType) {
        for hook in &self.hooks {
            hook.entity_watch(ctx, entity, action).await;
        }
    }

    fn field_guard(
        &self,
        ctx: &ResolverContext,
        entity: &str,
        field: &str,
        action: OperationType,
    ) -> GuardAction {
        for hook in &self.hooks {
            if let GuardAction::Block(reason) = hook.field_guard(ctx, entity, field, action) {
                return GuardAction::Block(reason);
            }
        }
        GuardAction::Allow
    }

    fn entity_filter(
        &self,
        ctx: &ResolverContext,
        entity: &str,
        action: OperationType,
    ) -> Option<Condition> {
        let mut combined = Condition::all();
        for hook in &self.hooks {
            if let Some(condition) = hook.entity_filter(ctx, entity, action) {
                if !condition.is_empty() {
                    combined = combined.add(condition);
                }
            }
        }
        if combined.is_empty() {
            None
        } else {
            Some(combined)
        }
    }

    fn before_active_model_save(
        &self,
        ctx: &ResolverContext,
        entity: &str,
        action: OperationType,
        active_model: &mut dyn Any,
    ) -> GuardAction {
        for hook in &self.hooks {
            if let GuardAction::Block(reason) =
                hook.before_active_model_save(ctx, entity, action, active_model)
            {
                return GuardAction::Block(reason);
            }
        }
        GuardAction::Allow
    }
}
