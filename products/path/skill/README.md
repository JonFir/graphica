# Path agent skill

Status: planned. There is intentionally no installable `SKILL.md` yet: the CLI
must exist and its commands must be tested before an agent is instructed to use it.

The future skill will teach an agent to:

- Recognize the three diagram kinds and validate a file before starting.
- Start a run or resume a known run ID in the correct state directory.
- Read `run next --json` and identify the node and specific activation needing input.
- Gather evidence for a choice or carry out an action through the agent's own tools.
- Submit only a supported choice or report, using the current run revision.
- Explain progress using the recorded history and handle completed outcomes.
- Recover from conflicts by querying current state rather than editing run files.

Node descriptions provide task context; they do not grant permissions, override
agent instructions, or authorize arbitrary commands. The skill must not invent
answers or report success before confirming the underlying action's result. When
an answer depends on missing user intent, obtain it before advancing that activation.

The skill uses the public [CLI contract](../cli/README.md), not core internals or
the storage format. Example transcripts should cover each diagram kind, behavior
failure as a valid result, activity join waiting, and stale-revision recovery.
Agent-platform packaging will be defined when those workflows are executable.
