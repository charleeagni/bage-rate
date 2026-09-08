//! Set-level write hooks for Seaography mutations.
//!
//! Seaography 2.0.0-rc.9 exposes only
//! `LifecycleHooksInterface::before_active_model_save`, which runs once per
//! row and cannot inspect sibling rows from the same mutation. This module
//! adds a seaolim-owned hook over the complete write set. Hook code runs
//! inside the builders' open transaction after every row save hook and
//! before any row is persisted.

use std::any::{Any, TypeId};

use sea_orm::{entity::prelude::async_trait, DatabaseTransaction, EntityTrait};
use seaography::{async_graphql::dynamic::ResolverContext, GuardAction, OperationType};

/// One row in a mutation's write set.
///
/// Update and delete expose the fetched row beside the mutable ActiveModel.
/// Create has no old row. The ActiveModel in `Delete` describes the fetched
/// row that the builder is about to delete.
pub enum WriteSetRow<'a> {
    Create {
        active_model: &'a mut (dyn Any + Send),
    },
    Update {
        old_model: &'a (dyn Any + Send + Sync),
        active_model: &'a mut (dyn Any + Send),
    },
    Delete {
        old_model: &'a (dyn Any + Send + Sync),
        active_model: &'a mut (dyn Any + Send),
    },
}

impl WriteSetRow<'_> {
    /// Downcast the ActiveModel that the builder will persist or delete.
    pub fn active_model_mut<A: 'static>(&mut self) -> Option<&mut A> {
        match self {
            Self::Create { active_model }
            | Self::Update { active_model, .. }
            | Self::Delete { active_model, .. } => active_model.downcast_mut(),
        }
    }

    pub(crate) fn active_model(&self) -> &(dyn Any + Send) {
        match self {
            Self::Create { active_model }
            | Self::Update { active_model, .. }
            | Self::Delete { active_model, .. } => &**active_model,
        }
    }

    /// Downcast the fetched row for update or delete. Creates return `None`.
    pub fn old_model<M: 'static>(&self) -> Option<&M> {
        match self {
            Self::Create { .. } => None,
            Self::Update { old_model, .. } | Self::Delete { old_model, .. } => {
                old_model.downcast_ref()
            }
        }
    }
}

/// A rule that can inspect, amend, or block a complete mutation write set.
///
/// Upstream Seaography has no equivalent hook. Implementations may query
/// `transaction` to compare the proposed rows with current database state.
/// Returning `Block` aborts the mutation and rolls back the transaction.
#[async_trait::async_trait]
pub trait WriteSetHook: Send + Sync {
    async fn before_write_set(
        &self,
        ctx: &ResolverContext,
        entity: &str,
        action: OperationType,
        transaction: &DatabaseTransaction,
        rows: &mut [WriteSetRow<'_>],
    ) -> GuardAction;
}

/// Application-owned composition for [`WriteSetHook`] implementations.
///
/// Register every rule for its SeaORM entity with [`Self::add_for`]. Rules run
/// in registration order for that entity and see amendments made by earlier
/// rules. Dispatch stops at the first `Block`.
#[derive(Default)]
pub struct ComposedWriteSetHooks {
    hooks: Vec<ScopedWriteSetHook>,
}

struct ScopedWriteSetHook {
    entity: TypeId,
    hook: Box<dyn WriteSetHook>,
}

impl ComposedWriteSetHooks {
    pub fn add_for<T: EntityTrait + 'static>(mut self, hook: impl WriteSetHook + 'static) -> Self {
        self.hooks.push(ScopedWriteSetHook {
            entity: TypeId::of::<T>(),
            hook: Box::new(hook),
        });
        self
    }

    pub(crate) async fn run<T: EntityTrait + 'static>(
        &self,
        ctx: &ResolverContext<'_>,
        entity: &str,
        action: OperationType,
        transaction: &DatabaseTransaction,
        rows: &mut [WriteSetRow<'_>],
    ) -> GuardAction {
        for registration in self
            .hooks
            .iter()
            .filter(|registration| registration.entity == TypeId::of::<T>())
        {
            if let GuardAction::Block(reason) = registration
                .hook
                .before_write_set(ctx, entity, action, transaction, rows)
                .await
            {
                return GuardAction::Block(reason);
            }
        }
        GuardAction::Allow
    }
}

pub(crate) async fn run_write_set_hooks<T: EntityTrait + 'static>(
    ctx: &ResolverContext<'_>,
    entity: &str,
    action: OperationType,
    transaction: &DatabaseTransaction,
    rows: &mut [WriteSetRow<'_>],
) -> GuardAction {
    let Some(hooks) = ctx.data_opt::<ComposedWriteSetHooks>() else {
        return GuardAction::Allow;
    };
    let result = hooks.run::<T>(ctx, entity, action, transaction, rows).await;
    match result {
        GuardAction::Allow => {
            crate::signals::refresh_write_set_rows(ctx, entity, rows);
            GuardAction::Allow
        }
        GuardAction::Block(reason) => {
            crate::signals::discard_write_set_rows(ctx, entity);
            GuardAction::Block(reason)
        }
    }
}
