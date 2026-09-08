//! This Module's rule about writes to its Model.
//!
//! A document locates one file under a root directory, so its `relPath` has
//! to stay relative and stay inside that root. Nothing in the migration can
//! say that — a `String` column accepts anything — and the caller's create
//! input goes straight to the Store, so the rule lives here, where it runs
//! inside the write's own transaction and rejecting it rolls the write back.

use module_host::{Decision, Txn, WriteAction, WriteHook, WriteSet};

use crate::entities::documents;

pub struct RelativePathRule;

impl WriteHook<documents::Entity> for RelativePathRule {
    fn check(
        &self,
        write: &mut WriteSet<'_, '_, documents::Entity>,
        _transaction: &Txn,
    ) -> Decision {
        if write.action() == WriteAction::Delete {
            return Decision::Allow;
        }
        for index in 0..write.len() {
            let Some(proposed) = write.proposed(index) else {
                continue;
            };
            let module_host::store::ActiveValue::Set(path) = &proposed.rel_path else {
                continue;
            };
            if let Some(reason) = rejection(path) {
                return Decision::reject(reason);
            }
        }
        Decision::Allow
    }
}

fn rejection(path: &str) -> Option<String> {
    if path.starts_with('/') {
        return Some(format!(
            "relPath \"{path}\" is absolute; a document is located relative to its rootDir"
        ));
    }
    if path.split('/').any(|segment| segment == "..") {
        return Some(format!("relPath \"{path}\" climbs out of its rootDir"));
    }
    if path.is_empty() {
        return Some("relPath is empty".to_owned());
    }
    None
}
