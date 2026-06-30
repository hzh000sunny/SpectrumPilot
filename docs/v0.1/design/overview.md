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
| Local database | SQLite | Proposal metadata, meeting records, download history, settings, search index |
| Search | SQLite FTS5 initially | Local search over proposal numbers, titles, companies, meetings, and keywords |
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
  │  ├─ Dashboard
  │  ├─ TDoc Downloader
  │  ├─ Batch Download
  │  ├─ Meeting Browser
  │  ├─ Proposal Library
  │  ├─ Keyword Watchlist
  │  └─ Settings
  │
  ├─ Tauri Command Layer
  │  ├─ Download commands
  │  ├─ Library commands
  │  ├─ Settings commands
  │  └─ Update commands
  │
  ├─ Rust Core
  │  ├─ 3GPP downloader
  │  ├─ Meeting and TDoc metadata parser
  │  ├─ Download queue
  │  ├─ File storage manager
  │  └─ Logging
  │
  └─ Local Storage
     ├─ SQLite database
     ├─ App config
     ├─ Logs
     └─ Downloaded proposal files
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
│   ├── threegpp-core/
│   └── local-index/
└── tests/
```

This layout keeps the desktop UI and Tauri application under `apps/desktop/`, while reusable Rust logic can live under `crates/`.

## 5. Data and Storage

Application data should live under the normal Windows application data directory:

```text
%APPDATA%/SpectrumPilot/
  app.db
  config.json
  logs/
```

Downloaded proposal files should use a user-configurable directory. A practical default can be selected during first-run setup or in Settings.

Example:

```text
D:\3GPP\
  RAN1\
    R1-2401234.*
  RAN2\
    R2-2404551.*
```

The database should store file paths and metadata, not duplicate file contents.

## 6. UI Direction

The UI should feel like a professional management console rather than a marketing website.

Key rules:

- Left sidebar is the primary navigation.
- Top bar shows current module context and global actions.
- Main content area changes by selected page.
- Tables, filters, progress states, and settings pages should be dense and work-focused.
- Future AI modules may appear as disabled or reserved navigation items, but v0.1 should not require LLM setup.

Initial sidebar groups:

| Group | Items |
|---|---|
| 3GPP | Dashboard, TDoc Downloader, Batch Download, Meeting Browser, Proposal Library, Keyword Watchlist |
| Future AI | Evidence Search, Patent Drafting, PPT Generator |
| System | Settings, Update, Logs |

## 7. 3GPP Core Boundary

The 3GPP downloader should not be embedded directly in UI components.

It should be implemented behind a local command/core boundary:

```text
React page
  ↓ invokes
Tauri command
  ↓ calls
threegpp-core
  ↓ writes
SQLite metadata + local proposal files
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

The preferred long-term state is Rust core logic under `threegpp-core`.

## 9. Testing Direction

v0.1 should include tests at the level appropriate for the implementation:

- Unit tests for TDoc number parsing, URL construction, and metadata normalization.
- Integration tests for download queue behavior with mocked network responses.
- SQLite tests for proposal metadata storage and search.
- UI tests for navigation, form submission, queue state, and library filtering.
- Packaging smoke test for installer build once the Tauri project exists.

## 10. Open Implementation Inputs

The following inputs are needed before detailed implementation planning:

- Existing downloader screenshot and implementation form.
- 3GPP URL and meeting/TDoc source rules used by the current tool.
- Preferred default download directory behavior.
- Whether the first build should use Ant Design or another admin UI component library.

These are planning inputs, not blockers for the current product foundation documentation.
