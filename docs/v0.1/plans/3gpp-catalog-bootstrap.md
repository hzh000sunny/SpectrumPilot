# 3GPP Catalog Seed And Status Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Package a staged 3GPP catalog seed with the desktop app, install it silently into an empty local cache on first launch, and expose catalog/storage status without requiring any user-facing initialization step.

**Architecture:** Keep manifest parsing and JSON storage in `crates/3gpp-core`. Keep live network calls in the Tauri desktop bridge for now. The packaged seed is generated deliberately during development or near release stabilization, then committed under the desktop resources directory. At runtime, the app copies the bundled seed into the local catalog only when the catalog is empty. The main 3GPP page stays focused on search and download; storage diagnostics live in Settings.

**Tech Stack:** Rust, Tauri 2 async commands, `reqwest` with rustls, React, TypeScript, Ant Design, Vitest.

---

## Scope

This slice originally fetched at most seven pages for an early development bootstrap:

- `https://www.3gpp.org/ftp/`
- `https://www.3gpp.org/ftp/tsg_cn/`
- `https://www.3gpp.org/ftp/tsg_ct/`
- `https://www.3gpp.org/ftp/tsg_geran/`
- `https://www.3gpp.org/ftp/tsg_ran/`
- `https://www.3gpp.org/ftp/tsg_sa/`
- `https://www.3gpp.org/ftp/tsg_t/`

That bounded fetch can remain as an internal development helper, but it is not a user workflow. Users should never be asked to click a bootstrap or initialization button after installing SpectrumPilot.

The shipped app should instead include a staged seed generated from local development catalog data. Near the end of a release cycle, maintainers can run one deliberate refresh/backfill pass, review the resulting JSON catalog, and freeze it into that release.

## File Structure

| Path | Responsibility |
|---|---|
| `crates/3gpp-core/src/catalog.rs` | Manifest file naming, manifest writes, and catalog summary |
| `crates/3gpp-core/tests/catalog_tests.rs` | Catalog storage and summary tests |
| `apps/desktop/src-tauri/Cargo.toml` | Desktop network dependencies |
| `apps/desktop/src-tauri/src/lib.rs` | Tauri commands for catalog status, seed installation, search, download, and internal bounded fetch helpers |
| `apps/desktop/src/api/gppCatalog.ts` | Typed frontend wrappers for 3GPP catalog commands |
| `apps/desktop/src/pages/GppPage.tsx` | Search/download workbench and result display |
| `apps/desktop/src/pages/SettingsPage.tsx` | 3GPP storage and seed catalog diagnostics |
| `apps/desktop/src/pages/GppPage.test.tsx` | UI tests for search/download page behavior |
| `apps/desktop/src/pages/SettingsPage.test.tsx` | UI tests for Settings storage diagnostics |

## Task 1: Core Catalog Storage

**Files:**
- Modify: `crates/3gpp-core/src/catalog.rs`
- Modify: `crates/3gpp-core/tests/catalog_tests.rs`

- [ ] **Step 1: Write failing tests**

Add tests for deterministic manifest paths, atomic manifest writes, and catalog summary counts.

- [ ] **Step 2: Run catalog tests and confirm failure**

Run: `PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core --test catalog_tests`

Expected: fail because manifest storage helpers and summary do not exist yet.

- [ ] **Step 3: Implement catalog helpers**

Add:

- `manifest_path_for_url`
- `write_manifest`
- `CatalogSummary`
- `summarize_catalog`

- [ ] **Step 4: Run catalog tests and confirm pass**

Run: `PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core --test catalog_tests`

Expected: pass.

## Task 2: Desktop Catalog Commands And Seed Installation

**Files:**
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Modify: `apps/desktop/src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing Rust tests for fixed bootstrap targets**

Add tests proving the bootstrap target list contains only `/ftp/` plus the six approved `tsg_` root URLs.

- [ ] **Step 2: Run desktop Rust tests and confirm failure**

Run: `PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml bootstrap_targets`

Expected: fail because bootstrap target helper does not exist yet.

- [ ] **Step 3: Implement commands and silent seed installation**

Add:

- `gpp_catalog_status(app: AppHandle)`
- `install_bundled_catalog_seed_if_empty`
- packaged seed manifest constants under desktop resources
- optional/internal `bootstrap_gpp_catalog(app: AppHandle)` helper for development refreshes only
- fixed target URL helper
- live fetch via `reqwest::Client`
- UTC checked-at timestamp
- manifest writes through `3gpp-core`

- [ ] **Step 4: Run desktop Rust tests and confirm pass**

Run: `PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml`

Expected: pass.

## Task 3: Frontend API And UI

**Files:**
- Create: `apps/desktop/src/api/gppCatalog.ts`
- Modify: `apps/desktop/src/pages/GppPage.tsx`
- Modify: `apps/desktop/src/pages/GppPage.test.tsx`
- Modify: `apps/desktop/src/pages/SettingsPage.tsx`
- Modify: `apps/desktop/src/pages/SettingsPage.test.tsx`

- [ ] **Step 1: Write failing UI tests**

Mock `gpp_catalog_status`; assert Settings shows 3GPP catalog paths, manifest counts, record counts, index counts, and bundled seed status. Assert the main 3GPP page does not expose a Bootstrap or initialization button.

- [ ] **Step 2: Run UI tests and confirm failure**

Run from `apps/desktop`: `npm test -- --run src/pages/GppPage.test.tsx src/pages/SettingsPage.test.tsx`

Expected: fail until the Settings diagnostics and main-page removal are implemented.

- [ ] **Step 3: Implement typed API and page controls**

Add status fields in Settings:

- manifest count
- record count
- index count
- last checked time when available
- catalog path
- cache path
- log path
- bundled seed description

- [ ] **Step 4: Run UI tests and confirm pass**

Run from `apps/desktop`: `npm test -- --run src/pages/GppPage.test.tsx src/pages/SettingsPage.test.tsx`

Expected: pass.

## Task 4: Full Verification

**Files:**
- All changed files

- [ ] **Step 1: Run core tests**

Run: `PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core`

Expected: pass.

- [ ] **Step 2: Run desktop Rust tests**

Run: `PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml`

Expected: pass.

- [ ] **Step 3: Run UI tests**

Run from `apps/desktop`: `npm test -- --run`

Expected: pass.

- [ ] **Step 4: Run frontend build**

Run from `apps/desktop`: `npm run build`

Expected: pass.

- [ ] **Step 5: Report remaining gaps**

State that this slice does not yet provide full historical backfill, scheduled incremental refresh, batch downloads, AI research workflows, or Windows installer packaging.
