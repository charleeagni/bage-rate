//! The pressure valve, and the handle the guard reads it by.
//!
//! An escape hatch is a mutation like any other once it is registered — same
//! root, same prefix check, same typed output. What makes it a distinct
//! primitive is that declaring one is a separate call, so the count is a
//! number the seam can hand to `scripts/check-escape-hatches.mjs` rather than
//! something a reviewer has to notice.

mod support;

use module_host::compose;
use support::{context, database};

mod ledger {
    use module_host::{custom_fields, migration::*, output, CustomOps, ModuleCtx, Result};

    #[output]
    pub struct LedgerRebalanceOutcome {
        pub moved: i32,
    }

    pub struct LedgerQueries;

    #[custom_fields]
    impl LedgerQueries {
        async fn ledger_total(_ctx: &ModuleCtx<'_>) -> Result<i32> {
            Ok(0)
        }
    }

    pub struct LedgerRebalance;

    #[custom_fields]
    impl LedgerRebalance {
        async fn ledger_rebalance(
            _ctx: &ModuleCtx<'_>,
            as_of: String,
        ) -> Result<LedgerRebalanceOutcome> {
            Ok(LedgerRebalanceOutcome {
                moved: as_of.len() as i32,
            })
        }
    }

    pub fn register(ops: &mut CustomOps) {
        ops.query::<LedgerQueries>();
        ops.output::<LedgerRebalanceOutcome>();
        ops.escape_hatch::<LedgerRebalance>("ledgerRebalance");
    }

    pub struct Migrator;

    #[async_trait::async_trait]
    impl MigratorTrait for Migrator {
        fn migrations() -> Vec<Box<dyn MigrationTrait>> {
            vec![]
        }
    }

    module_host::module_def! {
        name: "ledger",
        migrations: Migrator,
        custom: register,
    }
}

#[tokio::test]
async fn a_declared_escape_hatch_is_reported_by_name() {
    assert_eq!(
        ledger::module_def().escape_hatches(),
        vec!["ledgerRebalance"],
    );
}

#[tokio::test]
async fn a_module_that_fits_the_primitives_declares_none() {
    assert!(projects_module::module_def().escape_hatches().is_empty());
    assert!(documents_module::module_def().escape_hatches().is_empty());
}

// The hatch is still inside the framework: the output type is concrete, the
// root field carries the prefix, and the field lands in the same Mutation
// root as everything else.
#[tokio::test]
async fn an_escape_hatch_is_prefix_checked_and_typed_like_any_other_mutation() {
    let schema = compose(context(), database().await, &[ledger::module_def()])
        .expect("compose the ledger module");
    let sdl = schema.sdl();

    assert!(sdl.contains("ledgerRebalance(asOf: String!): LedgerRebalanceOutcome!"));
    assert!(sdl.contains("type LedgerRebalanceOutcome {"));
}
