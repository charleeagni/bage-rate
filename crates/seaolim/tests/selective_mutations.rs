//! SDL contract tests for selective generated-mutation registration.

mod common;

use std::sync::LazyLock;

use common::probes;
use seaography::{Builder, BuilderContext};

use seaolim::{register_generated_mutations, GeneratedMutations};

static CONTEXT: LazyLock<BuilderContext> = LazyLock::new(BuilderContext::default);
static INSERT_POLICY_CONTEXT: LazyLock<BuilderContext> = LazyLock::new(|| {
    let mut context = BuilderContext::default();
    context
        .entity_input
        .insert_skips
        .push("Probes.value".to_owned());
    context
});

async fn selective_sdl(context: &'static BuilderContext, mutations: GeneratedMutations) -> String {
    let database = common::database_with_probes().await;
    let mut builder = Builder::new(context, database.clone());
    seaography::register_entity!(builder, probes, mutation: false);
    register_generated_mutations::<probes::Entity, probes::ActiveModel>(&mut builder, mutations);
    builder
        .schema_builder()
        .data(database)
        .finish()
        .expect("build selective mutation test schema")
        .sdl()
}

async fn native_bundle_sdl() -> String {
    common::probes_schema(&CONTEXT).await.sdl()
}

#[tokio::test]
async fn all_selection_matches_seaography_native_bundle() {
    assert_eq!(
        selective_sdl(&CONTEXT, GeneratedMutations::ALL).await,
        native_bundle_sdl().await
    );
}

#[tokio::test]
async fn each_flag_exposes_only_its_mutation_and_required_inputs() {
    let cases = [
        (GeneratedMutations::CREATE_ONE, "CreateOne", true, false),
        (
            GeneratedMutations {
                create_batch: true,
                ..GeneratedMutations::default()
            },
            "CreateBatch",
            true,
            false,
        ),
        (
            GeneratedMutations {
                update: true,
                ..GeneratedMutations::default()
            },
            "Update",
            false,
            true,
        ),
        (
            GeneratedMutations {
                delete: true,
                ..GeneratedMutations::default()
            },
            "Delete",
            false,
            false,
        ),
    ];

    for (selection, selected, has_insert, has_update) in cases {
        let sdl = selective_sdl(&CONTEXT, selection).await;
        for operation in ["CreateOne", "CreateBatch", "Update", "Delete"] {
            assert_eq!(
                sdl.contains(&format!("probes{operation}(")),
                operation == selected,
                "unexpected {operation} field for {selection:?}"
            );
        }
        assert_eq!(sdl.contains("input ProbesInsertInput"), has_insert);
        assert_eq!(sdl.contains("input ProbesUpdateInput"), has_update);
        assert!(sdl.contains("type ProbesBasic"));
    }
}

#[tokio::test]
async fn empty_selection_registers_no_mutation_support_types() {
    let sdl = selective_sdl(&CONTEXT, GeneratedMutations::default()).await;

    assert!(!sdl.contains("type ProbesBasic"));
    assert!(!sdl.contains("input ProbesInsertInput"));
    assert!(!sdl.contains("input ProbesUpdateInput"));
}

#[tokio::test]
async fn selected_mutation_preserves_context_input_policy() {
    let sdl = selective_sdl(&INSERT_POLICY_CONTEXT, GeneratedMutations::CREATE_ONE).await;
    let start = sdl
        .find("input ProbesInsertInput {")
        .expect("insert input exists");
    let input = &sdl[start..];
    let input = &input[..input.find("\n}").expect("insert input terminates")];

    assert!(input.contains("\n\tid:"));
    assert!(!input.contains("\n\tvalue:"));
}
