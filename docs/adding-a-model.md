# Add a database-backed Model

Use this workflow for a table-backed SeaORM Model exposed through Seaography.
Every Model belongs to a Module — one folder under `modules/<name>/` carrying
the feature front to back. If the Model starts a new feature, scaffold its
Module first: `bun run new-module <name>` or `npm run new-module -- <name>`,
then follow the scaffold's printed next steps.

1. Add a reversible migration under `modules/<name>/rust/src/migrations/` and
   register it in that directory's `mod.rs`. Put nullability, uniqueness,
   references, defaults, indexes, and other database-enforceable invariants
   there. Every table name must carry the Module's name as its prefix; the
   module-host seam rejects the rest at generate time.
2. Run `bun run generate` or `npm run generate`. The script regenerates the
   module registry and frontend module index, migrates a clean scratch
   database per Module, replaces each Module's generated entity directory,
   exports the composed SDL, exports the TauRPC proxy, and runs
   operation-only GraphQL code generation against each Module's own
   mini-schema.
3. Never edit a file under `modules/*/rust/src/entities/`,
   `crates/module-registry/`, `src/generated/`, or
   `modules/*/ui/src/generated/`.
4. Review `schema.graphql` as public API. Generated mutations include batch
   create and optional-filter bulk update/delete. A Module that needs a
   different write surface — a rule enforced on every write, one mutation
   dropped so a custom operation is the only path — declares it through
   `module_host::CustomOps`; see `docs/writing-a-module-by-example.md` for the
   whole surface and `docs/operations/` for the recipe per surface.
5. Add `.graphql` operations under `modules/<name>/ui/operations/`. They are
   validated against the Module's own mini-schema, so they can name only this
   Module's types; a cross-module view is a host-level operation against the
   composed schema instead. For an identity-scoped update or delete, accept a
   non-null identity variable and bind it into a literal filter. Only expose a
   generated filter variable from an explicitly named bulk operation: a
   non-null `{}` filter still matches all rows. Construct caller-specific
   create/update `data` in the operation so auto-increment primary keys are
   not exposed to callers.
6. Pass generated documents directly to Apollo. Do not write mirror Model
   interfaces or manual Apollo generics.
7. Declare cache convergence for every mutation in the Module's
   `typePolicies`: update through the returned entity only when list
   membership and ordering cannot change; otherwise update/refetch affected
   lists. Create through list refetch/update, and delete through identity
   eviction or an explicit refetch.
8. Extend the Rust integration tests to cover clean migration, generated
   create/query/order/page/filtered-update/filtered-delete, constraints, and
   the down migration.
9. Add compile-time invalid-variable assertions where they protect important
   hazards.
10. Run `bun run verify` or `npm run verify`.

Once the Model is in, [`docs/operations/`](operations/README.md) covers
everything past generated CRUD: a table that picks the surface, then one
end-to-end page each for a [query](operations/query.md),
[mutation](operations/mutation.md), and
[subscription](operations/subscription.md).

Stop before adding custom CRUD, `mutation: false`, a repository that mirrors
generated behavior, hand-written GraphQL input/output types, raw Seaography
builder calls in Module code, or patches to a generated entity. Record the
exact missing behavior, why database/framework facilities cannot supply it,
the smallest custom seam, and its preventing test before proceeding.
