//! Per-operation selection of a Model's generated writes.
//!
//! Registering a Model publishes create-one, create-batch, update, and
//! delete together or not at all. That is the wrong granularity as soon as a
//! Module has a rule about one of them: the choice is between exposing a
//! write nobody reviewed and hand-writing the resolver. [`Writes`] makes it
//! a declaration instead — per operation, and per operation whether it runs
//! the Module's hooks.

/// One of the four generated writes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Write {
    CreateOne,
    CreateBatch,
    Update,
    Delete,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Mode {
    #[default]
    Off,
    Generated,
    Hooked,
}

/// The write surface one Model publishes.
///
/// Start from a constant and narrow it. Whatever is left `Off` is absent
/// from the SDL, so dropping `Update` really does mean a custom operation is
/// the only path to those columns.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Writes {
    pub(crate) create_one: Mode,
    pub(crate) create_batch: Mode,
    pub(crate) update: Mode,
    pub(crate) delete: Mode,
}

impl Writes {
    /// No writes at all: every mutation on this Model is a custom
    /// operation.
    pub const NONE: Self = Self {
        create_one: Mode::Off,
        create_batch: Mode::Off,
        update: Mode::Off,
        delete: Mode::Off,
    };

    /// What a Model publishes when it says nothing: all four generated
    /// writes, none of which runs a hook.
    pub const GENERATED: Self = Self {
        create_one: Mode::Generated,
        create_batch: Mode::Generated,
        update: Mode::Generated,
        delete: Mode::Generated,
    };

    /// Create-one, update, and delete with the Module's hooks running, and
    /// the SDL unchanged from `GENERATED` minus create-batch.
    ///
    /// Create-batch is left off deliberately: the substrate has no hooked
    /// batch create, so publishing it here would put one write beside its
    /// siblings that quietly skips every rule. A Module that wants it back
    /// says so with `.generated(Write::CreateBatch)` and accepts that.
    pub const HOOKED: Self = Self {
        create_one: Mode::Hooked,
        create_batch: Mode::Off,
        update: Mode::Hooked,
        delete: Mode::Hooked,
    };

    /// Publish this write as Seaography generates it, running no hooks.
    pub const fn generated(self, write: Write) -> Self {
        self.set(write, Mode::Generated)
    }

    /// Publish this write with the Module's hooks running. The field's SDL
    /// is identical to the generated one.
    pub const fn hooked(self, write: Write) -> Self {
        self.set(write, Mode::Hooked)
    }

    /// Drop this write from the contract.
    pub const fn without(self, write: Write) -> Self {
        self.set(write, Mode::Off)
    }

    const fn set(mut self, write: Write, mode: Mode) -> Self {
        match write {
            Write::CreateOne => self.create_one = mode,
            Write::CreateBatch => self.create_batch = mode,
            Write::Update => self.update = mode,
            Write::Delete => self.delete = mode,
        }
        self
    }

    pub(crate) fn needs_insert_input(self) -> bool {
        !matches!(self.create_one, Mode::Off) || !matches!(self.create_batch, Mode::Off)
    }

    pub(crate) fn needs_update_input(self) -> bool {
        !matches!(self.update, Mode::Off)
    }

    pub(crate) fn generated_selection(self) -> seaolim::GeneratedMutations {
        seaolim::GeneratedMutations {
            create_one: matches!(self.create_one, Mode::Generated),
            create_batch: matches!(self.create_batch, Mode::Generated),
            update: matches!(self.update, Mode::Generated),
            delete: matches!(self.delete, Mode::Generated),
        }
    }
}
