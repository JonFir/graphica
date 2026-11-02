# Run contract

Status: draft `0.1`; the local run store and interpreter are implemented. This is
a logical contract, not a promise of a stable serialized Rust type.

## Identity and lifecycle

A run contains an immutable validated diagram snapshot, an opaque run ID,
`runVersion: "0.1"`, a revision, interpreter state, and an ordered event history.
Editing or moving the original diagram cannot change an existing run. Two starts
of the same diagram create independent runs.

Start is revision `1`. Each accepted `choose` or `report` increments the revision
once, including a `running` report. All resulting automatic transitions belong to
that revision. Queries and rejected inputs do not alter state, history, or revision.
IDs are opaque, case-sensitive strings; callers must not derive them from titles
or interpret their generation scheme.

Run status is `active` or `completed`. Active runs await external inputs and may
also have waiting joins. A completed run has no pending inputs and an outcome:

- Decision tree: `{ "kind": "decision-tree", "nodeId": "...", "value": "..." }`.
- Behavior tree: `{ "kind": "behavior-tree", "status": "success" | "failure" }`.
- Activity diagram: `{ "kind": "activity-diagram", "nodeId": "..." }`, identifying
  the reached final.

An active run has `outcome: null`. Cancellation, undo, editing a live diagram, and
restart-in-place are deferred. A new attempt is a new run. An active state with no
available inputs after automatic advancement indicates an interpreter error, not
successful completion; do not commit a partially advanced state.

## Activations and inputs

Every entered node receives a fresh activation ID unique within the run. This is
distinct from the stable node ID. A `running` report retains the same activation;
re-entering a node after an activity loop creates a new activation.

The public `RunView` contains:

| Field | Meaning |
| --- | --- |
| `runVersion`, `runId`, `revision` | Version, identity, and concurrency boundary |
| `diagramId`, `kind` | Identity and kind from the snapshot |
| `status`, `outcome` | Lifecycle and optional terminal result |
| `activations` | Entered nodes with `activationId`, `nodeId`, lifecycle status, and optional reported result |
| `availableInputs` | Externally actionable leaf/question/action activations |
| `waitingJoins` | Join activations, fork-instance IDs, and arrived/pending branch IDs |
| `history` | All committed events in increasing sequence order |

Activation lifecycle status is `active`, `waiting`, or `completed`; a completed
behavior activation additionally records `success` or `failure`. A running action
can record `running` while remaining active. Composite behavior ancestors remain
active, but only leaves appear in `availableInputs`. Unvisited nodes have no
activation. Each fork instance has one waiting join activation, created on its
first arrival and completed when all arrivals are present.

An available input contains `activationId`, `nodeId`, and `operation`:

- `choose`: ordered `choices`, each with `id`, `label`, and `target`. Decision-tree
  IDs come from choices; activity IDs come from outgoing edges.
- `report`: `allowedStatuses`, in the order `running`, `success`, `failure` with
  inapplicable statuses omitted. Behavior conditions allow success/failure,
  behavior actions allow all three, and activity actions allow success only.

List available inputs by activation creation order. Display labels and descriptions
come from the diagram snapshot. Run queries return a complete public view; `next`
returns only run identity, revision, status, outcome, and available inputs.

Input is addressed to an activation, never just a node. The mutation includes an
expected revision. The CLI interpreter rejects a revision conflict, unknown or inactive
activation, operation mismatch, unavailable choice, or disallowed report before
making any changes. Mutations on completed runs are rejected. A retried successful
request with the old revision fails; clients query the run before deciding to retry.

## History and explanation

Events have a monotonically increasing `sequence` starting at `1`, a `revision`,
`type`, and relevant node/activation IDs. Define these event categories:

- `run-started`: identifies the frozen diagram and format version.
- `node-entered`: identifies the activation and its causal predecessor or parent.
- `choice-selected`: records the choice/edge ID and optional caller note.
- `status-reported`: records the supplied status and optional caller note.
- `node-completed`: records any result and the rule that completed the node.
- `transition-taken`: records source/target activations, the choice/edge or child
  position, and the control rule (for example, sequence continuation or selector fallback).
- `join-arrived` and `join-released`: record fork instance, branch IDs, and arrivals.
- `run-completed`: records the outcome.

Automatic events name their rule and causal event sequence. This explains how an
input led to the current state without claiming to know why the caller chose an
answer. At a join, all arrival events are causal inputs. The first arrival creates
the waiting join activation before emitting `join-arrived`. A failed command must
not appear as an accepted history event. Timestamps may aid display but do not
control routing or establish event order.

## Local storage

The CLI stores `<run-id>.json` under `.path/runs/` relative to the invocation's
working directory. `--state-dir DIR` selects the run directory itself, with
relative paths resolved against that working directory. There is no upward project
search or global fallback. Queries never create a missing directory.

One run file holds the diagram snapshot, interpreter state, and history in one
envelope. Write through a temporary file and atomic replacement under a per-run
lock, checking the expected revision while holding that lock. Persist before
reporting success; leave the previous file intact on failure. Refuse a corrupt or
unsupported run version rather than resetting or partially loading it.

Use collision-resistant IDs and exclusive creation for starts. Run IDs supplied
to commands are identifiers, not filesystem paths. Keep runtime data out of source
control. Migration and retention policies are deferred; there is no automatic
cleanup or migration in this draft.

The durable interpreter representation is private to the CLI/run-version pair.
The public `RunView` is derived from it and is not sufficient to resume execution.
Consumers must not edit persisted state. `run status` returns both the frozen
`diagram` and its `view` so a viewer can receive a matching pair even when the
source file has changed.
