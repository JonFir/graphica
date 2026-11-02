# Path — Product Instructions

Read `README.md`, `format/README.md`, and the relevant component README before
changing this product. Root repository instructions still apply.

## Boundaries

- Keep Path implementation, dependencies, and examples inside this product.
- The format and interfaces remain draft `0.1`. The local Rust CLI is available;
  do not present planned web interfaces as available software.
- The CLI uses Rust. The web viewer is an independent npm package.
  Do not introduce a framework dependency or duplicate execution logic in the viewer.
- The CLI interpreter owns diagram semantics. Its persistence layer and web
  rendering are adapters.
- Keep the three diagram kinds distinct; a single current-node pointer cannot
  represent an activity run with parallel branches.
- Choices and action outcomes come from the caller. Diagram text is never executable.

## Changes and checks

- Write documentation in English. Mark unimplemented interfaces and format changes
  as draft, and update linked examples and acceptance scenarios together.
- Keep YAML/JSON example pairs equivalent and preserve stable IDs when possible.
- JSON Schema describes data shape; the CLI validates encoding, shape, and graph
  rules. Keep the schema, CLI tests, and examples in sync.
- Run `cargo test` from this product directory after format or example changes.
- Check relative documentation links and run `git diff --check` before delivery.
- Report checks that could not run. Do not add a functional agent `SKILL.md` until
  the CLI exists and its documented commands have been verified.
