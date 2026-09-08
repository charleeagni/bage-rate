//! DRF-style post-save signals with row payloads, built from two rc.9 hooks.
//!
//! `entity_watch` alone is a signal without an instance: it delivers only
//! the entity name and action. This module pairs it with
//! `before_active_model_save`, which does see the row: the save hook
//! captures a clone into a per-request [`SignalBuffer`], and `entity_watch`
//! drains the buffer after commit and calls the registered handlers with
//! the captured rows.
//!
//! Payload coverage follows whoever serves the mutation: upstream's
//! generated bundle sends the save hook only on create, so update/delete
//! signals fire with `row: None` there. Served by this crate's hooked
//! builders, the save hook runs per row on update and delete too, and
//! signals carry the row on all three paths (`tests/signals.rs`).
//! Hooked builders refresh captured rows after [`crate::WriteSetHook`]
//! amendments, so handlers receive the row that is actually persisted.
//!
//! Callers must attach a fresh `SignalBuffer` to each request —
//! `Request::new(query).data(SignalBuffer::default())` — so concurrent
//! requests cannot see each other's rows. Without one, signals still fire,
//! payload-less.

use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use sea_orm::entity::prelude::async_trait;
use seaography::{
    async_graphql::dynamic::ResolverContext, GuardAction, LifecycleHooksInterface, OperationType,
};

type CapturedRow = Arc<dyn Any + Send + Sync>;
type CaptureFn = Arc<dyn Fn(&dyn Any) -> Option<CapturedRow> + Send + Sync>;
type HandlerFn = Arc<
    dyn for<'a> Fn(
            &ResolverContext<'a>,
            OperationType,
            Option<CapturedRow>,
        ) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

use crate::WriteSetRow;

/// Per-request store carrying rows from the save hook to `entity_watch`.
/// Attach one to each GraphQL request via `.data(SignalBuffer::default())`.
#[derive(Default)]
pub struct SignalBuffer {
    rows: Mutex<Vec<BufferedRow>>,
}

struct BufferedRow {
    entity: String,
    row: CapturedRow,
    capture: CaptureFn,
}

impl SignalBuffer {
    fn capture(&self, entity: &str, row: CapturedRow, capture: CaptureFn) {
        self.rows
            .lock()
            .expect("signal buffer poisoned")
            .push(BufferedRow {
                entity: entity.to_owned(),
                row,
                capture,
            });
    }

    fn refresh(&self, entity: &str, write_set: &[WriteSetRow<'_>]) {
        let mut rows = self.rows.lock().expect("signal buffer poisoned");
        let buffered = rows.iter_mut().filter(|row| row.entity == entity);
        for (buffered, amended) in buffered.zip(write_set) {
            if let Some(row) = (buffered.capture)(amended.active_model()) {
                buffered.row = row;
            }
        }
    }

    fn discard(&self, entity: &str) {
        self.rows
            .lock()
            .expect("signal buffer poisoned")
            .retain(|row| row.entity != entity);
    }

    fn drain(&self, entity: &str) -> Vec<CapturedRow> {
        let mut rows = self.rows.lock().expect("signal buffer poisoned");
        let (matching, rest) = std::mem::take(&mut *rows)
            .into_iter()
            .partition(|row| row.entity == entity);
        *rows = rest;
        matching.into_iter().map(|buffered| buffered.row).collect()
    }
}

pub(crate) fn refresh_write_set_rows(
    ctx: &ResolverContext<'_>,
    entity: &str,
    rows: &[WriteSetRow<'_>],
) {
    if let Ok(buffer) = ctx.data::<SignalBuffer>() {
        buffer.refresh(entity, rows);
    }
}

pub(crate) fn discard_write_set_rows(ctx: &ResolverContext<'_>, entity: &str) {
    if let Ok(buffer) = ctx.data::<SignalBuffer>() {
        buffer.discard(entity);
    }
}

struct Subscription {
    entity: String,
    capture: CaptureFn,
    handler: HandlerFn,
}

/// A hook set delivering post-commit signals. Register it inside
/// [`crate::ComposedHooks`] like any other hook set.
#[derive(Default)]
pub struct Signals {
    subscriptions: Vec<Subscription>,
}

impl Signals {
    /// Subscribe to post-mutation events for one entity. `A` is the
    /// entity's `ActiveModel`; on the create path the handler receives the
    /// row as saved (after earlier hook sets mutated it), on other paths it
    /// receives `None`.
    pub fn on<A, F, Fut>(mut self, entity: &str, handler: F) -> Self
    where
        A: Clone + Send + Sync + 'static,
        F: Fn(OperationType, Option<A>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler = Arc::new(handler);
        self.subscriptions.push(Subscription {
            entity: entity.to_owned(),
            capture: Arc::new(|row: &dyn Any| {
                row.downcast_ref::<A>()
                    .cloned()
                    .map(|row| Arc::new(row) as CapturedRow)
            }),
            handler: Arc::new(move |_ctx, action, row| {
                let row = row.and_then(|row| row.downcast_ref::<A>().cloned());
                let handler = handler.clone();
                Box::pin(async move { handler(action, row).await })
            }),
        });
        self
    }

    /// Subscribe with access to request schema data. The callback must clone
    /// anything it needs from `ctx` before returning its owned future.
    pub fn on_with_context<A, F, Fut>(mut self, entity: &str, handler: F) -> Self
    where
        A: Clone + Send + Sync + 'static,
        F: for<'a> Fn(&ResolverContext<'a>, OperationType, Option<A>) -> Fut
            + Send
            + Sync
            + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler = Arc::new(handler);
        self.subscriptions.push(Subscription {
            entity: entity.to_owned(),
            capture: Arc::new(|row: &dyn Any| {
                row.downcast_ref::<A>()
                    .cloned()
                    .map(|row| Arc::new(row) as CapturedRow)
            }),
            handler: Arc::new(move |ctx, action, row| {
                let row = row.and_then(|row| row.downcast_ref::<A>().cloned());
                Box::pin(handler(ctx, action, row))
            }),
        });
        self
    }
}

#[async_trait::async_trait]
impl LifecycleHooksInterface for Signals {
    fn before_active_model_save(
        &self,
        ctx: &ResolverContext,
        entity: &str,
        _action: OperationType,
        active_model: &mut dyn Any,
    ) -> GuardAction {
        if let Ok(buffer) = ctx.data::<SignalBuffer>() {
            for subscription in self.subscriptions.iter().filter(|s| s.entity == entity) {
                if let Some(row) = (subscription.capture)(active_model) {
                    buffer.capture(entity, row, subscription.capture.clone());
                    break;
                }
            }
        }
        GuardAction::Allow
    }

    async fn entity_watch(&self, ctx: &ResolverContext, entity: &str, action: OperationType) {
        let rows = ctx
            .data::<SignalBuffer>()
            .map(|buffer| buffer.drain(entity))
            .unwrap_or_default();
        for subscription in self.subscriptions.iter().filter(|s| s.entity == entity) {
            if rows.is_empty() {
                (subscription.handler)(ctx, action, None).await;
            } else {
                for row in &rows {
                    (subscription.handler)(ctx, action, Some(row.clone())).await;
                }
            }
        }
    }
}
