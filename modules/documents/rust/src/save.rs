//! The one write that generated CRUD cannot express: a compare-and-swap on a
//! document's content digest, whose conflict is data rather than an error.
//!
//! `documentsUpdate` could write the digest, but it cannot condition the
//! write on what the digest currently is, and an optimistic filter on
//! `contentDigest` cannot tell "someone else saved first" apart from "the
//! document is gone". Both distinctions are the whole point of the
//! operation, so it is a custom mutation — declared here, registered in
//! `custom.rs`, and prefix-checked like every generated field.

use module_host::{custom_fields, output, store::*, Error, ModuleCtx, Result};

use crate::entities::documents;

/// What a save did. `stale` carries the conflict back as data: the caller
/// learns which digest the Store actually holds and can re-read, merge, and
/// try again without parsing an error string.
#[output]
pub struct DocumentsSaveOutcome {
    pub document_id: String,
    pub digest: String,
    pub saved: bool,
    pub stale: bool,
}

/// One document saved. Published after the transaction commits, so a
/// subscriber never sees a save that was rolled back.
#[output]
#[derive(Clone)]
pub struct DocumentsSavedEvent {
    pub document_id: String,
    pub digest: String,
    pub saved_at: String,
}

pub struct DocumentsSaveMutations;

#[custom_fields]
impl DocumentsSaveMutations {
    /// Write `digest` onto the document, but only if it still holds
    /// `expected_digest`.
    async fn documents_save(
        ctx: &ModuleCtx<'_>,
        document_id: String,
        expected_digest: String,
        digest: String,
        saved_at: String,
    ) -> Result<DocumentsSaveOutcome> {
        let outcome = ctx
            .transaction(|transaction| {
                let document_id = document_id.clone();
                let digest = digest.clone();
                let saved_at = saved_at.clone();
                Box::pin(async move {
                    let Some(document) = documents::Entity::find_by_id(document_id.clone())
                        .one(transaction)
                        .await?
                    else {
                        // Not a conflict the caller can resolve by
                        // re-reading, so this one really is an error.
                        return Err(Error::new(format!("no document \"{document_id}\"")));
                    };

                    let held = document.content_digest.clone().unwrap_or_default();
                    if held != expected_digest {
                        // Reported through `Ok`: an `Err` would roll the
                        // transaction back, which is right, but it would also
                        // throw away the digest the caller needs.
                        return Ok(DocumentsSaveOutcome {
                            document_id,
                            digest: held,
                            saved: false,
                            stale: true,
                        });
                    }

                    let mut document = document.into_active_model();
                    document.content_digest = Set(Some(digest.clone()));
                    document.updated_at = Set(saved_at);
                    document.update(transaction).await?;

                    Ok(DocumentsSaveOutcome {
                        document_id,
                        digest,
                        saved: true,
                        stale: false,
                    })
                })
            })
            .await?;

        if outcome.saved {
            ctx.publish(DocumentsSavedEvent {
                document_id: outcome.document_id.clone(),
                digest: outcome.digest.clone(),
                saved_at,
            })?;
        }
        Ok(outcome)
    }
}
