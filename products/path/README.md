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

## Initial boundaries

The runtime uses local files for independent runs. The planned web component
will display diagrams and supplied run snapshots without advancing them. A working
`SKILL.md` remains a separate deliverable.

A visual editor, storage server, command execution, reactive behavior trees,
arbitrary scripting, cross-diagram calls, and full UML compatibility are outside
the initial contract. Draft changes must update the schema, documentation, and
examples together; package names and release versions are not reserved yet.
