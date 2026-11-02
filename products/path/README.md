# Path

Path is a family of tools for following a diagram to reach a decision or determine
the next actions. A person or an agent supplies choices and action results; Path
applies the diagram's control flow and records the run.

**Status: foundation only.** The format and interfaces are draft version `0.1`.
This product currently contains documentation, a JSON Schema, examples, and format
checks. The Rust core, CLI, web package, and agent skill are not implemented or
published. Command and API examples describe intended interfaces.

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
| [Core](core/README.md) | Planned Rust model, validator, and deterministic interpreter |
| [CLI](cli/README.md) | Planned local run management and machine-readable interface |
| [Web](web/README.md) | Planned embeddable npm viewer |
| [Skill](skill/README.md) | Planned guidance for agents using the CLI |
| [Examples](examples/README.md) | Equivalent YAML/JSON diagrams and walkthroughs |

Read the [architecture](docs/architecture.md), [run contract](docs/runs.md), and
[acceptance scenarios](docs/conformance.md) before implementing a component.
Product implementation, tooling, examples, and releases belong here. There are no
dependencies on other Graphica products.

## Check the foundation

From this product directory, with Python 3.10 or later:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r tooling/requirements.txt
.venv/bin/python tooling/check_format.py
```

The check validates the schema itself, checks all paired examples, compares their
decoded data, and exercises positive and negative format fixtures. It does not
implement graph validation or run execution. Those checks are specified as future
core acceptance scenarios.

## Initial boundaries

The first runtime will use local files for independent runs. The web component
will display diagrams and supplied run snapshots without advancing them. A working
`SKILL.md` will ship only after its CLI commands can be tested.

A visual editor, storage server, command execution, reactive behavior trees,
arbitrary scripting, cross-diagram calls, and full UML compatibility are outside
the initial contract. Draft changes must update the schema, documentation, and
examples together; package names and release versions are not reserved yet.
