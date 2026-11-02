# Path core

Status: planned Rust library; this directory currently defines responsibilities.

The core owns the typed diagram model, supported YAML/JSON parsing, data-shape and
graph validation, deterministic interpretation, and public run projection. It is
the source of execution semantics for all adapters.

## Intended interface

The following operations describe behavior, not final Rust signatures:

| Operation | Input | Output |
| --- | --- | --- |
| Parse | Text and explicit encoding | Diagram or encoding/shape diagnostics |
| Validate | Diagram | Validated diagram or graph diagnostics |
| Start | Validated diagram and adapter-supplied identity context | Initial run and events, advanced to external input or completion |
| Apply | Run, expected revision, activation ID, choice/report, optional note | New run and events, or an error leaving the old run unchanged |
| View | Run | Public `RunView` |
| Next | Run | Query projection of available inputs and outcome |

Only validated diagrams can start. Model the three kinds as distinct variants;
use separate interpreter state for trees and activity tokens. The adapter supplies
fresh IDs, while the core controls their logical creation order. Do not perform
filesystem access, execute leaf descriptions, or call an LLM from the interpreter.

Graph rules are defined in the [format](../format/README.md). Mutation, activation,
and event contracts are in [runs](../docs/runs.md). The
[conformance scenarios](../docs/conformance.md) are the implementation acceptance
criteria, including invalid graphs that intentionally pass JSON Schema.

Rust crate manifests, the concrete serialization of private interpreter state, and
parser library selection will accompany implementation. No Rust build or test
command is available at this stage.
