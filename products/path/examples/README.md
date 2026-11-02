# Examples

Each YAML file has a JSON twin with exactly the same decoded data. These files
are format fixtures; no runtime is included yet.

| Pair | Demonstrates | Expected walkthrough |
| --- | --- | --- |
| [decision-tree.yaml](decision-tree.yaml) / [JSON](decision-tree.json) | Nested questions and terminal values | `known` then `untested` produces `add-test` |
| [behavior-tree.yaml](behavior-tree.yaml) / [JSON](behavior-tree.json) | Sequence, selector, condition, action | `ready: failure`, `repair: running`, `repair: success`, `publish: success` completes successfully |
| [activity-diagram.yaml](activity-diagram.yaml) / [JSON](activity-diagram.json) | Merge, explicit loop, fork/join, completion | Prepare, approve, finish both branches, select `ship`, publish |

Reports address activation IDs returned by the runtime, not the node labels used
in these walkthroughs. The examples describe work to be performed externally;
loading a file does not perform checks or publish anything.

For the activity example, complete `prepare`, then choose `revise` to revisit it
with a new activation. Choose `approve` on a later review to activate both `docs`
and `checks`. Completing only one leaves a waiting join; the `release` decision
appears only after both finish. Choosing `retry` from `release` repeats the whole
process with a new fork instance and fresh branch activations.

See [conformance scenarios](../docs/conformance.md) for the complete expected
success, failure, resume, concurrency, and rejection behavior. Run
`../.venv/bin/python ../tooling/check_format.py` from this directory to check the
schema and example equivalence after setting up dependencies from the product README.
