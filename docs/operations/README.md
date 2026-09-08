# Add an operation

Generated CRUD already handles operations that map directly to a table:

- read filtered, ordered, paginated rows
- create rows
- update rows by filter
- delete rows by filter

A migration generates those operations. Do not write them by hand.

Use these guides when generated CRUD does not fit:

- [Queries](query.md) answer questions that the generated connection cannot.
- [Mutations](mutation.md) add custom writes, restrict generated writes, or run rules before a write.
- [Subscriptions](subscription.md) notify connected clients about events.

If this is your first Module, read
[Writing a Module by example](../writing-a-module-by-example.md) first. It
explains the full Module. These pages focus on adding one operation.

## Choose the right API

Start with the first row that meets the requirement.

| Requirement | Use | Guide |
| --- | --- | --- |
| Read filtered, ordered, paginated rows | Generated `<module>` query | Already generated |
| Create, update, or delete a row | Generated mutations | Already generated |
| Derive a value from one row | `CustomOps::computed_field` | [Example, section 3](../writing-a-module-by-example.md) |
| Check every write to a Model | `CustomOps::write_hook` | [Guard generated writes](mutation.md#guard-generated-writes) |
| Remove a generated write | `CustomOps::writes` | [Remove a generated write](mutation.md#remove-a-generated-write) |
| Return an answer that is not a list of rows | Custom query | [Queries](query.md) |
| Perform a write that generated CRUD cannot express | Custom mutation | [Mutations](mutation.md) |
| Notify connected clients | Subscription | [Subscriptions](subscription.md) |
| Read several values, branch on them, then choose a write | `CustomOps::escape_hatch`, with a written exception | [Example, section 4](../writing-a-module-by-example.md#4-when-nothing-fits) |

`module_host::CustomOps` is the complete list. Module code cannot call lower
level Seaography builders. `scripts/check-module-imports.mjs` rejects those
imports.

If none of these APIs fits, write an exception in `docs/exceptions/`. If a
second Module needs the same behavior, add a reusable API to `module-host`
instead of adding another exception.

## If you know Django REST Framework

The closest DRF equivalents are:

| Django REST Framework | This project |
| --- | --- |
| `ModelSerializer` and `ModelViewSet` | Generated CRUD |
| `SerializerMethodField` | `CustomOps::computed_field` |
| `Serializer.validate()` or `pre_save` signal | `CustomOps::write_hook` |
| `http_method_names` or removing a mixin | `CustomOps::writes` |
| `@action(detail=False, methods=["get"])` | Custom query |
| `@action(methods=["post"])` | Custom mutation |
| `transaction.atomic()` | `ModuleCtx::transaction` |
| Request, settings, or storage access | Capabilities provided by `ModuleCtx` |
| Channels consumer | `CustomOps::subscription` and `ModuleCtx::publish` |
| `APIView` | `CustomOps::escape_hatch`, limited to two per Module and requiring an exception |
| `urls.py` | Nothing. `module_def()` registers the operation. |

The main difference is that there is no unrestricted equivalent of
`APIView`. Modules must use `CustomOps`, and the escape hatch is counted.

## The seven steps

Queries, mutations, and subscriptions follow nearly the same path.

| Step | Work | File |
| --- | --- | --- |
| 1 | Define the output type | `rust/src/<operation>.rs`, with `#[output]` |
| 2 | Write the resolver or publisher | The same Rust file |
| 3 | Register it | `rust/src/custom.rs` |
| 4 | Regenerate contracts | Run `npm run generate` |
| 5 | Add the caller's GraphQL document | `ui/operations/<module>.graphql` |
| 6 | Define cache behavior | `ui/src/cache.ts` |
| 7 | Test the composed schema | `crates/module-host/tests/primitives.rs` |

`npm run generate` composes the Module schemas, checks name prefixes, updates
`schema.graphql`, and runs GraphQL Code Generator. Each Module's caller
operations are checked against that Module's own schema. A Module cannot use
another Module's fields.

The GraphQL document in step 5 is required. Code generation creates the
`...Document` constant that the UI passes to Apollo. Do not replace it with a
handwritten TypeScript type or Apollo generic.

## Checks run by `npm run verify`

- Every custom GraphQL type and root field must start with its Module name.
  The check ignores case and underscores.
- Handwritten Module Rust may import only `crate` and `module_host`. Module UI
  may import React, Apollo, GraphQL, and files from the same Module.
- Module manifests may contain only allowlisted dependencies.
- Generated files must match their sources. Run `npm run generate` after a
  schema change.
- Every escape hatch needs a written exception. A Module may have at most two.

Run `npm run verify` before you finish.
