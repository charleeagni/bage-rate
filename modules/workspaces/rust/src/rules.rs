//! Rules that preserve Rustry's authored Workspace identity and creation time.

use module_host::{Decision, Txn, WriteAction, WriteHook, WriteSet};

use crate::entities::workspaces;

pub struct ImmutableWorkspaceIdentity;

impl WriteHook<workspaces::Entity> for ImmutableWorkspaceIdentity {
    fn check(
        &self,
        write: &mut WriteSet<'_, '_, workspaces::Entity>,
        _transaction: &Txn,
    ) -> Decision {
        if write.action() != WriteAction::Update {
            return Decision::Allow;
        }

        for index in 0..write.len() {
            let Some(current) = write.current(index) else {
                continue;
            };
            let current_id = current.id.clone();
            let current_created_at = current.created_at;
            let Some(proposed) = write.proposed(index) else {
                continue;
            };

            if matches!(
                &proposed.id,
                module_host::store::ActiveValue::Set(id) if id != &current_id
            ) {
                return Decision::reject("a workspace update cannot change id");
            }
            if matches!(
                &proposed.created_at,
                module_host::store::ActiveValue::Set(created_at)
                    if created_at != &current_created_at
            ) {
                return Decision::reject("a workspace update cannot change createdAt");
            }
        }

        Decision::Allow
    }
}
