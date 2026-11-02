# Path CLI

Status: planned Rust binary named `path`. The commands below are a draft contract;
they are not installed or implemented by this foundation.

## Commands

```text
path validate <file> [--json]
path run start <file> [--state-dir <dir>] [--json]
path run status <run-id> [--state-dir <dir>] [--json]
path run next <run-id> [--state-dir <dir>] [--json]
path run choose <run-id> <activation-id> <choice-id> --revision <n> [--note <text>] [--state-dir <dir>] [--json]
path run report <run-id> <activation-id> <status> --revision <n> [--note <text>] [--state-dir <dir>] [--json]
path run history <run-id> [--state-dir <dir>] [--json]
```

All commands are noninteractive. Missing arguments are errors; no prompts select
an answer or silently complete an action. `validate` checks encoding, shape, and
graph semantics without starting a run or creating storage.

`start` validates and freezes a diagram, creates a new run, and performs automatic
control transitions. `status`, `next`, and `history` are read-only. `choose` and
`report` require the revision returned by the latest query and return the updated
state after committing it. Status values are lowercase and constrained by the
selected activation's `allowedStatuses`.

## Machine output

With `--json`, write exactly one JSON object to stdout, including for errors:

```json
{"ok": true, "data": {}}
```

```json
{"ok": false, "error": {"code": "revision-conflict", "message": "Expected revision 2; found 3.", "details": {"expected": 2, "actual": 3}}}
```

The empty success `data` above illustrates the envelope only. Actual data is:

| Command | `data` |
| --- | --- |
| `validate` | `{valid: true, diagramId, kind, formatVersion}` |
| `start`, `status`, `choose`, `report` | `{diagram, view}` using the frozen diagram and `RunView` |
| `next` | `{runId, revision, status, outcome, availableInputs}` |
| `history` | `{runId, revision, events}` in increasing sequence order |

Errors have a stable `code`, human-readable `message`, and object `details`.
Validation errors use `details.diagnostics` with code, message, and JSON Pointer
for each diagnostic. Additional progress/log output goes to stderr. Human output
may change; scripts and the agent skill must use `--json`.

| Exit code | Meaning |
| --- | --- |
| `0` | Command accepted, including a behavior tree that completed with failure |
| `2` | Invalid arguments, document, choice, report, or activation |
| `3` | Run not found, revision conflict, or already completed run |
| `1` | I/O, corrupt/unsupported stored state, or internal failure |

Input rejection must not modify the run. Storage rules, revision checks, and
activation identity are defined in [runs](../docs/runs.md).

## Intended agent loop

1. Validate the file and start a run; retain its returned run ID.
2. Query `next`. If completed, consume the outcome.
3. Inspect an available input and the associated node description.
4. Obtain the answer or perform the action outside Path.
5. Submit `choose` or `report` with the returned activation ID and revision.
6. Query again after a conflict or uncertain write result; never resend blindly.

Use an explicit `--state-dir` when commands run from different working directories.
No interactive UI, import/export command, scripting engine, daemon, or automatic
migration is included in this draft.
