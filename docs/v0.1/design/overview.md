# SpectrumPilot v0.1 Design Overview

This document describes the implementation direction for SpectrumPilot v0.1.

The product contract is defined in `../prd/spectrumpilot.md`. If this design conflicts with the PRD, update the design to match the PRD.

## 1. Architecture Summary

SpectrumPilot v0.1 should be a Windows desktop application built with:

| Layer | Technology | Purpose |
|---|---|---|
| Desktop shell | Tauri 2 | Windows desktop application, installer, local system integration, updater |
| UI | React, TypeScript, Vite | Management-console interface |
| UI components | Ant Design or equivalent admin UI library | Tables, forms, layout, navigation, modals, settings |
| Local core | Rust | 3GPP download logic, file operations, task queue, configuration, logs |
| Local catalog | JSON seed, manifests, shards, and lookup indexes | 3GPP FTP directory metadata, TDoc records, and fast local lookup |
| Optional local database | SQLite | Future proposal library, download history, settings, and full-text search when needed |
| Packaging | Tauri bundler | Windows `.exe` installer |
| Updates | Tauri updater plugin | Automatic update channel for later releases |

The UI is bundled into the desktop application. It is not deployed as a website.

## 2. Delivery Model

SpectrumPilot should be installed through a Windows installer.

End users should not manually install:

- Node.js.
- Rust.
- Python.
- SQLite.
- A backend server.
- A frontend server.

Windows WebView2 can be handled by the installer. Windows 10 and Windows 11 are the target operating systems.

## 3. Logical Components

```text
SpectrumPilot Desktop App
  ├─ React Management Console
  │  ├─ Sidebar navigation
  │  ├─ Top bar
  │  ├─ 3GPP Ftp
  │  └─ Settings
  │
  ├─ Tauri Command Layer
  │  ├─ 3GPP lookup/download commands
  │  ├─ Catalog status commands
  │  ├─ Runtime path commands
  │  ├─ Settings commands
  │  └─ Update commands
  │
  ├─ Rust Core
  │  ├─ 3GPP query parser and resolver
  │  ├─ 3GPP catalog seed installer
  │  ├─ 3GPP background refresh
  │  ├─ ZIP downloader/extractor/opener
  │  └─ Logging
  │
  └─ Local Storage
     ├─ 3GPP catalog JSON
     ├─ User workspace
     ├─ App config
     └─ Logs
```

## 4. Repository Shape

The expected repository name is `SpectrumPilot`.

Initial repository layout:

```text
SpectrumPilot/
├── README.md
├── docs/
│   ├── README.md
│   ├── STRUCTURE.md
│   ├── rules/
│   ├── future/
│   └── v0.1/
├── apps/
│   └── desktop/
│       ├── src/
│       ├── src-tauri/
│       ├── package.json
│       └── vite.config.ts
├── crates/
│   └── 3gpp-core/
└── tests/
```

This layout keeps the desktop UI and Tauri application under `apps/desktop/`, while reusable Rust logic can live under `crates/`.

## 5. Data and Storage

SpectrumPilot separates internal application storage from the user workspace.

```text
%APPDATA%/SpectrumPilot/
  config/
  metadata/
  cache/
  logs/
```

The 3GPP catalog lives under internal application metadata. Downloaded proposal and specification files use the user workspace directory. The detailed runtime layout is defined in `runtime-layout.md`.

Example workspace root:

```text
D:\SpectrumPilotWorkspace\
  3gpp\
```

The catalog stores metadata and file paths, not duplicate 3GPP document contents.

## 6. UI Direction

The UI should feel like a professional management console rather than a marketing website.

Key rules:

- Left sidebar is the primary navigation.
- Top bar shows current module context and global actions.
- Main content area changes by selected page.
- Tables, filters, progress states, and settings pages should be dense and work-focused.
- v0.1 navigation should stay focused on the active 3GPP Ftp and Settings surfaces. Future AI modules should not appear as v0.1 navigation items or placeholders.

v0.1 sidebar groups:

| Group | Items |
|---|---|
| 3GPP | 3GPP Ftp |
| System | Settings |

## 7. 3GPP Core Boundary

The 3GPP downloader should not be embedded directly in UI components.

It should be implemented behind a local command/core boundary:

```text
React page
  ↓ invokes
Tauri command
  ↓ calls
3gpp-core + desktop workflow
  ↓ writes
catalog JSON + workspace files
```

This makes it possible to test downloader behavior without rendering the UI and to replace an early wrapped implementation with a Rust implementation later.

## 8. Existing Tool Migration

Before implementing the TDoc Downloader, the existing proposal-number downloader should be reviewed.

Migration options:

| Option | When to Use |
|---|---|
| Wrap existing tool as a sidecar | Existing logic is reliable and rewriting would slow the first milestone |
| Reimplement in Rust | URL rules and parsing are straightforward enough to make the new core cleaner |
| Temporary bridge, then rewrite | Existing logic is useful for validation but should not become long-term architecture |

The preferred long-term state is Rust core logic under `3gpp-core`, with desktop-only file, opener, installer, and updater behavior kept in the Tauri layer.

## 9. Testing Direction

v0.1 should include tests at the level appropriate for the implementation:

- Unit tests for TDoc number parsing, URL construction, and metadata normalization.
- Integration tests for download queue behavior with mocked network responses.
- Catalog tests for manifest, shard, index, and seed behavior.
- UI tests for navigation, form submission, progress state, Settings, and catalog status display.
- Packaging smoke test for Windows installer and updater behavior.

## 10. Open Implementation Inputs

The following inputs are needed before detailed implementation planning:

- Final Windows installer format and signing approach.
- Automatic update channel and release hosting details.
- Release seed coverage target for the final v0.1 build.
- Windows 10/11 validation environment.

These are planning inputs, not blockers for the current product foundation documentation.
