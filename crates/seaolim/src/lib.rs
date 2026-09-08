//! A layer over stock Seaography that closes the write-path gaps agents
//! fall into: hook composition that silently drops the save hook,
//! all-or-nothing mutation bundles, hook-free update/delete, unscoped
//! creates, row-less post-save events, and row hooks that cannot inspect a
//! complete mutation write set. Each gap becomes one call here.
//!
//! - [`ComposedHooks`] composes hook sets and forwards all five lifecycle
//!   methods. Upstream's `MultiLifecycleHooks` forwards four.
//! - [`register_generated_mutations`] publishes a chosen subset of the
//!   generated mutation bundle.
//! - [`register_hooked_create_one`], [`register_hooked_update`], and
//!   [`register_hooked_delete`] are SDL-identical replacements that run
//!   `before_active_model_save` per row and enforce `entity_filter` on
//!   create.
//! - [`Signals`] delivers post-commit events carrying the affected row.
//! - [`WriteSetHook`] inspects or amends all rows in one mutation before
//!   persistence. [`ComposedWriteSetHooks`] composes those rules.
//! - [`register_restricted_model_mutation`] and
//!   [`register_restricted_model_set_mutation`] keep an existing flattened
//!   field while handing its resolver, transaction, hooks, and persistence
//!   to the library — one row and a complete row set respectively.
//!
//! The README explains each trap with before/after code. The tests double
//! as usage examples. `docs/seaography-drf-parity.md` records what stock
//! Seaography did at the pinned version; ADR 0001 records why this is a
//! library and not a fork.

pub mod composed_hooks;
pub mod generated_mutations;
pub mod hooked_create_one_mutation;
pub mod hooked_delete_mutation;
pub mod hooked_update_mutation;
pub mod restricted_model_mutation;
pub mod restricted_model_set_mutation;
pub mod signals;
pub mod write_set_hooks;

pub use composed_hooks::ComposedHooks;
pub use generated_mutations::{register_generated_mutations, GeneratedMutations};
pub use hooked_create_one_mutation::register_hooked_create_one;
pub use hooked_delete_mutation::register_hooked_delete;
pub use hooked_update_mutation::register_hooked_update;
pub use restricted_model_mutation::{
    mutation_error, register_restricted_model_mutation, string_argument, ModelWrite,
    PreparedModelWrite, RestrictedModelMutation, RestrictedMutationField, WritePermit,
};
pub use restricted_model_set_mutation::{
    register_restricted_model_set_mutation, ModelSetWrite, PreparedModelSet,
    RestrictedModelSetMutation,
};
pub use signals::{SignalBuffer, Signals};
pub use write_set_hooks::{ComposedWriteSetHooks, WriteSetHook, WriteSetRow};
