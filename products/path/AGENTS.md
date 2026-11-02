# Path — Product Instructions

Read `README.md`, `format/README.md`, and the relevant component README before
changing this product. Root repository instructions still apply.

## Boundaries

- Keep Path implementation, dependencies, tooling, and examples inside this product.
- This stage defines a draft contract. Do not present planned CLI commands or web
  interfaces as available software.
- The future core and CLI use Rust. The web viewer is an independent npm package.
  Do not introduce a framework dependency or duplicate execution logic in the viewer.
- The core owns diagram semantics. CLI persistence and web rendering are adapters.
- Keep the three diagram kinds distinct; a single current-node pointer cannot
  represent an activity run with parallel branches.
- Choices and action outcomes come from the caller. Diagram text is never executable.

## Changes and checks

- Write documentation in English. Mark unimplemented interfaces and format changes
  as draft, and update linked examples and acceptance scenarios together.
- Keep YAML/JSON example pairs equivalent and preserve stable IDs when possible.
- JSON Schema validates data shape. Document graph and runtime rules separately;
  do not claim the foundation checker verifies them.
- Run `.venv/bin/python tooling/check_format.py` from this directory after format or
  example changes. Dependency setup is in `README.md`.
- Check relative documentation links and run `git diff --check` before delivery.
- Report checks that could not run. Do not add a functional agent `SKILL.md` until
  the CLI exists and its documented commands have been verified.
