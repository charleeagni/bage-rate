//! The read half of the compare-and-swap: whether a save would land.
//!
//! The generated `documents` query already returns the row, so a caller
//! could fetch it and compare digests itself. The rule for what counts as up
//! to date would then live in every caller — an absent digest is not equal
//! to anything, and a document nobody registered is not the same as one
//! whose digest differs — each copy free to drift from the mutation that
//! enforces the real thing. This states it once, beside the write it
//! predicts.

use module_host::{custom_fields, output, store::*, ModuleCtx, Result};

use crate::entities::documents;

/// What a save holding `digest` would do, without doing it.
#[output]
pub struct DocumentsSaveCheck {
    pub document_id: String,
    /// Null while the document has never been saved, which is a different
    /// answer from the empty string and worth keeping distinct.
    pub held: Option<String>,
    pub known: bool,
    pub up_to_date: bool,
}

pub struct DocumentsSaveQueries;

#[custom_fields]
impl DocumentsSaveQueries {
    /// Answer whether `documentsSave` would write or report a conflict.
    async fn documents_save_check(
        ctx: &ModuleCtx<'_>,
        document_id: String,
        digest: String,
    ) -> Result<DocumentsSaveCheck> {
        // One read, so `store()` rather than `transaction()`: there is
        // nothing here for a transaction to hold together, and the answer is
        // advisory the moment it is returned. The save re-reads inside its
        // own transaction, which is what makes it safe to act on this.
        let document = documents::Entity::find_by_id(document_id.clone())
            .one(ctx.store()?)
            .await?;

        let known = document.is_some();
        let held = document.and_then(|document| document.content_digest);
        Ok(DocumentsSaveCheck {
            up_to_date: known && held.as_deref().unwrap_or_default() == digest,
            document_id,
            held,
            known,
        })
    }
}
