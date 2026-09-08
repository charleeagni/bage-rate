# Framework-enforced structure as prompt engineering

Research date: 2026-08-24

## Short answer

Yes. People already use most of this idea, although no single established term
covers the whole thing.

The closest current term is **coding-agent harness engineering**: make the
codebase, tools, generators, type checker, and tests steer an agent before it
writes the wrong thing and give it deterministic feedback afterward. Birgitta
Böckeler calls type checkers, linters, tests, and structural analysis
"computational" guides and sensors. She also argues that strong types, clear
module boundaries, and opinionated frameworks make a codebase more
"harnessable". Her proposed "harness templates" are especially close to this
repository: a known application topology packaged with the rules and checks
that keep an agent inside it. See [Harness engineering for coding agent
users](https://martinfowler.com/articles/harness-engineering.html).

The most direct article I found is Unmesh Joshi's July 2026 [DSLs Enable Reliable Use
of LLMs](https://martinfowler.com/articles/llm-and-dsls.html). Its core claim is
almost exactly the direction of this template. A small domain-specific API
shrinks the space of valid programs; illegal uses fail to compile; the LLM
becomes a natural-language interface to a vocabulary and checker that humans
designed. The maintained source of truth is then the DSL program and its
semantic model, not the prompt that first produced it.

This is therefore a real and timely direction. The novel part is not code
generation, schema-first development, static checking, or constrained tools by
themselves. It is treating all of them as one authoring environment designed
for probabilistic programmers as well as human ones.

There is measured evidence for the underlying hypothesis. The PLDI 2025 paper
[Type-Constrained Code Generation with Language
Models](https://doi.org/10.1145/3729274) applied a TypeScript type system during
decoding, so the model could choose only prefixes that remained completable to
a well-typed program. Across HumanEval and MBPP tasks and six open-weight model
families, it cut compilation errors by more than half. Functional correctness
rose by 3.5% to 5.5% relatively for synthesis and translation, and correct
repair of non-compiling programs rose by 37% relatively on average. That is a
tighter constraint than this repo's compile-after-generation loop, but it is
good evidence that formal authoring rules improve more than formatting.

## The useful names, and what each one means

| Name | Established meaning | Fit for this project |
| --- | --- | --- |
| Coding-agent **harness engineering** | Feed instructions and context into an agent, then use deterministic checks and other feedback to correct its output. | Best umbrella term for `AGENTS.md`, scaffolds, code generation, drift checks, type checking, and `verify`. |
| **Harnessability** or **ambient affordances** | Structural properties of a codebase that make it legible and governable by agents. | Names the property the template is deliberately creating. |
| **Agent-computer interface** (ACI) | The actions an agent can take and the feedback format it receives. The [SWE-agent paper](https://papers.neurips.cc/paper_files/paper/2024/file/5a7c947568c1b1328ccc5230172e1e7c-Paper-Conference.pdf) found that a small, agent-oriented action set with guardrails substantially outperformed a plain shell interface. | A precise existing name for "capabilities as prompt engineering" at the coding-tool boundary. |
| **Language-oriented programming** / a domain-specific language | Build a small language or fluent API around the concepts of a problem rather than repeatedly programming them with unrestricted primitives. Martin Fowler's [Language Workbenches](https://martinfowler.com/articles/languageWorkbench.html) traces this style through DSLs, semantic models, and code generation. | The `module-host` registration seam and its closed set of authoring primitives, the module scaffold, the migration format, and the caller-operation format together form a small application-building language, even though they are not one parser or syntax. Closing the set is what makes it a language rather than a library. |
| **Type-driven development** | Use types as plans and let the type checker guide construction. Idris describes types as tools for building programs and checking assumptions before execution. See [Idris: A Language for Type-Driven Development](https://idris-lang.org/). | Generated Rust and TypeScript types move decisions from memory and review into compiler feedback. |
| **Make invalid states unrepresentable** / static enforcement | Choose an API whose accepted types rule out bad inputs. The Rust API Guidelines explicitly prefer argument types that exclude invalid values because this catches errors at compile time. See [Dependability](https://rust-lang.github.io/api-guidelines/dependability.html) and [Type safety](https://rust-lang.github.io/api-guidelines/type-safety.html). | The project's strongest rules should ideally be impossible to express or should fail generation, rather than merely being prohibited in prose. |
| **Typestate** | Restrict which operations are available for an object in its current state. Strom and Yemini's original [Typestate paper](https://research.ibm.com/publications/typestate-a-programming-language-concept-for-enhancing-software-reliability) catches syntactically valid but semantically undefined operation sequences at compile time. | Relevant when a generated builder or API should expose only the next legal construction steps, not merely validate the finished artifact. |
| **Schema-first**, **contract-first**, or **generated-contract development** | Author or derive one machine-readable contract, validate it, generate language bindings, and block incompatible changes. | This is the repo's own clearest architectural label: migrations produce entities, entities produce GraphQL, operations produce caller-specific types, and drift checks prove that committed outputs agree. |
| **Executable architecture** / **architecture fitness functions** | Express architectural intent as checks that run continuously rather than as diagrams or review customs. Fowler's introduction to [Building Evolutionary Architectures](https://martinfowler.com/articles/evo-arch-forward.html) describes fitness functions as feedback that monitors architecture. | `verify`, the mini-schema ownership check, generated drift tests, dependency checks, and target bundle checks are architecture rules that execute. |
| **Correctness by construction** | In its formal-methods sense, refine a precise specification into an implementation while preserving proved properties. [Hall and Chapman](https://doi.org/10.1109/52.976937) describe an industrial secure system built with formal specification, rigorous design, and verified code. | Useful as an aspiration, but too strong as an unqualified description of this repo. The template prevents classes of structural and contract errors; it does not prove full functional correctness. |
| **Capability-based design** | In security literature, authority comes from the references or capabilities a component actually holds, rather than ambient access. Mark Miller's [Robust Composition](https://www.erights.org/talks/thesis/markm-thesis.pdf) develops this object-capability model; Wasmtime's [security documentation](https://docs.wasmtime.dev/security.html) gives a current implementation example. | Good for describing which tools and seams an agent or Module can access, and increasingly literal here: a Module reaches the Store only through the handle `ModuleCtx` gives it, and any other effect it ever needs arrives the same way rather than as a free import. It is still not the standard name for schema/codegen enforcement, and readers may assume you mean security. |

I would call the project direction **generated-contract architecture for an
agent harness**, or more conversationally, **making invalid edits
unrepresentable**. "Capability-based design as prompt engineering" is a sharp
phrase, but it should define *capability* as the allowed authoring operations,
because the existing security meaning is narrower.

## Prior art that closely matches the repository

### Framework-enforced module boundaries

[ArchUnit](https://www.archunit.org/userguide/html/000_Index.html) turns rules
such as "services may only be accessed by controllers" into executable tests
over Java bytecode. It checks layers, package dependencies, cycles, and custom
structural rules. [Spring Modulith's module
verification](https://docs.spring.io/spring-modulith/reference/2.1/verification.html)
goes further: it rejects module cycles, access to another module's internal
packages, and dependencies not listed as allowed.

That is the same family of mechanism as this repository's per-Module
mini-schema. A `modules/<name>/ui/operations/` document cannot name another
Module's fields because code generation validates it against a schema that
does not contain those fields. Spring Modulith checks the dependency graph
after code exists. This template narrows what the caller can even describe to
the generator.

Racket's language-oriented programming material makes the stronger version of
this point directly. Its [languages-versus-APIs
example](https://summer-school.racket-lang.org/2018/plan/mon-mor-lecture.html)
shows a graph library whose string-based API permits dangling node references,
then a DSL whose compiler rejects them. The lesson is that a language can check
cross-statement properties an ordinary library call cannot. The mini-schema is
playing that role here: it gives the operation language a smaller universe of
names.

### Machine-readable contracts that govern generated code

The [GraphQL specification](https://spec.graphql.org/September2025/) defines a
type system that determines whether an operation is valid and what response
types it can produce. [GraphQL Code Generator's operation
types](https://the-guild.dev/graphql/codegen/plugins/typescript/typescript-operations)
derive result and variable types from both that schema and the caller's actual
selection. This is the basis for the repo's rule against hand-written mirror
model types.

Buf is a strong comparison outside GraphQL. Its [schema
checks](https://buf.build/docs/bsr/checks/) treat Protobuf schemas as the
producer-consumer contract and reject breaking, duplicate, or policy-violating
changes before downstream generated SDKs advance. Its local [`buf
breaking`](https://buf.build/docs/breaking/) command compares the current
contract with a baseline and reports source, JSON, or wire incompatibilities.
That is close to reviewing the complete SDL diff as public API and rebuilding
artifacts in a clean directory to detect drift.

The framework choices reinforce this chain. SeaORM's [schema-first migration
guidance](https://www.sea-ql.org/SeaORM/docs/migration/writing-migration/)
recommends writing migrations first and generating entities from the resulting
database. [Seaography](https://www.sea-ql.org/Seaography/) then derives a typed
GraphQL API, including CRUD, filters, and pagination, from those entities.

### Constrained interfaces for agents

The NeurIPS 2024 [SWE-agent
paper](https://papers.neurips.cc/paper_files/paper/2024/file/5a7c947568c1b1328ccc5230172e1e7c-Paper-Conference.pdf)
is published precedent for treating interface design much like prompt design.
Its ACI replaces a broad shell with a small set of actions, concise feedback,
and editing guardrails. With the same GPT-4 Turbo model, its agent-oriented
interface improved the SWE-bench Lite resolution rate by 10.7 percentage
points over the shell baseline. The project's [first-party ACI
notes](https://github.com/SWE-agent/SWE-agent/blob/main/docs/background/aci.md)
make the connection explicit: the edit command runs a linter and refuses a
syntactically invalid edit instead of letting the error propagate.

Anthropic's [Writing effective tools for
agents](https://www.anthropic.com/engineering/writing-tools-for-agents) calls a
tool an interface between deterministic software and a nondeterministic agent.
It recommends clear namespaces, precise parameter names, strict input and
output models, actionable validation errors, and evaluation against real
tasks. It explicitly describes tool descriptions and specifications as prompt
engineering because the agent reads them as context.

The [Model Context Protocol tool
specification](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)
makes the idea concrete. A tool advertises a name, description, JSON Schema
input, and optional output schema. Servers must validate inputs, and a declared
output schema requires conforming structured results. A framework can therefore
remove malformed actions from the interaction language instead of asking the
model to remember their shape.

OpenAI's [Structured
Outputs](https://openai.com/index/introducing-structured-outputs-in-the-api/)
shows the harder version of the same move at generation time. The decoder masks
tokens that cannot lead to a value accepted by the supplied JSON Schema. In its
published evaluation, strict constrained output reached 100% schema adherence
where prompting alone did not. The documented limitation matters: structural
conformance does not make the values semantically correct. The repo likewise
needs behavior tests and human judgment on top of generated contracts.

## How this repository already implements the idea

The repository is unusually coherent on this point. Its architecture is a
chain of progressively narrower languages:

| Authored input | Framework-enforced result | Error moved earlier |
| --- | --- | --- |
| Reversible migration with database constraints | Scratch database and generated SeaORM entities | Invalid persistent shape or stale hand mirror |
| Module registration through `module-host` | Namespaced Seaography registration | Raw builder use and registration ownership violations |
| One `CustomOps` call per thing a Module needs beyond CRUD | Custom operations, write rules, write selection, transactions, computed fields, and subscriptions, all inside the contract | Hand-written resolvers, ad-hoc validation, and private seams reinvented per application |
| A Module's hand-authored files and its two manifests | Import scan and dependency allowlist | Reaching below the seam, and widening it by installing something |
| Composed entities | Reviewable `schema.graphql` | Hidden or accidental API drift |
| Caller-specific `.graphql` operation | Generated variables, result types, and typed document | Invalid fields, nullability errors, manual Apollo generics |
| Per-Module operation against its mini-schema | Module-local generated client | Cross-Module dependency |
| Module folder | Generated Rust registry and frontend index | Forgotten host wiring and inconsistent discovery |
| Authored sources plus committed generated artifacts | Clean-room regeneration and byte comparison | Stale generated output |
| Whole repository | `npm run verify` | Type, test, formatting, lint, dependency, target-boundary, and contract drift errors before release |

This is more than a prompt file. `AGENTS.md` tells a human or agent what to do,
but the migration runner, generator, mini-schema, type checker, and verification
scripts decide which outputs the repository will accept. Joshi's earlier [What
Is Code?](https://martinfowler.com/articles/what-is-code.html) uses almost this
exact framing: stable abstractions, types, tests, and invariants make the code
itself part of the agent's context and harness.

## Where the enforcement is still soft

The project should not claim that every rule is enforced structurally yet.
Some remain instructions or review obligations:

- A GraphQL operation can be well typed while choosing the wrong business
  behavior or returning too little data for Apollo cache convergence.
- The generated API intentionally retains bulk writes and broad filters. The
  caller-operation convention prevents accidental broad identity writes only
  when callers follow it or a check recognizes the dangerous operation shape.
- Reviewing the SDL as public API is a human decision unless compatibility or
  policy checks turn parts of that review into executable rules.
- The written exception required before custom CRUD prevents casual drift only
  if CI can identify the bypass or reviewers enforce the record.

Those are sensible boundaries. The cited Structured Outputs documentation
makes the same distinction: enforcing shape eliminates shape errors, not
errors in meaning. A useful design test for each new repository rule is:

1. Can the supported API make the invalid action impossible?
2. If not, can generation or compilation reject it?
3. If not, can `verify` detect it with an actionable error?
4. If judgment is unavoidable, is the required evidence small and explicit?

That ladder is the project's real version of "capability-based design as prompt
engineering." Prose supplies intent. The framework supplies the grammar, and
the build supplies the consequences.

## Suggested reading order

1. [DSLs Enable Reliable Use of LLMs](https://martinfowler.com/articles/llm-and-dsls.html), the closest direct statement of the idea.
2. [Type-Constrained Code Generation with Language Models](https://doi.org/10.1145/3729274), the strongest empirical evidence that formal constraints reduce invalid code and improve functional results.
3. [SWE-agent: Agent-Computer Interfaces Enable Automated Software Engineering](https://papers.neurips.cc/paper_files/paper/2024/file/5a7c947568c1b1328ccc5230172e1e7c-Paper-Conference.pdf), for measured effects of constraining actions and adding edit guardrails.
4. [Harness engineering for coding agent users](https://martinfowler.com/articles/harness-engineering.html), the best umbrella model for guides, deterministic checks, architecture fitness, and reusable topology templates.
5. [What Is Code?](https://martinfowler.com/articles/what-is-code.html), for code, vocabulary, and abstractions as agent context.
6. [Writing effective tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents), for capability and interface design as measurable prompt engineering.
7. [Racket and Language-oriented Programming](https://summer-school.racket-lang.org/2018/plan/mon-mor-lecture.html) and [Rust API Guidelines: Dependability](https://rust-lang.github.io/api-guidelines/dependability.html), for DSL and type-system enforcement of invalid states.
8. [Spring Modulith verification](https://docs.spring.io/spring-modulith/reference/2.1/verification.html) and the [ArchUnit user guide](https://www.archunit.org/userguide/html/000_Index.html), for executable module and architecture constraints.
9. [GraphQL type-system specification](https://spec.graphql.org/September2025/), [GraphQL operation code generation](https://the-guild.dev/graphql/codegen/plugins/typescript/typescript-operations), and [Buf schema checks](https://buf.build/docs/bsr/checks/), for generated-contract governance.
10. [Structured Outputs](https://openai.com/index/introducing-structured-outputs-in-the-api/), for the clearest measured example of enforcement outperforming prompt-only format instructions.
