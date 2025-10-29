# Repository Guidelines

## Project Structure & Module Organization
Core logic lives under `src/`, with `main.rs` hosting the CLI wrapper and `lib.rs` re-exporting shared types. Encoding rules are isolated in `src/encoding.rs`, while card deck behaviors live in `src/punchcards.rs` to keep transformations testable. Integration tests belong in `tests/`; the current `tests/placeholder` can be replaced with scenario-focused suites. Long-form design notes and fix lists reside in `docs/`—sync any meaningful workflow or architecture updates there after implementing them.

## Build, Test, and Development Commands
Run `cargo fmt` before committing to keep formatting standard. Use `cargo clippy --all-targets --all-features` to catch lint issues early; treat warnings as actionable. Execute `cargo test` to verify both unit and integration coverage. While iterating on the CLI, `cargo run -- --help` validates argument wiring, and `cargo run -- --render --style ascii-x --seq` is a quick smoke test for rendering paths.

## Coding Style & Naming Conventions
Follow Rust defaults: four-space indentation, `snake_case` for modules/functions, `PascalCase` for types, and `SCREAMING_SNAKE_CASE` for compile-time constants. Keep public APIs documented with `///` doc comments; supplement complex transformations with concise inline notes. When extending the CLI, mirror existing Clap patterns (derive macros, `ArgGroup`) and prefer explicit enums over stringly-typed flags where possible.

## Testing Guidelines
Use focused unit tests near tricky encoding branches and deck sequencing logic; integration flows should sit under `tests/` and be named `<feature>_spec.rs` for clarity. Favor `pretty_assertions` for snapshot-like comparisons of rendered cards. Add regression tests whenever fixing issues captured in `docs/FIX.md`. Aim to cover new command-line switches with both success and failure cases to protect argument parsing.

## Commit & Pull Request Guidelines
Match the existing `<scope>: message` convention (`doc:`, `misc:`, `feat:`) using imperative voice and ≤72-character summaries, with optional issue references like `(#42)`. Each PR should state motivation, highlight CLI examples or user-facing changes, and list the tests you ran. Include screenshots or sample output when rendering behavior changes. Keep PRs focused; split large feature sets into reviewable chunks.

## Documentation & Knowledge Base
Update `README.md` whenever usage or installation steps evolve, and extend the `docs/` folder for deeper narratives (e.g., encoding specs, future work). Prefer linking to relevant doc sections from PR descriptions so reviewers can cross-check assumptions quickly.
