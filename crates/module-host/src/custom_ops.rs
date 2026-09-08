//! The closed vocabulary a Module has for everything beyond generated CRUD.
//!
//! `module_def!`'s `custom:` function receives one of these and registers
//! against it. Everything registered here lands in the Module's own
//! mini-schema, so the prefix check and the per-module codegen cover custom
//! operations with no new machinery: a root field or type outside the
//! Module's namespace fails `generate`, and a Caller Operation naming
//! another Module's custom types fails codegen.
//!
//! The set is closed by decision. An application that needs something
//! outside it writes a host-level exception with the written-exception
//! discipline; recurring exceptions are the signal to grow this file — not
//! to grow the Module.

use std::marker::PhantomData;

use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel};
use seaography::{
    async_graphql::dynamic::{Field, SchemaBuilder, SubscriptionField},
    Builder, BuilderContext, CustomFields, CustomOutputObject, CustomOutputType,
    EntityInputBuilder, EntityObjectBuilder, GqlScalarValueType,
};

use crate::{
    events::Events,
    hooks::{HookAdapter, WriteHook},
    writes::{Mode, Writes},
};

type Registration = Box<dyn Fn(&mut Builder)>;
type ComputedField = Box<dyn Fn(&'static BuilderContext) -> (String, Field)>;
type DroppedInputs = Box<dyn Fn(&'static BuilderContext) -> Vec<String>>;
type HookRegistration =
    Box<dyn FnOnce(seaolim::ComposedWriteSetHooks) -> seaolim::ComposedWriteSetHooks>;
type SchemaData = Box<dyn Fn(SchemaBuilder) -> SchemaBuilder>;

/// What one Module registers beyond the CRUD its migrations generate.
#[derive(Default)]
pub struct CustomOps {
    selects_writes: bool,
    escape_hatches: Vec<&'static str>,
    registrations: Vec<Registration>,
    computed_fields: Vec<ComputedField>,
    dropped_inputs: Vec<DroppedInputs>,
    hooks: Vec<HookRegistration>,
    schema_data: Vec<SchemaData>,
}

impl CustomOps {
    /// A custom output object. Registering the type is separate from
    /// returning it, so one output can serve several operations.
    pub fn output<T>(&mut self)
    where
        T: CustomOutputObject + 'static,
    {
        self.registrations
            .push(Box::new(|builder| builder.register_custom_output::<T>()));
    }

    /// Every `#[custom_fields]` function on `T` becomes a Query root field.
    pub fn query<T>(&mut self)
    where
        T: CustomFields + 'static,
    {
        self.registrations
            .push(Box::new(|builder| builder.register_custom_query::<T>()));
    }

    /// Every `#[custom_fields]` function on `T` becomes a Mutation root
    /// field.
    pub fn mutation<T>(&mut self)
    where
        T: CustomFields + 'static,
    {
        self.registrations
            .push(Box::new(|builder| builder.register_custom_mutation::<T>()));
    }

    /// Declare an escape-hatch mutation.
    ///
    /// Reach for this only when no primitive fits: many reads, branching on
    /// what was read, and a write whose shape follows from them. It is a
    /// separate call from [`mutation`](Self::mutation) precisely so that its
    /// use is visible in the one place a reviewer already reads, and
    /// `scripts/check-escape-hatches.mjs` fails verify when a declared hatch
    /// has no written exception or when a Module declares too many.
    ///
    /// `declared_as` is the camel-cased root field name, which is also the
    /// name the exception record is filed under.
    pub fn escape_hatch<T>(&mut self, declared_as: &'static str)
    where
        T: CustomFields + 'static,
    {
        self.escape_hatches.push(declared_as);
        self.mutation::<T>();
    }

    /// Stream `T` to subscribers. Events reach the channel through
    /// [`ModuleCtx::publish`](crate::ModuleCtx::publish); the channel itself
    /// is never visible to the Module.
    pub fn subscription<T>(&mut self, name: &'static str)
    where
        T: CustomOutputObject + CustomOutputType + Clone + Send + Sync + 'static,
    {
        self.output::<T>();
        self.schema_data
            .push(Box::new(|schema| schema.data(Events::<T>::default())));
        self.registrations.push(Box::new(move |builder| {
            let type_ref = T::gql_output_type_ref(builder.context);
            builder.register_subscription_field(SubscriptionField::new(
                name,
                type_ref,
                crate::events::subscribe::<T>,
            ));
        }));
    }

    /// A read-only field on a Model, computed from the row.
    ///
    /// This is the sanctioned way to publish something derived: the column
    /// stays as the migration wrote it, and the derived value is one more
    /// field beside it. The field name carries the Module's prefix only
    /// because the Model's type name already does.
    pub fn computed_field<E, R, F>(&mut self, name: &'static str, compute: F)
    where
        E: EntityTrait + 'static,
        E::Model: Sync + 'static,
        R: GqlScalarValueType + Send + 'static,
        F: Fn(&E::Model) -> R + Send + Sync + Copy + 'static,
    {
        self.computed_fields.push(Box::new(move |context| {
            let object_name = EntityObjectBuilder { context }.type_name::<E>();
            let field = Field::new(name, R::gql_output_type_ref(context), move |ctx| {
                seaography::async_graphql::dynamic::FieldFuture::new(async move {
                    let row = seaography::try_downcast_ref::<E::Model>(ctx.parent_value)?;
                    Ok(compute(row).gql_field_value(context))
                })
            });
            (object_name, field)
        }));
    }

    /// Which of a Model's generated writes to publish, and which of those
    /// run the Module's hooks.
    ///
    /// Declaring this for any Model replaces the whole Module's generated
    /// write surface, so a Module that selects writes selects them for every
    /// Model it owns. The SDL diff is where an omission shows up.
    pub fn writes<E, A>(&mut self, writes: Writes)
    where
        E: EntityTrait + 'static,
        E::Model: Sync + IntoActiveModel<A>,
        A: ActiveModelTrait<Entity = E> + sea_orm::ActiveModelBehavior + Send + 'static,
    {
        self.selects_writes = true;
        self.registrations.push(Box::new(move |builder| {
            seaolim::register_generated_mutations::<E, A>(builder, writes.generated_selection());
            if matches!(writes.create_one, Mode::Hooked) {
                seaolim::register_hooked_create_one::<E, A>(builder);
            }
            if matches!(writes.update, Mode::Hooked) {
                seaolim::register_hooked_update::<E, A>(builder);
            }
            if matches!(writes.delete, Mode::Hooked) {
                seaolim::register_hooked_delete::<E, A>(builder);
            }
        }));
        self.dropped_inputs.push(Box::new(move |context| {
            let inputs = EntityInputBuilder { context };
            let mut dropped = Vec::new();
            if !writes.needs_insert_input() {
                dropped.push(inputs.insert_type_name::<E>());
            }
            if !writes.needs_update_input() {
                dropped.push(inputs.update_type_name::<E>());
            }
            dropped
        }));
    }

    /// A rule this Module enforces over writes to one of its Models.
    ///
    /// Hooks run only for writes registered as
    /// [`Writes::hooked`](crate::Writes::hooked); a generated write has no
    /// hook point, so a rule attached to a Model whose writes are all
    /// generated never fires. That pairing is deliberate and visible in one
    /// place: both calls sit in the same `custom` function.
    pub fn write_hook<E, H>(&mut self, hook: H)
    where
        E: EntityTrait + Send + Sync + 'static,
        E::Model: 'static,
        E::ActiveModel: 'static,
        H: WriteHook<E>,
    {
        self.hooks.push(Box::new(move |hooks| {
            hooks.add_for::<E>(HookAdapter::<E, H> {
                hook,
                entity: PhantomData,
            })
        }));
    }

    /// The escape-hatch operations this Module declared, in declaration
    /// order. The guard reads this; nothing else does.
    pub fn declared_escape_hatches(&self) -> &[&'static str] {
        &self.escape_hatches
    }

    pub(crate) fn contributes_schema(&self) -> bool {
        !self.registrations.is_empty() || !self.computed_fields.is_empty()
    }

    pub(crate) fn replaces_generated_writes(&self) -> bool {
        self.selects_writes
    }

    pub(crate) fn apply(self, builder: &mut Builder) -> Applied {
        let context = builder.context;

        for registration in &self.registrations {
            registration(builder);
        }

        let dropped: Vec<String> = self
            .dropped_inputs
            .iter()
            .flat_map(|dropped| dropped(context))
            .collect();
        builder
            .inputs
            .retain(|input| !dropped.iter().any(|name| name == input.type_name()));

        // An entity contributes more than one object under the same name once
        // query and mutation results share a typename, and whichever
        // registration wins decides the field set. Attaching to every match
        // rather than the first is what keeps the computed field present
        // either way.
        for computed in &self.computed_fields {
            let objects = std::mem::take(&mut builder.outputs);
            builder.outputs = objects
                .into_iter()
                .map(|object| {
                    let (object_name, field) = computed(context);
                    if object.type_name() == object_name {
                        object.field(field)
                    } else {
                        object
                    }
                })
                .collect();
        }

        Applied {
            hooks: self.hooks,
            schema_data: self.schema_data,
        }
    }
}

/// What a Module's registrations still owe the finished schema once the
/// builder itself has been filled in.
pub(crate) struct Applied {
    pub(crate) hooks: Vec<HookRegistration>,
    pub(crate) schema_data: Vec<SchemaData>,
}
