//! Selective registration of Seaography's generated mutation bundle.
//!
//! Upstream registration is all-or-nothing per entity: `register_entity!`
//! publishes create-one, create-batch, update, and delete together or not at
//! all. This module lets an entity register
//! with `mutation: false` and publish only the generated writes it can
//! honor, keeping the rest of the bundle — resolvers, guards, filters,
//! codecs, lifecycle hooks — untouched.

use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel};
use seaography::{
    async_graphql::dynamic::{InputObject, Object},
    Builder, EntityCreateBatchMutationBuilder, EntityCreateOneMutationBuilder,
    EntityDeleteMutationBuilder, EntityInputBuilder, EntityObjectBuilder,
    EntityUpdateMutationBuilder,
};

/// Push an output type unless a prior registrar already added it, so
/// generated and hooked registrars can be combined per entity in any mix.
pub(crate) fn push_output_once(builder: &mut Builder, object: Object) {
    if !builder
        .outputs
        .iter()
        .any(|existing| existing.type_name() == object.type_name())
    {
        builder.outputs.push(object);
    }
}

/// Push an input type unless a prior registrar already added it.
pub(crate) fn push_input_once(builder: &mut Builder, input: InputObject) {
    if !builder
        .inputs
        .iter()
        .any(|existing| existing.type_name() == input.type_name())
    {
        builder.inputs.push(input);
    }
}

/// The generated Seaography writes to publish for one registered entity.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GeneratedMutations {
    pub create_one: bool,
    pub create_batch: bool,
    pub update: bool,
    pub delete: bool,
}

impl GeneratedMutations {
    pub const CREATE_ONE: Self = Self {
        create_one: true,
        create_batch: false,
        update: false,
        delete: false,
    };

    pub const ALL: Self = Self {
        create_one: true,
        create_batch: true,
        update: true,
        delete: true,
    };

    fn any(self) -> bool {
        self.create_one || self.create_batch || self.update || self.delete
    }
}

/// Register selected fields from Seaography's generated mutation bundle.
///
/// The entity itself must already be registered with `mutation: false`. This
/// function only assembles Seaography's public builders; its generated
/// resolvers, guards, filters, codecs, and lifecycle hooks remain unchanged.
pub fn register_generated_mutations<T, A>(builder: &mut Builder, mutations: GeneratedMutations)
where
    T: EntityTrait,
    T::Model: Sync + IntoActiveModel<A>,
    A: ActiveModelTrait<Entity = T> + sea_orm::ActiveModelBehavior + Send + 'static,
{
    if !mutations.any() {
        return;
    }

    let context = builder.context;
    push_output_once(
        builder,
        EntityObjectBuilder { context }.to_basic_object::<T>(),
    );

    let input_builder = EntityInputBuilder { context };
    if mutations.create_one || mutations.create_batch {
        push_input_once(builder, input_builder.insert_input_object::<T>());
    }
    if mutations.update {
        push_input_once(builder, input_builder.update_input_object::<T>());
    }

    if mutations.create_one {
        builder
            .mutations
            .push(EntityCreateOneMutationBuilder { context }.to_field::<T, A>());
    }
    if mutations.create_batch {
        builder
            .mutations
            .push(EntityCreateBatchMutationBuilder { context }.to_field::<T, A>());
    }
    if mutations.update {
        builder
            .mutations
            .push(EntityUpdateMutationBuilder { context }.to_field::<T, A>());
    }
    if mutations.delete {
        builder
            .mutations
            .push(EntityDeleteMutationBuilder { context }.to_field::<T, A>());
    }
}
