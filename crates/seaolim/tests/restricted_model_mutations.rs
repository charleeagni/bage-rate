mod common;

use std::sync::{LazyLock, Mutex};

use common::probes;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Condition, DatabaseTransaction, EntityTrait,
};
use seaography::{
    async_graphql::{dynamic::ResolverContext, Request},
    Builder, BuilderContext, LifecycleHooks, LifecycleHooksInterface, OperationType,
};
use seaolim::{
    register_restricted_model_mutation, string_argument, ComposedHooks, ModelWrite,
    PreparedModelWrite, RestrictedModelMutation, RestrictedMutationField, SignalBuffer, Signals,
};

static SEEN: Mutex<Vec<String>> = Mutex::new(Vec::new());
static CONTEXT: LazyLock<BuilderContext> = LazyLock::new(|| {
    let signals = Signals::default().on_with_context(
        "Probes",
        |_ctx, _action, row: Option<probes::ActiveModel>| async move {
            if let Some(Set(value)) = row.map(|row| row.value) {
                SEEN.lock().expect("seen poisoned").push(value);
            }
        },
    );
    let mut context = BuilderContext::default();
    context.hooks = LifecycleHooks::new(ComposedHooks::default().add(signals));
    context
});

struct CreateProbe;

struct DeleteProbe;

struct ProbeScope;

impl LifecycleHooksInterface for ProbeScope {
    fn entity_filter(
        &self,
        _ctx: &ResolverContext<'_>,
        entity: &str,
        action: OperationType,
    ) -> Option<Condition> {
        (entity == "Probes" && action == OperationType::Create)
            .then(|| Condition::all().add(probes::Column::Value.eq("allowed")))
    }
}

#[sea_orm::prelude::async_trait::async_trait]
impl RestrictedModelMutation<probes::Entity, probes::ActiveModel> for CreateProbe {
    async fn prepare(
        &self,
        ctx: &ResolverContext<'_>,
        _transaction: &DatabaseTransaction,
    ) -> seaography::async_graphql::Result<PreparedModelWrite<probes::ActiveModel, probes::Model>>
    {
        Ok(PreparedModelWrite::new(
            ModelWrite::Insert(probes::ActiveModel {
                value: Set(ctx.args.try_get("value")?.string()?.to_owned()),
                ..Default::default()
            }),
            (),
        ))
    }
}

#[sea_orm::prelude::async_trait::async_trait]
impl RestrictedModelMutation<probes::Entity, probes::ActiveModel> for DeleteProbe {
    async fn prepare(
        &self,
        ctx: &ResolverContext<'_>,
        transaction: &DatabaseTransaction,
    ) -> seaography::async_graphql::Result<PreparedModelWrite<probes::ActiveModel, probes::Model>>
    {
        let id = ctx.args.try_get("id")?.i64()?;
        let model = probes::Entity::find_by_id(id)
            .one(transaction)
            .await?
            .expect("probe exists");
        Ok(PreparedModelWrite::new(
            ModelWrite::Delete {
                active_model: model.clone().into(),
                model,
            },
            (),
        ))
    }
}

#[tokio::test]
async fn restricted_builder_owns_the_resolver_and_preserves_a_flat_field() {
    SEEN.lock().expect("seen poisoned").clear();
    let database = common::database_with_probes().await;
    let mut builder = Builder::new(&CONTEXT, database.clone());
    seaography::register_entity!(builder, probes, mutation: false);
    register_restricted_model_mutation::<probes::Entity, probes::ActiveModel, _>(
        &mut builder,
        RestrictedMutationField::new("create_probe", OperationType::Create)
            .argument(string_argument("value")),
        CreateProbe,
    );
    let schema = builder
        .schema_builder()
        .data(database)
        .finish()
        .expect("build restricted schema");

    assert!(schema
        .sdl()
        .contains("create_probe(value: String!): Probes!"));
    let response = schema
        .execute(
            Request::new(r#"mutation { create_probe(value: "kept") { value } }"#)
                .data(SignalBuffer::default()),
        )
        .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert_eq!(*SEEN.lock().expect("seen poisoned"), vec!["kept"]);
}

#[tokio::test]
async fn restricted_insert_enforces_the_create_entity_filter() {
    let mut context = BuilderContext::default();
    context.hooks = LifecycleHooks::new(ProbeScope);
    let context = Box::leak(Box::new(context));
    let database = common::database_with_probes().await;
    let mut builder = Builder::new(context, database.clone());
    seaography::register_entity!(builder, probes, mutation: false);
    register_restricted_model_mutation::<probes::Entity, probes::ActiveModel, _>(
        &mut builder,
        RestrictedMutationField::new("create_probe", OperationType::Create)
            .argument(string_argument("value")),
        CreateProbe,
    );
    let schema = builder
        .schema_builder()
        .data(database.clone())
        .finish()
        .expect("build restricted schema");

    let response = schema
        .execute(r#"mutation { create_probe(value: "outside") { id } }"#)
        .await;

    assert_eq!(response.errors.len(), 1);
    assert!(response.errors[0]
        .message
        .contains("created row is outside the caller's scope"));
    assert!(probes::Entity::find()
        .all(&database)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn restricted_delete_can_preserve_a_boolean_success_contract() {
    let database = common::database_with_probes().await;
    let inserted = probes::ActiveModel {
        value: Set("delete me".to_owned()),
        ..Default::default()
    }
    .insert(&database)
    .await
    .unwrap();
    let context = Box::leak(Box::new(BuilderContext::default()));
    let mut builder = Builder::new(context, database.clone());
    seaography::register_entity!(builder, probes, mutation: false);
    register_restricted_model_mutation::<probes::Entity, probes::ActiveModel, _>(
        &mut builder,
        RestrictedMutationField::new("delete_probe", OperationType::Delete)
            .returns_boolean()
            .argument(seaography::async_graphql::dynamic::InputValue::new(
                "id",
                seaography::async_graphql::dynamic::TypeRef::named_nn(
                    seaography::async_graphql::dynamic::TypeRef::INT,
                ),
            )),
        DeleteProbe,
    );
    let schema = builder
        .schema_builder()
        .data(database.clone())
        .finish()
        .expect("build restricted schema");

    assert!(schema.sdl().contains("delete_probe(id: Int!): Boolean!"));
    let response = schema
        .execute(format!("mutation {{ delete_probe(id: {}) }}", inserted.id))
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert_eq!(
        response.data,
        seaography::async_graphql::value!({"delete_probe": true})
    );
    assert!(probes::Entity::find_by_id(inserted.id)
        .one(&database)
        .await
        .unwrap()
        .is_none());
}
