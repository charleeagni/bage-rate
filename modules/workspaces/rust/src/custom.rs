//! The reviewed generated-write selection and rules for workspaces.

use module_host::{CustomOps, Writes};

use crate::{entities::workspaces, rules::ImmutableWorkspaceIdentity};

pub fn register(ops: &mut CustomOps) {
    // Match Rustry's published write surface. Batch create stays absent, and
    // every published write passes through the Module's rule.
    ops.writes::<workspaces::Entity, workspaces::ActiveModel>(Writes::HOOKED);
    ops.write_hook::<workspaces::Entity, _>(ImmutableWorkspaceIdentity);
}
