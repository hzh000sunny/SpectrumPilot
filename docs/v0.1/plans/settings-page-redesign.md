# Settings Page Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign Settings into a compact management-console page focused on scheduled update status, catalog health, workspace, and internal storage paths.

**Architecture:** Keep backend data access behind typed runtime and 3GPP APIs. Render `System` with workspace and application storage, and render `3GPP Ftp` with `Scheduled Update`, `Catalog`, and `Data Locations`.

**Tech Stack:** React, TypeScript, Ant Design, Vitest, Playwright screenshots.

---

## Follow-up IA Decision

The Settings page uses an internal secondary sidebar. The global app sidebar remains unchanged; the Settings content area is split into a narrow text-only sidebar and a main detail pane.

- Sidebar width target: about 148px on desktop.
- Sidebar labels are text-only, with no icons and no group headings, to preserve detail-pane width.
- Default active section: `3GPP Ftp`, because v0.1 settings are primarily for the 3GPP catalog and refresh workflow.
- Current sections: `System` and `3GPP Ftp`.
- Workspace paths are part of `System`, not a separate section.
- Future AI and LLM provider settings are intentionally not rendered in v0.1. The internal sidebar structure leaves room for those sections later without exposing unfinished AI configuration now.

### Task 1: UI Contract Tests

**Files:**
- Modify: `apps/desktop/src/pages/SettingsPage.test.tsx`

- [ ] Add tests that assert Settings shows `Scheduled Update`, `Catalog`, and `Data Locations`.
- [ ] Assert the redundant inner `Settings` heading is gone.
- [ ] Assert the scheduled update switch remains accessible by name.
- [ ] Assert the refresh log path is shown under storage paths.

### Task 2: Component Refactor

**Files:**
- Modify: `apps/desktop/src/pages/SettingsPage.tsx`

- [ ] Replace the repeated card grid with three section blocks.
- [ ] Use compact summary tiles only for `Manifests`, `Indexed TDocs`, and `Index shards`.
- [ ] Use status pills for scheduled update state and last error.
- [ ] Keep the existing backend API and settings toggle behavior.

### Task 3: Styling

**Files:**
- Modify: `apps/desktop/src/App.css`

- [ ] Add Settings-specific grid and panel classes.
- [ ] Fix the scheduled update switch so it keeps native width.
- [ ] Render path rows with monospace text, truncation, and full value in `title`.
- [ ] Keep desktop layout dense and mobile layout single-column.

### Task 4: Verification

- [ ] Run `npm test -- --run src/pages/SettingsPage.test.tsx`.
- [ ] Run `npm test -- --run`.
- [ ] Run `npm run build`.
- [ ] Capture a Settings screenshot at `/#/settings`.
