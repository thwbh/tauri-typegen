# tauri-typegen

`tauri-typegen` generates TypeScript (and optionally Zod) bindings from a Tauri
backend's Rust code. Tauri ships no built-in way to keep frontend types in sync
with `#[tauri::command]` functions and their models; this crate fills that gap
by parsing the Rust source with `syn` and rendering typed output with Tera
templates.

It ships as both a library (`tauri-typegen`) and a cargo subcommand binary
(`cargo-tauri-typegen`). Consumers use it as a build dependency
(`BuildSystem::generate_at_build_time()` in `build.rs`) or as a CLI
(`cargo tauri-typegen generate`).

- Edition: 2021 · MSRV: 1.77.2 · CI toolchain: 1.93.0
- Validation modes: `none` (vanilla TS types) and `zod` (runtime schemas +
  hooks). There are no other modes — don't introduce one without instruction.

## Build, test, and verify

Run these from the repo root. They mirror CI (`.github/workflows/ci.yml`); all
three must pass before a change is complete:

```bash
cargo test --all-features
cargo fmt -- --check          # formatting is enforced; run `cargo fmt` to fix
cargo clippy --all-targets --all-features -- -D warnings   # warnings fail CI
```

To run a focused test: `cargo test <name>` (e.g. `cargo test command_parser`,
`cargo test --test integration_e2e <name>`).

## Architecture (the generation pipeline)

The flow is **analysis → models → generation**, single-pass over a cached AST.

1. **Analysis** (`src/analysis/`) — parses Rust files into the data model.
   - `ast_cache.rs` parses and caches every `.rs` file once (single-pass goal).
   - `command_parser.rs`, `struct_parser.rs`, `event_parser.rs`,
     `channel_parser.rs`, `serde_parser.rs`, `validator_parser.rs` extract
     their respective constructs.
   - `type_resolver.rs` + `dependency_graph.rs` perform lazy, on-demand type
     discovery. `mod.rs` (`CommandAnalyzer`) orchestrates the pass.
2. **Models** (`src/models.rs`) — the shared data model: `CommandInfo`,
   `ParameterInfo`, `StructInfo`, `FieldInfo`, `EnumVariantInfo`,
   `TypeStructure`, etc. Analysis fills these; generation reads them.
3. **Generation** (`src/generators/`) — turns models into output files.
   - `base/` holds shared machinery: `template_context.rs` (the `*Context`
     structs handed to templates), `type_visitor.rs` (the `TypeVisitor`
     trait), and `templates.rs`.
   - `ts/` and `zod/` each provide a generator, a `TypeVisitor` impl, and a
     `templates/` directory of `.tera` files. Zod adds `schema_builder.rs` and
     `filters.rs`.
   - `mod.rs` builds the contexts (`create_command_contexts`,
     `create_event_contexts`, …) consumed by every template.
4. **Build/IO** (`src/build/`) — project scanning, caching (`.typecache`),
   and writing output. **Interface** (`src/interface/`) — CLI, config parsing,
   logging. `BuildSystem` and `GenerateConfig` are re-exported from `lib.rs`.

## Conventions you must follow

- **Match surrounding code.** This is idiomatic Rust formatted with `rustfmt`;
  keep clippy clean (CI denies warnings).
- **Deduplication happens at the generation layer, not analysis.** Commands and
  events are deduped by name (first-occurrence-wins) in
  `src/generators/mod.rs` (`create_command_contexts` /
  `create_event_contexts`). The analyzer stays a faithful record of the source
  — do not dedupe there. This is how cross-platform `#[cfg(...)]`-gated
  duplicate commands collapse to one declaration.
- **Parse serde/attribute metadata with `attr.parse_nested_meta(...)`**, the
  AST-based approach (see `serde_parser.rs`), not string matching.
- **Serde naming precedence:** field/parameter-level `#[serde(rename = "...")]`
  overrides container-level `#[serde(rename_all = "...")]`. Command-level
  `rename_all` affects parameters/channels only — never the command (invoke)
  name or the generated function name.
- **Enums:** all-unit-variant enums become string-literal unions / `z.enum`;
  enums with data become discriminated unions / `z.discriminatedUnion`. See
  `is_simple_enum()` / `is_complex_enum()` on `StructInfo`.
- **Output formatting lives in templates.** Change `.tera` files under
  `src/generators/{ts,zod}/templates/` for output shape; keep the two backends
  consistent. Both `commands.ts` functions and the `types.ts` Params interfaces
  derive from the same deduped contexts — don't reintroduce a second source.

## Adding or changing features

- **New tests are required.** Per repo policy: changes are validated against
  existing tests; new features are validated against new tests.
  - Unit tests: `#[cfg(test)]` modules colocated in the relevant `src/**` file.
  - End-to-end tests: `tests/integration_e2e.rs`, using the `TestProject` /
    `TestGenerator` helpers in `tests/common/mod.rs` and fixtures in
    `tests/fixtures/`. Prefer an e2e test when behavior spans analysis +
    generation (e.g. the cfg-dedup regression test).
  - Backward-compatibility tests: `tests/regression/`.
- **Touching the data model** (`models.rs`) ripples into both generators and
  their templates — update all of analysis, both `TypeVisitor` impls, and the
  affected `.tera` files together.
- **Always run the full verify trio** (test + fmt + clippy) before declaring
  done.

## Versioning and changelog

The project follows [Semantic Versioning](https://semver.org/) and keeps a
[Keep a Changelog](https://keepachangelog.com/)-style `CHANGELOG.md`. Git tags
are bare version numbers, no `v` prefix (e.g. `0.5.1`).

**The version source of truth is crates.io, not `Cargo.toml`.** The
`version` in `Cargo.toml` is intentionally stale and is *never* bumped for a
release — do not "update the version" there as part of a change. The release
workflow (`.github/workflows/cd.yml`) reads the latest published version from
crates.io, computes the next one, and overwrites `Cargo.toml` only on the CI
runner (the change is not committed back).

**Releases are manual** via the `CD` workflow's `workflow_dispatch` trigger
with a `major` / `minor` / `patch` choice. The workflow then: fetches the
current crates.io version → bumps it → **extracts release notes from
`CHANGELOG.md` for the new version** → `cargo set-version` → build + test →
`cargo publish` → `gh release create` (tag + GitHub release).

**What this means for you as an agent:**
- When a change is user-facing, add or update a CHANGELOG entry for the
  upcoming version. CD **fails** if no `## [<new_version>] - <date>` section
  exists for the version being released, so the entry must already be in place
  (committed via PR) before a release is run.
- Use the existing section structure: `## [X.Y.Z] - YYYY-MM-DD` with
  `### Added` / `### Changed` / `### Fixed` subsections, newest at the top.
  Match the entry style — a bold lead-in followed by sub-bullets.
- Pick the next version per SemVer relative to the latest crates.io release
  (which is what CD will bump from), not the number in `Cargo.toml`.
- Do **not** create git tags, run `cargo publish`, or bump `Cargo.toml`
  yourself — releasing is the CD workflow's job.

---

# Roadmap (maintainer notes — not active work unless asked)

## Future plans
- When using zod, schema validation should match the parameters in the
  `validator` macro. Example: `#[validate(range(min = 1, max = 10, message = "Must be between 1 and 10"))]`
  → `z.coerce.number().min(1).max(10)`, carrying over the message per
  https://zod.dev/api. (Groundwork: `validator_parser.rs`.)

## Nice to have — Macro-Based Command Discovery

**Concept:** extract commands from `tauri::generate_handler!` invocations
instead of parsing all files for `#[tauri::command]`.

**Benefits:** only analyzes registered commands; better performance; no false
positives from unused commands; single source of truth.

**Approaches:** (1) parse `main.rs`/`lib.rs` for
`tauri::generate_handler![...]`; (2) hook the build/macro-expansion process;
(3) custom proc-macro wrapper around `generate_handler!`; (4) hybrid — macro
parsing first, fall back to file parsing, cross-reference to validate.

**Challenges:** multiple/conditional registration points (plugins), complex
proc-macro analysis, compilation-process integration, dynamic registration.

**Resources:**
- https://docs.rs/tauri/latest/tauri/macro.generate_handler.html
- https://docs.rs/tauri-macros/latest/tauri_macros/
- https://v2.tauri.app/develop/calling-rust/

**Priority:** Low — current file-parsing approach works well for most cases.
