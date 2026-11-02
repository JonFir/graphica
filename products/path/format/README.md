# Path diagram format

Status: draft `0.1`. The normative data-shape contract is
[`path.schema.json`](path.schema.json), using JSON Schema Draft 2020-12. The graph
and execution rules below are additional requirements for the future core.

## Encoding and common fields

A file contains one UTF-8 JSON object or one YAML document with the same data
model. Choose the parser by `.json`, `.yaml`, or `.yml`. Reject duplicate mapping
keys, non-string mapping keys, YAML aliases, anchors, merge keys, and explicit
tags. There is no interpolation or executable content.

All scalar fields in version `0.1` are strings. For portable YAML, quote strings
that could be resolved as booleans, nulls, numbers, or dates, including the version.
Mapping order has no meaning. Array order is preserved.

| Field | Contract |
| --- | --- |
| `formatVersion` | Exactly the string `"0.1"` |
| `kind` | `decision-tree`, `behavior-tree`, or `activity-diagram` |
| `id` | Stable identifier for this diagram |
| `title` | Nonempty display text |
| `description` | Optional plain text |
| `nodes` | Nonempty array of nodes |
| `root` | Root node ID for either tree kind |
| `entry` | Initial node ID for an activity diagram |
| `edges` | Nonempty array of directed edges, only for activity diagrams |

Diagram, node, choice, and edge IDs match `^[A-Za-z][A-Za-z0-9_-]*$`, case
sensitively. Node IDs are unique within a diagram. Edge IDs are unique within an
activity diagram; choice IDs are unique within their question. References use node
IDs, not labels or array positions. Renaming an ID changes its identity.

Every node has `id`, `type`, `title`, and optional `description`. Unknown fields and
node types are errors. There are no renderer-specific coordinates in this version.
Titles, labels, descriptions, and result values are plain text, not HTML or code.

## Decision trees

| Node type | Additional fields | Meaning |
| --- | --- | --- |
| `question` | `choices`: at least two `{id, label, target}` objects | Wait for a choice ID |
| `result` | `value`: nonempty string | Complete with this result value |

Choices are ordered for presentation; order does not establish a default answer.
The root may be a question or a result. Every non-root node has exactly one parent,
every reference resolves, and every node is reachable from the root. Cycles and
shared children, including two choices targeting the same node, are invalid.

Starting at a question exposes one `choose` input. Choosing follows exactly that
choice's target. Entering a result immediately completes the run. Starting at a
result produces a completed run without caller input.

## Behavior trees

| Node type | Additional fields | Meaning |
| --- | --- | --- |
| `sequence` | Ordered, nonempty `children` of distinct node IDs | Require every child to succeed |
| `selector` | Ordered, nonempty `children` of distinct node IDs | Accept the first child that succeeds |
| `action` | None | Await `running`, `success`, or `failure` |
| `condition` | None | Await `success` or `failure` |

These are memoryful, non-reactive composites. On entry, a composite activates its
first child. A sequence proceeds on success and fails immediately on failure. A
selector proceeds on failure and succeeds immediately on success. Exhausting the
children gives sequence success or selector failure. Later siblings that are not
needed remain unvisited.

`running` leaves the same action activation pending. A later report addresses that
same activation. Completed siblings are not rechecked when resuming. There is at
most one externally actionable leaf at a time. No periodic tick or background
evaluation occurs. A condition's name or description is a question for the caller;
Path does not evaluate it.

Every non-root node has exactly one parent, all nodes are reachable, references
resolve, and there are no cycles or shared children. The root may be any supported
node. Its success or failure completes the run with that outcome; a failed root is
a valid domain result, not a tool error. Starting another attempt creates a new run.
Decorators, retries, reactive composites, and parallel composites are deferred.

## Activity diagrams

Edges contain `id`, `source`, `target`, and optional nonempty `label`. A decision's
outgoing edges must have labels. Their edge IDs are the choices accepted by the
runtime. Labels describe the choice; they are not executable guards.

| Node type | Incoming edges | Outgoing edges | Additional fields / behavior |
| --- | --- | --- | --- |
| `initial` | 0 | 1 | The unique entry; forwards automatically |
| `action` | 1 | 1 | Waits for a `success` report, then forwards |
| `decision` | 1 | At least 2 | Waits for selection of an outgoing edge |
| `merge` | At least 2 | 1 | Forwards each arriving token without waiting for others |
| `fork` | 1 | At least 2 | `join`: ID of its paired join; starts all branches |
| `join` | At least 2 | 1 | `fork`: ID of its paired fork; waits for all its branches |
| `final` | At least 1 | 0 | Completes the run when reached outside a fork region |

There is exactly one initial and at least one final. `entry` identifies the initial.
All nodes are reachable from entry and have a graph path to a final. Each edge must
resolve to existing nodes; parallel edges with identical source and target are
invalid. Use explicit decision/merge and fork/join nodes; no implicit splitting or
synchronization is inferred from an action's edge counts. Multiple incoming edges
at a final are allowed only for alternatives outside fork regions.

### Structured parallel regions

A fork and join reference each other one-to-one. Each outgoing fork edge starts
one branch region, which ends at exactly one distinct incoming edge of that join.
The number of outgoing fork edges equals the number of incoming join edges.

Branches are disjoint until the paired join: they cannot cross into siblings,
escape the region, end at a final, or accept an entry from outside. Alternative
paths inside a branch must merge before its join arrival. Nested fork/join regions
are allowed and must be properly nested. Every branch node has a graph path to
its boundary join; user choices may still keep a run in a loop indefinitely.

Each fork activation creates a fresh fork-instance ID. A branch ID is the ID of
its outgoing fork edge, scoped to that instance. Tokens carry both identities.
A join consumes one arrival from each branch of that same
instance, then emits one continuation token. It must never combine arrivals from
different iterations or nested forks. An early arrival is shown as waiting and
cannot be completed by the caller.

### Loops and completion

Loops may occur entirely within a branch or outside parallel regions, including a
loop that re-enters a whole fork/join region after it has finished. They cannot
cross an open region boundary or re-enter an open fork. Every directed cycle must
contain an externally controlled node (`action` or `decision`). Equivalently, the
subgraph induced by automatic nodes must be acyclic. This ensures automatic
advancement reaches a waiting point in finitely many steps.

Use a merge for entry and loop-back paths before an action or decision. Each visit
creates a new activation ID. A run can revisit nodes indefinitely through explicit
caller input; Path does not impose a hidden iteration limit.

An activity action accepts only `success`, meaning that the caller declares it
complete. To model failure or retry, follow an action with an explicit decision.
`running` and `failure` are not activity reports in this draft. Reaching a final
completes the run; open fork regions must already have joined. There is no implicit
cancellation of parallel branches or UML flow-final node.

## Validation layers

1. **Encoding:** valid single document, unambiguous mappings, supported YAML profile.
2. **Shape:** version, kind, required fields, scalar types, allowed node variants.
3. **Graph:** ID uniqueness, resolved references, reachability, topology, edge
   counts, structured fork/join regions, and automatic-cycle checks.
4. **Runtime:** active activation, permitted operation, valid choice/report, and
   current run revision.

The JSON Schema and foundation checker cover shape; the checker also verifies
example encoding and YAML/JSON equivalence. Graph and runtime checks belong to the
future core. Diagnostics should include a stable code, message, and data location
(JSON Pointer), plus node/edge IDs when applicable. See the
[acceptance scenarios](../docs/conformance.md).
