//! The module-host seam: the only path through which a Module reaches the
//! App Schema, and the complete vocabulary it has for doing so.
//!
//! Per the ADR-0005 boundary this crate composes Modules and enforces their
//! namespaces; the write-path substrate underneath it is `seaolim`, and
//! neither of them knows how Modules are discovered or how the host assembles
//! them.
//!
//! # What a Module may say
//!
//! | Primitive | What it replaces |
//! | --- | --- |
//! | [`module_def!`] with `entities:` | generated CRUD from a migration |
//! | [`CustomOps::query`], [`mutation`](CustomOps::mutation), [`subscription`](CustomOps::subscription) | a hand-written resolver and a hand-wired endpoint |
//! | [`CustomOps::write_hook`] | validation scattered through resolvers |
//! | [`CustomOps::writes`] with [`Writes::HOOKED`] | overriding the write path to add behaviour |
//! | [`CustomOps::writes`] with [`Writes::without`] | leaving a write published that nobody may call |
//! | [`ModuleCtx::transaction`] | several writes that must land together |
//! | [`CustomOps::computed_field`] | a derived value mirrored in the client |
//! | [`CustomOps::escape_hatch`] | inventing a private seam when no primitive fits |
//!
//! The set is closed by decision. Everything registered through it lands in
//! the Module's mini-schema, so the prefix check and the per-module codegen
//! cover custom operations for free.
//!
//! # What a Module may not say
//!
//! A Module's hand-authored Rust names exactly two crate roots: `crate` and
//! `module_host`. `scripts/check-module-imports.mjs` enforces that, and
//! `scripts/check-module-dependencies.mjs` holds each Module's manifests to
//! an allowlist. The seal is a check rather than a compile error because the
//! generated entity directory still needs what its own codegen emits.

// The macros expand to `::module_host` paths, which have to resolve inside
// this crate too — its own tests are the first consumer.
extern crate self as module_host;

mod compose;
mod custom_ops;
mod error;
mod events;
mod hooks;
pub mod migration;
mod module_ctx;
mod module_def;
mod prefix_check;
pub mod store;
mod writes;

#[doc(hidden)]
pub mod __private;

pub use compose::compose;
pub use custom_ops::CustomOps;
pub use error::{ComposeError, Error, Result};
pub use hooks::{Decision, WriteAction, WriteHook, WriteSet};
pub use module_ctx::ModuleCtx;
pub use module_def::ModuleDef;
pub use module_host_macros::{custom_fields, output};
pub use store::{Store, Txn};
pub use writes::{Write, Writes};
