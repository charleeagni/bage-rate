use std::{future::Future, pin::Pin};

use sea_orm::TransactionTrait;
use seaography::async_graphql::Context;

use crate::{
    error::{Error, Result},
    store::{Store, Txn},
};

/// Everything a Module's own code may reach at request time.
///
/// A resolver receives one of these instead of the GraphQL context, which is
/// the whole point: the Store arrives borrowed, the transaction primitive is
/// the only way to make several writes atomic, and nothing else that the
/// host attached to the schema is visible from here. Any other effect a
/// Module ever needs — filesystem, network, a process — arrives the same
/// way, as a capability on this type, never as a free import inside the
/// Module.
pub struct ModuleCtx<'a> {
    context: &'a Context<'a>,
}

impl<'a> ModuleCtx<'a> {
    pub(crate) fn new(context: &'a Context<'a>) -> Self {
        Self { context }
    }

    /// The Store connection, for reads and single writes that need no
    /// transaction.
    pub fn store(&self) -> Result<&'a Store> {
        self.context.data::<Store>()
    }

    /// Run `work` against one transaction over this Module's own Models.
    ///
    /// The transaction commits when `work` returns `Ok` and rolls back when
    /// it returns `Err`, so a conflict reported as data still has to be
    /// returned through `Ok` — an `Err` undoes the write that discovered it.
    ///
    /// Atomicity stops at the process: a GraphQL request composing several
    /// root mutations runs them one after another, so anything that must be
    /// atomic is one operation, not a client-side sequence.
    pub async fn transaction<F, T>(&self, work: F) -> Result<T>
    where
        F: for<'t> FnOnce(&'t Txn) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 't>> + Send,
    {
        let transaction = self.store()?.begin().await?;
        match work(&transaction).await {
            Ok(value) => {
                transaction.commit().await?;
                Ok(value)
            }
            // The caller's error leads, because it is the one they can act
            // on. A rollback that also failed is appended rather than
            // replacing it or vanishing.
            Err(error) => match transaction.rollback().await {
                Ok(()) => Err(error),
                Err(rollback) => Err(Error::new(format!(
                    "{}; the transaction also failed to roll back: {rollback}",
                    error.message
                ))),
            },
        }
    }

    /// Publish one event to every open subscription over `T`.
    ///
    /// Delivery is best-effort and live-only: an event raised while nobody
    /// is subscribed is dropped, which is what makes this a notification
    /// channel rather than a second Store.
    pub fn publish<T>(&self, event: T) -> Result<()>
    where
        T: Clone + Send + Sync + 'static,
    {
        self.context
            .data::<crate::events::Events<T>>()?
            .publish(event);
        Ok(())
    }
}
