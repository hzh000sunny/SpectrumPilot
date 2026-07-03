# Runtime Layout Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor SpectrumPilot runtime storage into a clear internal application storage root and a user workspace root, then copy existing 3GPP catalog/settings/log data from the older Tauri default directories into the new layout without deleting legacy files.

**Architecture:** Add a single runtime layout builder in the Tauri layer and route all app-owned 3GPP metadata, config, cache, and logs through it. Keep downloaded and extracted user documents under the workspace root. Settings should present `Workspace` first and `Application Storage` as internal data, with 3GPP catalog paths described as internal metadata.

**Tech Stack:** Tauri 2, Rust, React, TypeScript, Vitest, cargo tests.

---

### Task 1: Runtime Layout Contract Tests

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`

- [x] Add tests proving Linux-style storage collapses under `~/.local/share/SpectrumPilot` instead of exposing both `.local/share/com.hzh.spectrumpilot` and `.cache/com.hzh.spectrumpilot`.
- [x] Add tests proving 3GPP catalog root becomes `<app-storage>/metadata/3gpp/catalog`.
- [x] Add tests proving refresh log becomes `<app-storage>/logs/3gpp-refresh.log`.
- [x] Add tests proving default workspace root remains `<home>/SpectrumPilotWorkspace`.
- [x] Add migration tests for old catalog, settings, and refresh log files.
- [x] Add migration safety tests proving legacy files are retained and existing new targets are not overwritten.
- [x] Add a first-run app settings test proving the default workspace is persisted under `<app-storage>/config/settings.json`.

### Task 2: Runtime Layout Implementation

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`

- [x] Add a `RuntimeLayout` struct with application storage, config, metadata, cache, logs, workspace, 3GPP workspace, 3GPP catalog, and refresh log paths.
- [x] Add `build_runtime_layout(home_dir, legacy_app_data_dir, legacy_app_cache_dir)` for deterministic tests and runtime use.
- [x] Add `migrate_runtime_layout(layout)` that copies old 3GPP catalog, `config/3gpp-settings.json`, and `logs/3gpp-refresh.log` into the new layout when new targets are absent.
- [x] Replace direct app data/cache/log path usage in 3GPP catalog/status/background refresh paths with `RuntimeLayout`.
- [x] Persist first-run app settings so the default workspace path is visible as an application setting.

### Task 3: API and UI Contract

**Files:**
- Modify: `apps/desktop/src/api/runtime.ts`
- Modify: `apps/desktop/src/pages/SettingsPage.tsx`
- Modify: `apps/desktop/src/pages/SettingsPage.test.tsx`

- [x] Extend `RuntimePaths` to expose `appStorageDir`, `configDir`, `metadataDir`, `internalCacheDir`, `logsDir`, `workspaceRoot`, and `threeGppWorkspaceDir`.
- [x] Keep existing fields only if needed during transition; do not emphasize legacy path labels in the UI.
- [x] Update Settings System to show `Workspace` and `Application Storage`.
- [x] Update 3GPP Ftp Settings to label catalog as internal metadata and documents as workspace content.

### Task 4: Verification

- [x] Run `cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml runtime_layout`.
- [x] Run `cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Run `cargo test -p spectrum-3gpp-core`.
- [x] Run `cd apps/desktop && npm test -- --run`.
- [x] Run `cd apps/desktop && npm run build`.
- [x] Run `PATH=/home/hzh/.cargo/bin:$PATH cargo fmt --all --check`.
- [x] Run `git diff --check`.
- [x] Capture Settings screenshots after implementation:
  - `apps/desktop/tmp/settings-runtime-layout-gpp-1440.png`
  - `apps/desktop/tmp/settings-runtime-layout-system-1440.png`
