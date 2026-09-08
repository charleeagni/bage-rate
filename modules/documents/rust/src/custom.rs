//! Everything this Module contributes beyond the CRUD its migration
//! generates, in the one place a reviewer reads.

use module_host::{CustomOps, Write, Writes};

use crate::{
    check::{DocumentsSaveCheck, DocumentsSaveQueries},
    entities::documents,
    rules::RelativePathRule,
    save::{DocumentsSaveMutations, DocumentsSaveOutcome, DocumentsSavedEvent},
};

pub fn register(ops: &mut CustomOps) {
    // The write surface. Update is dropped outright, so `documentsSave` is
    // the only path to a document's digest and the compare-and-swap cannot
    // be bypassed by a caller that would rather not check. Create and delete
    // stay, and run this Module's rules.
    ops.writes::<documents::Entity, documents::ActiveModel>(Writes::HOOKED.without(Write::Update));
    ops.write_hook::<documents::Entity, _>(RelativePathRule);

    // The compare-and-swap itself, and the shape its conflict comes back as.
    ops.output::<DocumentsSaveOutcome>();
    ops.mutation::<DocumentsSaveMutations>();

    // Its read half: what a save would do, asked without doing it.
    ops.output::<DocumentsSaveCheck>();
    ops.query::<DocumentsSaveQueries>();

    // Live saves. The event is published from inside the operation, after
    // its transaction commits.
    ops.subscription::<DocumentsSavedEvent>("documentsSaved");

    // A read-only field beside the columns, derived from the row: a document
    // that has never been saved has no digest yet.
    ops.computed_field::<documents::Entity, bool, _>("neverSaved", |document| {
        document.content_digest.is_none()
    });
}
