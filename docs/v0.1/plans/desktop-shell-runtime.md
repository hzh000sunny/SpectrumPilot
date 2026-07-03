# Desktop Shell Runtime Implementation Plan

> Runtime storage details in this early shell plan are superseded by `runtime-layout-refactor.md` and `../design/runtime-layout.md`.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the scaffolded Tauri app into a SpectrumPilot desktop shell with real route-based pages and a live local 3GPP runtime status surface.

**Architecture:** Keep path resolution and filesystem setup in Rust Tauri commands. Keep the React UI focused on layout, routing, command invocation, and state presentation. Use `HashRouter` so the packaged desktop app does not depend on server-side route fallback.

**Tech Stack:** Tauri 2, Rust, React 19, TypeScript, Vite, Ant Design, Vitest, Testing Library.

---

## File Structure

| Path | Responsibility |
|---|---|
| `apps/desktop/src-tauri/tauri.conf.json` | SpectrumPilot product name, identifier, window title, and initial size |
| `apps/desktop/src-tauri/src/lib.rs` | Tauri command bridge for app status and runtime path information |
| `apps/desktop/src/App.tsx` | Application providers and router entry |
| `apps/desktop/src/shell/AppShell.tsx` | Management-console layout, sidebar navigation, and top bar |
| `apps/desktop/src/shell/navigation.tsx` | Route metadata and sidebar menu item definitions |
| `apps/desktop/src/pages/*.tsx` | Individual desktop pages |
| `apps/desktop/src/api/runtime.ts` | Typed frontend wrapper around Tauri commands |
| `apps/desktop/src/test/setup.ts` | UI test setup |
| `apps/desktop/src/**/*.test.tsx` | Focused UI behavior tests |

## Task 1: Runtime Path Command

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Test: `apps/desktop/src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing Rust tests**

Add unit tests that assert runtime path construction places 3GPP catalog data under the app cache feature directory.

- [ ] **Step 2: Run the Rust command tests and confirm failure**

Run: `PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop runtime_paths`

Expected: fail because the runtime path helper does not exist yet.

- [ ] **Step 3: Implement the runtime path helper and command**

Add serializable structs for app data, app cache, app log, 3GPP cache, and 3GPP catalog root. Add a `runtime_paths` Tauri command that resolves directories through `AppHandle::path()` and ensures the 3GPP catalog directories exist.

- [ ] **Step 4: Run the Rust command tests and confirm pass**

Run: `PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop runtime_paths`

Expected: pass.

## Task 2: Tauri Branding

**Files:**
- Modify: `apps/desktop/src-tauri/tauri.conf.json`

- [ ] **Step 1: Rename scaffolded metadata**

Set `productName` and window `title` to `SpectrumPilot`, identifier to `com.hzh.spectrumpilot`, and use a wider initial management-console window.

- [ ] **Step 2: Validate config through build/test**

Run: `PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml`

Expected: pass.

## Task 3: React Routing And Shell Split

**Files:**
- Modify: `apps/desktop/package.json`
- Modify: `apps/desktop/vite.config.ts`
- Modify: `apps/desktop/src/App.tsx`
- Create: `apps/desktop/src/shell/AppShell.tsx`
- Create: `apps/desktop/src/shell/navigation.tsx`
- Create: `apps/desktop/src/pages/DashboardPage.tsx`
- Create: `apps/desktop/src/pages/GppPage.tsx`
- Create: `apps/desktop/src/pages/ProposalLibraryPage.tsx`
- Create: `apps/desktop/src/pages/KeywordWatchlistPage.tsx`
- Create: `apps/desktop/src/pages/SettingsPage.tsx`
- Create: `apps/desktop/src/test/setup.ts`
- Create: `apps/desktop/src/shell/AppShell.test.tsx`

- [ ] **Step 1: Add UI test dependencies and script**

Add `vitest`, `jsdom`, `@testing-library/react`, and `@testing-library/jest-dom`. Add `test` script.

- [ ] **Step 2: Write failing shell navigation test**

Assert the app renders SpectrumPilot branding, the 3GPP navigation item, and 3GPP page content by default.

- [ ] **Step 3: Run the UI test and confirm failure**

Run: `npm test -- --run`

Expected: fail before shell routing is implemented.

- [ ] **Step 4: Implement router and page split**

Use `HashRouter`, nested layout routing, and `Navigate` from `/` to `/3gpp`.

- [ ] **Step 5: Run the UI test and confirm pass**

Run: `npm test -- --run`

Expected: pass.

## Task 4: 3GPP Page Runtime Status

**Files:**
- Create: `apps/desktop/src/api/runtime.ts`
- Modify: `apps/desktop/src/pages/GppPage.tsx`
- Test: `apps/desktop/src/pages/GppPage.test.tsx`

- [ ] **Step 1: Write failing UI test for runtime status**

Mock the Tauri `invoke` call and assert the 3GPP page displays the returned catalog path and ready status.

- [ ] **Step 2: Run the UI test and confirm failure**

Run: `npm test -- --run`

Expected: fail because the page does not call the runtime API yet.

- [ ] **Step 3: Implement typed runtime API and page state**

Create a typed `getRuntimeSnapshot()` wrapper and load runtime paths from `GppPage`.

- [ ] **Step 4: Run the UI test and confirm pass**

Run: `npm test -- --run`

Expected: pass.

## Task 5: Full Verification

**Files:**
- All files changed in this plan

- [ ] **Step 1: Run Rust core tests**

Run: `PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core`

Expected: pass.

- [ ] **Step 2: Run desktop Rust tests**

Run: `PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml`

Expected: pass.

- [ ] **Step 3: Run UI tests**

Run: `npm test -- --run`

Expected: pass.

- [ ] **Step 4: Run desktop frontend build**

Run: `npm run build`

Expected: pass.

- [ ] **Step 5: Report changed files, verification results, and remaining gaps**

Report that this slice does not implement live 3GPP indexing, online search, downloading, Windows installer packaging, or updater behavior.
