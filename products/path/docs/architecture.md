# Architecture

Status: draft `0.1`; the Rust CLI is implemented locally. The web viewer and
skill remain planned.

## Ownership and dependencies

Path is one product with multiple deliverables. Its Rust CLI, npm viewer,
format specification, and agent skill evolve together. The CLI is the only
Rust crate in the product Cargo workspace.

| Component | Depends on | Owns |
| --- | --- | --- |
| Format | No runtime | Document schema, semantics, compatibility examples |
| CLI | Format | Parsing, graph validation, execution, run projections, terminal rendering, arguments, filesystem persistence, JSON output |
| Web | Format and public run projection | Layout, rendering, navigation, inspection |
| Skill | Documented CLI | Agent workflow and interpretation of results |

The CLI keeps parsing, validation, and interpretation in internal modules separate
from argument handling and filesystem persistence. The skill uses CLI commands
rather than editing run files. The viewer receives data from its host application
rather than reading the CLI's internal storage. Neither a service nor WASM is
required for the viewer.

## Data flow

1. Load one YAML or JSON document into the shared data model.
2. Validate its shape, then apply the selected kind's graph rules.
3. Start a run from an immutable copy of the validated diagram.
4. Advance automatic control nodes until external input is required or the run ends.
5. Expose available inputs, active activations, waiting branches, and history.
6. Accept one explicit choice or report, validate it, and calculate the next state.
7. Persist the new state and its events as one transaction.

`status`, `next`, `history`, and `run render` are queries. Reading a run never ticks a behavior
tree, advances a token, creates an activation, or changes a revision.

## Three interpreters, shared lifecycle

Use distinct interpreter state for the three kinds. Reuse IDs,
diagnostics, activation identity, event ordering, run lifecycle, and public views.

- Decision trees track the active question or terminal result.
- Behavior trees track entered composites, their current child, and leaf statuses.
- Activity diagrams track tokens and fork instances, including arrivals at joins.

Automatic transitions must be deterministic. Preserve declared child, choice,
and edge order; when expanding a fork, process outgoing edges in document order.
Caller input determines the order in which independent active actions complete.
Wall-clock time and run IDs cannot affect routing. The CLI supplies the run ID;
activation and fork-instance IDs are generated from per-run counters.

## Persistence and display

The [run contract](runs.md) separates durable interpreter state from a public
`RunView`. The CLI owns transactional storage and derives the view. Its terminal
renderer draws a validated diagram, optionally annotated with a saved `RunView`,
without changing run state. The web package renders the supplied diagram and
view without implementing another interpreter.
Multiple activations of one node remain distinguishable in history and display.

Schema validation alone is insufficient to declare a diagram executable. The CLI
also checks references, reachability, tree structure, and structured activity
regions. Runtime checks reject stale or inapplicable inputs before state changes.

## Delivery order

1. This foundation: schema, examples, semantic contracts, and acceptance scenarios.
2. Rust CLI, with graph checks, interpreter tests, persistence tests, and
   machine-readable output: implemented locally. A tested agent skill remains.
3. Web viewer using the same diagrams and public run projections as fixtures.

No packages are published. A format-version change is
distinct from a package release. Unknown format versions must be rejected; no
silent migration or guessed interpretation is allowed.
