# Conformance and acceptance scenarios

Status: draft `0.1`. CLI tests cover encoding, shape fixtures, example
equivalence, graph, runtime, and storage scenarios. Viewer and agent scenarios
remain future acceptance criteria.

## Format checks in the CLI tests

- Parse the draft schema and check its declared JSON Schema dialect.
- Decode every YAML/JSON example pair, validate both documents, and compare them.
- Require examples for all three kinds and coverage of every supported node type.
- Check valid minimal roots and invalid shape fixtures in
  [schema-cases.json](../format/schema-cases.json).
- Reject ambiguous encoding (duplicate keys, multiple YAML documents, aliases,
  anchors, explicit tags, non-string keys, and non-JSON scalar values).
- Check links between local Markdown files and reject whitespace errors in the diff
  before delivery.

## Graph validation

For schema-valid inputs, the CLI must reject these cases with diagnostics identifying
the relevant data location. Codes below are the intended diagnostic vocabulary.

| Case | Expected code |
| --- | --- |
| Duplicate node IDs, choice IDs within a question, or activity edge IDs | `duplicate-id` |
| Unresolved root, entry, child, choice target, edge endpoint, or fork/join reference | `unknown-reference` |
| A node unreachable from the root/entry | `unreachable-node` |
| A tree cycle, shared child, incoming link to its root, or duplicate-target choices | `invalid-tree` |
| Activity entry not initial, multiple initials, or no final | `invalid-entry-or-final` |
| Activity node edge counts outside the specified range | `invalid-degree` |
| Missing label on an outgoing decision edge | `missing-choice-label` |
| Two activity edges with the same source and target | `duplicate-edge` |
| Activity node without any graph path to a final | `no-final-path` |
| Unpaired/mismatched fork and join or unequal branch/arrival counts | `invalid-parallel-region` |
| Branch escape, sibling crossing, early final, outside entry, or overlapping regions | `invalid-parallel-region` |
| Loop entering/exiting an open parallel region or re-entering an open fork | `invalid-parallel-region` |
| Directed cycle composed only of automatic activity nodes | `automatic-cycle` |

Accept a single result decision tree and a single action or condition behavior tree.
Accept properly nested parallel regions, a loop fully inside one branch, and a
loop around an entire completed fork/join region. Do not require that every caller
choice eventually terminates: repeated explicit choices may keep a valid run active.

## Decision-tree execution

Using the decision-tree example:

1. Start at `understood`; expose `known` and `unknown` in document order.
2. Choose `known`; expose `covered` with a new activation.
3. Choose `untested`; complete with result node `add-test` and value `add-test`.
4. On a separate run, choose `unknown`; complete with `investigate`.
5. A result-root fixture completes during start, with no external input.

Record the chosen IDs and the transitions explaining the outcome. Reject another
choice on either a completed activation or a completed run.

## Behavior-tree execution

Using the behavior-tree example:

| Inputs by node | Expected behavior |
| --- | --- |
| `ready: success` | Skip `repair`, activate `publish` |
| `ready: failure` | Activate `repair` through selector fallback |
| Then `repair: running` | Keep the same activation; retain composite positions |
| Then `repair: success` | Complete selector successfully, activate `publish` |
| Then `publish: success` | Complete root with success |
| `ready: failure`, `repair: failure` | Complete root with failure; never activate `publish` |
| `ready: success`, `publish: failure` | Complete root with failure |

Persist and reload after `repair: running`; the next input must still target that
activation. Querying does not reevaluate `ready`. A completed failure is CLI success
(exit `0`) with a failure outcome. `running` for a condition is rejected.

## Activity execution

Using the activity-diagram example:

1. Starting automatically passes `start` and `prepare-entry`, then waits at `prepare`.
2. Report success; expose the `review` choices `revise`, `approve`.
3. Choose `revise`; revisit `prepare` with a new activation ID. The earlier ID is stale.
4. Complete preparation and choose `approve`; expose both `docs` and `checks`.
5. Complete `docs`; show one arrival at `ready`, pending `checks`, and no `release` input.
6. Complete `checks`; release exactly one continuation and expose `release`.
7. Choose `ship`, complete `publish`, and finish at `done` with no pending inputs.

Repeat with the two branch completions reversed; get the same available next node
and terminal outcome, while history preserves the actual input order. Persist and
reload with one branch waiting at the join; preserve its arrival and fork instance.

Instead of `ship`, choose `retry` and reach `work` again. All activations and the
fork instance must be fresh. An arrival from the previous iteration cannot satisfy
the new join. The nested fork fixtures verify that
inner and outer join arrivals remain isolated. Waiting joins do not accept reports.

## Rejected input and persistence

All rejections leave revision, events, and persisted bytes unchanged:

| Input or failure | Expected behavior |
| --- | --- |
| Unknown choice or report status | `invalid-choice` / `invalid-report` |
| Report for a question or choose for an action | `operation-mismatch` |
| Unknown or completed activation | `inactive-activation` |
| Stale expected revision | `revision-conflict`; requery before retrying |
| Mutation after run completion | `run-completed` |
| Missing run | `run-not-found`; do not create storage |
| Corrupt or unsupported persisted state | Storage error; do not reset or migrate |
| Atomic write fails | Previous committed run remains readable; report I/O failure |

Additional acceptance checks:

- Start the same diagram twice and verify independent run and activation histories.
- Modify/delete the source file and resume using the frozen snapshot.
- Query `status`, `next`, and `history` repeatedly without writes or new events.
- Race two mutations at one revision: exactly one commits, the other conflicts.
- Retry after an uncertain successful write: the old revision cannot apply twice.
- Reject traversal-like run IDs; respect explicit state directory without global fallback.
- Verify event sequences, revisions, causal explanations, and outcome after reload.
- Start invalid input without creating a run; reject unsupported format versions.

## Terminal rendering

- Render each YAML/JSON example pair with equal output for its diagram kind.
- Show branch labels, ordered behavior children, activity loops, and parallel
  fork/join connections; retain full node descriptions below the graph.
- Render a saved run after the source file is removed, using its frozen diagram.
  Show available activations and a partially satisfied join without writing the
  run file or changing its revision.
- Check ASCII connectors and markers, JSON `{text}` output, and escaped control
  characters from diagram text.

## Viewer and agent acceptance

The viewer must render all example node kinds and display supplied state without
advancing it. Check diagram-only mode, multiple active branches, join waiting,
repeated visits, node inspection, version/identity mismatch, and disposal.

The skill's future verified transcripts must cover each kind, required revisions,
missing information, behavior failure, waiting joins, and conflict recovery. It
must use CLI JSON output and avoid modifying private run files.
