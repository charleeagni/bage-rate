//! The source adapter behind a Module's subscriptions.
//!
//! One broadcast channel per event type, attached to the schema by
//! [`compose`](crate::compose) for every event type a Module declared a
//! subscription over. A Module publishes through
//! [`ModuleCtx::publish`](crate::ModuleCtx::publish) and never sees the
//! channel.

use futures_util::{stream, Stream, StreamExt};
use seaography::async_graphql::dynamic::{FieldValue, ResolverContext, SubscriptionFieldFuture};
use tokio::sync::broadcast::{self, error::RecvError};

/// How many events one slow subscriber may fall behind before it starts
/// missing them. Live data is a notification channel, not a queue: a
/// subscriber that cannot keep up re-reads through a query.
const BACKLOG: usize = 64;

pub struct Events<T> {
    sender: broadcast::Sender<T>,
}

impl<T> Default for Events<T>
where
    T: Clone + Send + 'static,
{
    fn default() -> Self {
        Self {
            sender: broadcast::Sender::new(BACKLOG),
        }
    }
}

impl<T> Events<T>
where
    T: Clone + Send + 'static,
{
    pub(crate) fn publish(&self, event: T) {
        // An error here means nobody is listening, which is not a failure.
        let _ = self.sender.send(event);
    }

    pub(crate) fn stream(&self) -> impl Stream<Item = T> + Send + 'static {
        stream::unfold(self.sender.subscribe(), |mut receiver| async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => return Some((event, receiver)),
                    // A lagging subscriber skips to the newest events rather
                    // than ending its subscription.
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => return None,
                }
            }
        })
    }
}

/// Resolve one subscription field: hand the caller the stream of events
/// published for `T` since it subscribed.
///
/// This is a named function rather than a closure so that its elided
/// lifetime gives the higher-ranked signature a subscription resolver needs;
/// a closure would pin the stream to `'static` and fail to unify.
pub(crate) fn subscribe<T>(ctx: ResolverContext<'_>) -> SubscriptionFieldFuture<'_>
where
    T: Clone + Send + Sync + 'static,
{
    SubscriptionFieldFuture::new(async move {
        let events = ctx.data::<Events<T>>()?;
        Ok(events
            .stream()
            .map(|event| crate::Result::Ok(FieldValue::owned_any(event))))
    })
}
