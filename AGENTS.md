# Graphica — Repository Instructions

## Purpose

Graphica is an AI-first ecosystem for building and maintaining software using explicit architecture, structured project knowledge, and agent-friendly workflows.

This repository contains multiple related products, examples, documentation, and development tooling.

## Repository Structure

- `products/` — Independent Graphica products. A product may contain multiple technologies, packages, and applications.
- `examples/` — Runnable examples demonstrating how to use one or more Graphica products.
- `docs/` — Ecosystem-wide documentation, architecture, and concepts.
- `tooling/` — Shared development infrastructure, scripts, and automation.
- `.github/` — CI/CD workflows, issue templates, and GitHub automation.

Product-specific documentation and tooling should remain inside the corresponding product whenever possible.

## Architectural Principles

### 1. Organize by Product

Products are the primary architectural boundaries.

Do not organize products by programming language, framework, platform, or distribution method.

A single product may contain:

- Rust libraries and CLI applications
- TypeScript packages and web components
- Desktop applications
- AI skills and plugins
- Other tightly coupled components

Components that share an implementation or require coordinated releases should remain within the same product.

### 2. Products Are Self-Contained

Each product owns its implementation, dependencies, tests, documentation, and release process.

Products may use different languages, build systems, and internal directory structures.

Do not impose a universal structure on all products.

### 3. Product ≠ Package ≠ Repository

- **Product** — A cohesive unit of functionality and architectural responsibility.
- **Package** — A distributable artifact, such as an npm package, Rust crate, or binary.
- **Repository** — A source-control boundary.

A product may publish multiple packages with coordinated versions.

Although products currently share a monorepository, preserve boundaries that allow them to be extracted into independent repositories later.

### 4. Explicit Dependencies

Dependencies between products must be intentional, explicit, and documented.

Prefer public interfaces over direct access to another product's internal implementation.

Avoid introducing shared abstractions or cross-product coupling without a concrete need.

## Agent Workflow

When working on a task:

1. Identify the product or area responsible for the requested functionality.
2. Read its README, relevant documentation, and local `AGENTS.md` if present.
3. Follow existing architectural conventions and keep changes scoped to the responsible product.
4. When changes affect multiple products, consider compatibility and update affected interfaces together.
5. Run relevant tests, linting, and build checks using the product's own tooling.
6. Update documentation or examples when public behavior changes.
7. Clearly report any checks that could not be completed.

## Instruction Hierarchy

This file defines repository-wide principles.

Product-specific instructions belong in their respective directories, including:

- Technology choices and coding conventions
- Build and test commands
- Internal architecture
- Release procedures
- Product-specific agent workflows

Keep this root file concise. Do not duplicate detailed product instructions here.
