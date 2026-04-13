

# Contributing

## Development principles

This project is in early development. Prefer simple, legible code with strong assumptions over defensive code that tries to tolerate invalid input.

## Thin clients, shared Rust core

The intended architecture is thin platform clients with a shared Rust core. Keep platform-specific code small and push gameplay, presentation logic, and other shared behavior into Rust whenever possible.

When making changes:
- prefer shared Rust code over client-specific duplication
- keep Android, desktop, Kindle, and other clients focused on platform plumbing
- avoid copying gameplay or presentation logic into client layers unless it is truly platform-specific

## Fail fast, but stay simple

Do not add fallback behavior for states that should be impossible. If an invariant is violated, that is a bug, and it should fail loudly.

Just as important: do not clutter the code with defensive handling for impossible cases. Prefer clear code that assumes valid inputs where the architecture guarantees them. When those guarantees are broken, surface the error immediately rather than continuing silently.

What is wanted:
- simple code with explicit invariants
- strong assumptions where the architecture already guarantees correctness
- assertions or hard failure when impossible states occur
- code that makes invalid states hard to represent

What is not wanted:
- silent fallback behavior
- returning harmless defaults for impossible inputs
- extra defensive branching that obscures the main logic
- code that "does its best" after a broken invariant

The bias in this codebase is toward clarity and correctness, not graceful recovery from internal mistakes.

## General guidance

Prefer changes that make the code easier to reason about:
- keep responsibilities clear
- keep state transitions legible
- avoid hidden fallback paths
- avoid mixing long-term state with one-shot event metadata unless there is a clear reason

When simplifying, prefer fewer concepts and clearer boundaries over generalized machinery that the project does not need yet.

## Notes for agents and automated contributors

Do not assume that adding more guards makes the code safer. In this project, extra defensive code often makes behavior harder to reason about and can hide real bugs.

If a caller should never provide invalid input, prefer preserving that assumption and making failures obvious rather than silently accepting bad data.

Before adding fallback logic, ask whether the better fix is:
- to strengthen the invariant
- to simplify the call path
- to make the invalid state impossible
- or to fail immediately when the invariant is broken