# 3GPP Background Update Settings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Settings-controlled scheduled 3GPP catalog update switch, persisted default-on configuration, and refresh logging.

**Architecture:** Keep configuration and refresh status in the Tauri layer because the scheduler is desktop runtime behavior. Store user settings under the app data directory, keep catalog state under the catalog root, and write refresh logs under the Tauri app log directory. The React Settings page reads catalog status, toggles the persisted setting through a Tauri command, and shows the log path.

**Tech Stack:** Tauri 2, Rust, React, TypeScript, Ant Design, Vitest.

---

### Task 1: Backend Settings and Logging

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing Rust tests**

Add tests that call helper functions directly:

```rust
#[test]
fn background_refresh_settings_default_to_enabled() {
    let temp = tempfile::tempdir().expect("temp");
    let settings = read_background_refresh_settings(temp.path()).expect("settings");
    assert!(settings.enabled);
    assert_eq!(settings.interval_minutes, GPP_BACKGROUND_REFRESH_INTERVAL_MINUTES);
}

#[test]
fn background_refresh_settings_roundtrip_disabled() {
    let temp = tempfile::tempdir().expect("temp");
    write_background_refresh_settings(
        temp.path(),
        &BackgroundRefreshSettings {
            record_type: "3gpp-background-refresh-settings".to_string(),
            enabled: false,
            interval_minutes: 60,
        },
    )
    .expect("write settings");
    let settings = read_background_refresh_settings(temp.path()).expect("settings");
    assert!(!settings.enabled);
}

#[test]
fn background_refresh_log_appends_lines_to_log_file() {
    let temp = tempfile::tempdir().expect("temp");
    append_background_refresh_log(temp.path(), "started root refresh").expect("log");
    let body = std::fs::read_to_string(background_refresh_log_path(temp.path())).expect("read log");
    assert!(body.contains("started root refresh"));
}
```

- [ ] **Step 2: Run tests and verify failure**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml background_refresh_settings background_refresh_log -- --nocapture
```

Expected: fail because the settings and logging helpers do not exist.

- [ ] **Step 3: Implement minimal Rust helpers**

Add `BackgroundRefreshSettings`, read/write helpers, log path helper, and append helper. Use default `enabled: true`, interval `60`.

- [ ] **Step 4: Wire scheduler behavior**

Update the background loop to read settings before each cycle. If disabled, write status `disabled`, append a skipped log entry, and sleep for the configured interval without touching the network.

- [ ] **Step 5: Run Rust tests**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml background_refresh -- --nocapture
```

Expected: pass.

### Task 2: Settings API and UI

**Files:**
- Modify: `apps/desktop/src/api/gppCatalog.ts`
- Modify: `apps/desktop/src/pages/SettingsPage.tsx`
- Modify: `apps/desktop/src/pages/SettingsPage.test.tsx`

- [ ] **Step 1: Write failing frontend test**

Add a test that renders Settings, expects `Enable scheduled update`, toggles it off, and verifies the Tauri command receives `enabled: false`.

- [ ] **Step 2: Run test and verify failure**

Run:

```bash
cd apps/desktop && npm test -- --run src/pages/SettingsPage.test.tsx
```

Expected: fail because the toggle and setter API do not exist.

- [ ] **Step 3: Implement API and UI**

Add `setGppBackgroundRefreshEnabled(enabled: boolean)` in `gppCatalog.ts`, expose a Tauri command, and render an Ant Design `Switch` plus log path in Settings.

- [ ] **Step 4: Run frontend tests**

Run:

```bash
cd apps/desktop && npm test -- --run src/pages/SettingsPage.test.tsx
```

Expected: pass.

### Task 3: Full Verification

- [ ] **Step 1: Run backend tests**

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml
```

- [ ] **Step 2: Run frontend tests and build**

```bash
cd apps/desktop && npm test -- --run
cd apps/desktop && npm run build
```

- [ ] **Step 3: Run formatting and diff checks**

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo fmt --all --check
git diff --check
```
