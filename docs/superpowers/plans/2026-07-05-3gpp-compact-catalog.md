# 3GPP Compact Catalog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the 3GPP catalog from verbose sharded JSON into a compact release seed, adapt lookup to read it first, and add catalog download/install state so the full seed can be fetched asynchronously after install.

**Architecture:** Keep the existing sharded JSON format as an overlay for runtime discoveries and background refresh. Add a compact read-only active seed under `catalog/compact`, with prefix/year index shards pointing into workgroup record shards. Lookup checks overlay legacy shards first, then compact seed, then online fallback. The packaged app keeps a small bootstrap seed while release metadata can point to a downloadable full compact catalog.

**Tech Stack:** Python unittest conversion tooling, Rust serde-based catalog reader in `spectrum-3gpp-core`, Tauri commands/state in Rust, React/TypeScript settings display, Vitest and Cargo tests.

---

### Task 1: Compact Seed Conversion Tool

**Files:**
- Create: `scripts/3gpp/build_compact_seed.py`
- Create: `scripts/3gpp/test_build_compact_seed.py`
- Modify: `docs/v0.1/features/3gpp-search-download.md`

- [ ] Write Python tests that build a tiny legacy seed with two RAN2 records and verify compact output has one workgroup record shard, one prefix/year index shard, deterministic counts, and URL reconstruction inputs.
- [ ] Run `python3 -m unittest discover -s scripts/3gpp -p 'test_build_compact_seed.py'` and verify the test fails because the script is missing.
- [ ] Implement `build_compact_seed.py` to read `records/tdoc/**.json`, produce `compact/records/<WG>.json`, `compact/index/<PREFIX>_<YY>.json`, `compact/summary.json`, and `seed.json` metadata.
- [ ] Run the Python test and verify it passes.
- [ ] Run the conversion against the existing 350MB baseline with no network access.

### Task 2: Rust Compact Catalog Model and Reader

**Files:**
- Create: `crates/3gpp-core/src/compact.rs`
- Modify: `crates/3gpp-core/src/lib.rs`
- Modify: `crates/3gpp-core/src/catalog.rs`
- Create: `crates/3gpp-core/tests/compact_catalog_tests.rs`

- [ ] Write Rust tests for reading a compact seed, resolving `R2-2601401`, reconstructing the canonical HTTPS URL, and summarizing compact counts.
- [ ] Run `cargo test -p spectrum-3gpp-core compact_catalog` and verify failure due to missing compact module.
- [ ] Implement compact structs and `resolve_tdoc_from_compact_catalog` with index-shard-first reads.
- [ ] Add compact-aware summary fields without breaking existing `CatalogSummary` callers.
- [ ] Run the compact Rust tests and existing core tests.

### Task 3: Lookup Workflow Integration

**Files:**
- Modify: `apps/desktop/src-tauri/src/gpp/workflow.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src-tauri/src/gpp/workflow.rs` tests

- [ ] Add a failing desktop backend test showing lookup resolves from compact catalog when legacy index shards are absent.
- [ ] Implement local resolution order: legacy overlay index first, compact active seed second, online fallback third.
- [ ] Ensure direct-probe hits and online discoveries still write to legacy overlay shards, not compact active seed.
- [ ] Run desktop backend tests for lookup workflow and seed install.

### Task 4: Async Catalog Seed Download State Skeleton

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src/api/gppCatalog.ts`
- Modify: `apps/desktop/src/pages/SettingsPage.tsx`
- Modify: `apps/desktop/src/pages/SettingsPage.test.tsx`

- [ ] Add tests for catalog download state values `not_installed`, `downloading`, `ready`, and `failed` in Settings data.
- [ ] Add `catalog-download.json` state file with source URL, version, last attempt, last success, error, downloaded bytes, expected bytes, and sha256.
- [ ] Add startup background task skeleton that marks downloadable catalog state and supports local/file/http URL configuration, but does not require network for tests.
- [ ] Update Settings UI to show catalog install/download state separately from scheduled refresh.
- [ ] Run frontend Settings tests.

### Task 5: Generate Compact Baseline and Verify Counts

**Files:**
- Generate: `apps/desktop/src-tauri/resources/3gpp/catalog_seed/compact/**`
- Modify: `apps/desktop/src-tauri/resources/3gpp/catalog_seed/seed.json`
- Modify: release seed tests as needed

- [ ] Run compact conversion using `/home/hzh/.config/superpowers/worktrees/SpectrumPilot/3gpp-v01-completion/apps/desktop/src-tauri/resources/3gpp/catalog_seed` or `/tmp/spectrumpilot-3gpp-2024-2026-catalog` as input.
- [ ] Verify compact output record count equals 215,379 and index items equal 215,338.
- [ ] Measure compact size and compare against 350MB baseline.
- [ ] Keep the packaged bootstrap small unless explicitly choosing to commit the full compact seed into `resources`.

### Task 6: Full Verification

**Files:**
- All touched files

- [ ] Run `python3 -m unittest discover -s scripts/3gpp -p 'test_*.py'`.
- [ ] Run `PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core`.
- [ ] Run `PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [ ] Run `cd apps/desktop && npm install` if dependencies are absent, then `npm test -- --run`.
- [ ] Run `PATH=/home/hzh/.cargo/bin:$PATH cargo fmt --all --check`.
- [ ] Run `git diff --check`.
