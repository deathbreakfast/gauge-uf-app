# gauge-uf-app verification

Re-run after code or doc changes. This workspace is the **Gauge ops UI**
(`gauge-app` / `PermissionRoutes`). Domain contracts live in the sibling
[gauge](https://github.com/unified-field-dev/gauge) repo.

GitHub Actions (`.github/workflows/ci.yml`) runs Layer 1 and Layer 2 on every
pull request and push to `main` / `master`.

## Environment

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-gauge-uf-app
```

## Layer 1 — fmt / clippy / test / check / rustdoc

```bash
cargo fmt -p gauge-app -p gauge-uf-app-e2e -- --check
cargo clippy -p gauge-app --features ssr --all-targets -- \
  -D warnings -A clippy::pedantic -A clippy::nursery
cargo test -p gauge-app --features ssr
cargo check -p gauge-app --features ssr
cargo check -p gauge-uf-app-e2e --features ssr
RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links" cargo doc -p gauge-app --features ssr --no-deps
```

Skip clippy on `gauge-uf-app-e2e` (lab harness uses `expect` heavily). UI compile
and rustdoc are pin-dependent on Orbital / `uf-product`. When those graphs fail,
prefer domain gates in gauge over treating this as a permission API regression.
`gauge-app` uses `#![allow(missing_docs)]` on macro-heavy surfaces; primary-task
teaching lives on the crate-root Features / mount guide.

## Layer 2 — E2E (Playwright)

Host: [`gauge-uf-app-e2e`](../gauge-uf-app-e2e/) on `127.0.0.1:3150`. Scenario
catalog: [`gauge-uf-app-e2e/README.md`](../gauge-uf-app-e2e/README.md). The e2e
package enables Cargo feature `gauge-app/e2e-lab` for seed overrides; production
hosts must not enable `e2e-lab`.

```bash
cd gauge-uf-app-e2e/end2end && npm ci && npx playwright install chromium && cd ../..
cargo leptos end-to-end --project gauge-uf-app-e2e
```

Do not Ctrl-C; the process exits when Playwright finishes. The e2e host mounts
permission pages eagerly (same components as `PermissionRoutes`); production
split hosts still use Lazy + `cargo leptos --split`.

### leptos-lints (CI job `leptos-lints`)

Needs `cargo-dylint` / `dylint-link` 6.0.1 and toolchain `nightly-2025-05-14`
(see `.github/workflows/ci.yml`). Workspace `[workspace.metadata.dylint]` pins
the library; rustc deny names are declared under `[workspace.lints.rust]`.

```bash
# cargo install cargo-dylint --locked --version 6.0.1
# cargo install dylint-link --locked --version 6.0.1
# rustup toolchain install nightly-2025-05-14 --component rustc-dev,llvm-tools-preview
export CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback
export RUSTFLAGS="-D warnings -Zcrate-attr=feature(stdarch_x86_avx512)"
cargo dylint --all -p gauge-app --no-deps -- --features hydrate
```
