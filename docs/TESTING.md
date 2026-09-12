# Testing

<!-- constrained-by ../PROJECT_STRUCTURE.md#Testing -->

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

- **Unit tests** live next to the code (`#[cfg(test)]` in `src/`).
- **Integration tests** live in `tests/` (CLI handlers, WebUI routes, DB migrations).
- JJ-dependent tests need `jj` on `PATH` (installed in CI). Tests that need a real TTY are `#[ignore]` or skip when stdin is not a TTY.

`cargo test` currently runs on the order of 270 tests. Do not copy counts into this file; the suite size changes often.

WebUI route tests use `axum` + `tower::ServiceExt` against `build_router` with an in-memory database.

Skill/plugin packaging: `python3 scripts/validate_package.py`.
