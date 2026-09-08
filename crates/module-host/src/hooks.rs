//! The write-hook primitive: a Module's rule about a write, checked inside
//! the write's own transaction.
//!
//! A hook runs once per mutation over the mutation's complete write set,
//! after the rows have been fetched and the proposed values folded in, and
//! before anything is persisted. That position is what makes one hook able
//! to do the work Django splits across `pre_save` signals and a serializer's
//! `validate()`: it sees each row's current and proposed state together, it
//! sees siblings, and rejecting it rolls the whole mutation back.
//!
//! Hooks only run for writes a Module registered as
//! [`hooked`](crate::Writes::hooked) — generated Seaography mutations have
//! no hook point at all, which is the gap the substrate closes.

use std::marker::PhantomData;

use sea_orm::{entity::prelude::async_trait, DatabaseTransaction, EntityTrait};
use seaography::{async_graphql::dynamic::ResolverContext, GuardAction, OperationType};

use crate::store::Txn;

/// Which write a hook is looking at.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteAction {
    Create,
    Update,
    Delete,
}

/// A hook's answer. `Reject` aborts the mutation, rolls its transaction
/// back, and returns the reason to the caller as a GraphQL error.
#[derive(Debug)]
pub enum Decision {
    Allow,
    Reject(String),
}

impl Decision {
    pub fn reject(reason: impl Into<String>) -> Self {
        Self::Reject(reason.into())
    }
}

/// Every row one mutation is about to write, addressed by index.
///
/// `current` is the row as the Store holds it and is absent on a create;
/// `proposed` is the row as the mutation would leave it, and amending it
/// there amends what gets persisted.
pub struct WriteSet<'rows, 'row, E>
where
    E: EntityTrait,
{
    action: WriteAction,
    rows: &'rows mut [seaolim::WriteSetRow<'row>],
    entity: PhantomData<E>,
}

impl<E> WriteSet<'_, '_, E>
where
    E: EntityTrait,
    E::Model: 'static,
    E::ActiveModel: 'static,
{
    pub fn action(&self) -> WriteAction {
        self.action
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The row as the Store holds it. `None` for a create, which has no
    /// prior row.
    pub fn current(&self, index: usize) -> Option<&E::Model> {
        self.rows.get(index)?.old_model::<E::Model>()
    }

    /// The row as this mutation proposes to leave it. Amending it here
    /// amends what is persisted.
    pub fn proposed(&mut self, index: usize) -> Option<&mut E::ActiveModel> {
        self.rows
            .get_mut(index)?
            .active_model_mut::<E::ActiveModel>()
    }
}

/// One Module rule about writes to one of its Models.
pub trait WriteHook<E>: Send + Sync + 'static
where
    E: EntityTrait,
{
    /// Inspect, amend, or reject the write set. `transaction` is the
    /// mutation's own open transaction, so a rule that depends on rows this
    /// mutation is not writing reads them through it and sees a consistent
    /// picture.
    fn check(&self, write: &mut WriteSet<'_, '_, E>, transaction: &Txn) -> Decision;
}

/// Adapts one Module hook onto the substrate's set-level hook.
pub(crate) struct HookAdapter<E, H> {
    pub(crate) hook: H,
    pub(crate) entity: PhantomData<E>,
}

#[async_trait::async_trait]
impl<E, H> seaolim::WriteSetHook for HookAdapter<E, H>
where
    E: EntityTrait + Send + Sync + 'static,
    E::Model: 'static,
    E::ActiveModel: 'static,
    H: WriteHook<E>,
{
    async fn before_write_set(
        &self,
        _ctx: &ResolverContext,
        _entity: &str,
        action: OperationType,
        transaction: &DatabaseTransaction,
        rows: &mut [seaolim::WriteSetRow<'_>],
    ) -> GuardAction {
        let action = match action {
            OperationType::Create => WriteAction::Create,
            OperationType::Update => WriteAction::Update,
            OperationType::Delete => WriteAction::Delete,
            // The hooked builders only ever run this for a write.
            OperationType::Read => return GuardAction::Allow,
        };
        let mut write = WriteSet::<E> {
            action,
            rows,
            entity: PhantomData,
        };
        match self.hook.check(&mut write, transaction) {
            Decision::Allow => GuardAction::Allow,
            Decision::Reject(reason) => GuardAction::Block(Some(reason)),
        }
    }
}
