# Path

Path is a family of tools for following a diagram to reach a decision or determine
the next actions. A person or an agent supplies choices and action results; Path
applies the diagram's control flow and records the run.

**Status: local Rust CLI available.** The format and interfaces remain draft
version `0.1`. The JSON Schema, examples, and CLI are available. The web
package and agent skill are not implemented or published.

## Diagram types

| Kind | Purpose | External input |
| --- | --- | --- |
| `decision-tree` | Follow questions to a result | Select a named answer |
| `behavior-tree` | Coordinate ordered actions and alternatives | Report a leaf's status |
| `activity-diagram` | Follow actions, branches, loops, and parallel flows | Complete an action or select a branch |

The three kinds share a document envelope and run lifecycle, but have distinct
validation and execution rules. Path does not execute commands, evaluate prose as
code, or decide which answer is true.

## Product components

| Area | Responsibility |
| --- | --- |
| [Format](format/README.md) | YAML/JSON document contract and diagram semantics |
| [CLI](cli/README.md) | Rust parser, validator, interpreter, terminal rendering, local run management, and machine-readable interface |
| [Web](web/README.md) | Planned embeddable npm viewer |
| [Skill](skill/README.md) | Planned guidance for agents using the CLI |
| [Examples](examples/README.md) | Equivalent YAML/JSON diagrams and walkthroughs |

Read the [architecture](docs/architecture.md), [run contract](docs/runs.md), and
[acceptance scenarios](docs/conformance.md) before implementing a component.
Product implementation, examples, and releases belong here. There are no
dependencies on other Graphica products.

## Build, validate, and test

Install Rust. From this product directory:

```sh
cargo test
cargo run --bin path -- validate examples/decision-tree.yaml --json
cargo run --bin path -- render examples/activity-diagram.yaml
cargo run --bin path -- run start examples/decision-tree.yaml --json
```

The CLI validates encoding, data shape, and graph semantics. Its Rust tests check
the shape fixtures, YAML/JSON example equivalence, node coverage, execution,
terminal rendering, and storage behavior.

The binary is `target/debug/path` after a build. Run files are placed under
`.path/runs/` by default; that directory is ignored by Git.

## CLI

Run the built binary as `target/debug/path` from this directory. Commands that
take a file accept `.json`, `.yaml`, and `.yml` diagrams. All commands are
noninteractive.

| Command | What it does |
| --- | --- |
| `path validate <file>` | Check encoding, data shape, and graph rules without creating a run. |
| `path render <file>` | Draw a validated diagram in the terminal without starting a run. |
| `path run start <file>` | Validate and freeze the diagram, create a run, and advance automatic nodes to the first inputs or outcome. |
| `path run status <run-id>` | Show the frozen diagram and complete current run view, including activations, available inputs, waiting joins, history, and outcome. |
| `path run next <run-id>` | Show the run's revision, status, outcome, and available inputs without the full diagram or history. |
| `path run choose <run-id> <activation-id> <choice-id> --revision <n>` | Select an available answer in a decision tree or outgoing edge in an activity diagram, then save the new state. |
| `path run report <run-id> <activation-id> <status> --revision <n>` | Report an action or condition result, then save the new state. Allowed statuses come from that input's `allowedStatuses`. |
| `path run history <run-id>` | Show the run's events in sequence order. |
| `path run render <run-id>` | Draw the frozen diagram with current inputs, waiting joins, completed nodes, and outcome marked. |

Use the `runId`, `revision`, and `activationId` returned by `start`, `status`, or
`next` when submitting `choose` or `report`. Each accepted mutation increments
the revision. `status`, `next`, `history`, and `run render` only read the run.

`--json` produces a machine-readable response for any command. Run commands
accept `--state-dir <dir>` to choose a run directory; the default is `.path/runs/`
under the current working directory. Both render commands accept `--ascii`.
`choose` and `report` accept `--note <text>` to record caller context in history.
Use `path --help`, `path run --help`, or a command's `--help` for argument help.
See the [CLI command reference](cli/README.md) for output fields, examples, and
exit codes.

## Initial boundaries

The runtime uses local files for independent runs. The planned web component
will display diagrams and supplied run snapshots without advancing them. A working
`SKILL.md` remains a separate deliverable.

A visual editor, storage server, command execution, reactive behavior trees,
arbitrary scripting, cross-diagram calls, and full UML compatibility are outside
the initial contract. Draft changes must update the schema, documentation, and
examples together; package names and release versions are not reserved yet.
