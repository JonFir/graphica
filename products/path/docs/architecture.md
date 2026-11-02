# Architecture

Status: draft `0.1`; runtime components are planned.

## Ownership and dependencies

Path is one product with multiple deliverables. Its Rust core and CLI, npm viewer,
format specification, and agent skill evolve together. Internal source layouts
and build manifests will be introduced with their implementations.

| Component | Depends on | Owns |
| --- | --- | --- |
| Format | No runtime | Document schema, semantics, compatibility examples |
| Core | Format | Parsing, graph validation, execution, run projections |
| CLI | Core | Arguments, diagnostics, filesystem persistence, JSON output |
| Web | Format and public run projection | Layout, rendering, navigation, inspection |
| Skill | Documented CLI | Agent workflow and interpretation of results |

The core must not depend on CLI arguments, the filesystem run store, browser APIs,
or a particular renderer. The skill uses CLI commands rather than editing run
files. The viewer receives data from its host application rather than reading the
CLI's internal storage. Neither a service nor WASM is required for the viewer.

## Data flow

1. Load one YAML or JSON document into the shared data model.
2. Validate its shape, then apply the selected kind's graph rules.
3. Start a run from an immutable copy of the validated diagram.
4. Advance automatic control nodes until external input is required or the run ends.
5. Expose available inputs, active activations, waiting branches, and history.
6. Accept one explicit choice or report, validate it, and calculate the next state.
7. Persist the new state and its events as one transaction.

`status`, `next`, and `history` are queries. Reading a run never ticks a behavior
tree, advances a token, creates an activation, or changes a revision.

## Three interpreters, shared lifecycle

Use distinct typed models and interpreter state for the three kinds. Reuse IDs,
diagnostics, activation identity, event ordering, run lifecycle, and public views.

- Decision trees track the active question or terminal result.
- Behavior trees track entered composites, their current child, and leaf statuses.
- Activity diagrams track tokens and fork instances, including arrivals at joins.

Automatic transitions must be deterministic. Preserve declared child, choice,
and edge order; when expanding a fork, process outgoing edges in document order.
Caller input determines the order in which independent active actions complete.
Wall-clock time and random IDs are supplied by the adapter and cannot affect routing.

## Persistence and display

The [run contract](runs.md) separates durable interpreter state from a public
`RunView`. The CLI owns transactional storage; the core derives the view. The web
package renders the supplied diagram and view without implementing another interpreter.
Multiple activations of one node remain distinguishable in history and display.

Schema validation alone is insufficient to declare a diagram executable. The core
also checks references, reachability, tree structure, and structured activity
regions. Runtime checks reject stale or inapplicable inputs before state changes.

## Delivery order

1. This foundation: schema, examples, semantic contracts, and acceptance scenarios.
2. Rust core and CLI, with graph checks, interpreter tests, persistence tests, and
   machine-readable output; then a tested agent skill.
3. Web viewer using the same diagrams and public run projections as fixtures.

No packages are published in the foundation stage. A format-version change is
distinct from a package release. Unknown format versions must be rejected; no
silent migration or guessed interpretation is allowed.
