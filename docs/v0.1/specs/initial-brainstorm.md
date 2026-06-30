# Initial Brainstorm Snapshot

## Date

2026-06-30.

## Product Direction

The project is a new desktop tool for wireless industry pre-research workflows.

The long-term idea includes:

- Downloading wireless standards meeting proposals from 3GPP.
- Searching related journals, papers, patents, and evidence based on clues or proposals.
- Generating patent disclosure drafts.
- Generating PPT decks and research briefs.

## Scope Decomposition

The full vision is too broad for one initial implementation. It should be decomposed into stages.

The first stage should establish:

- The desktop application framework.
- The management-console UI.
- The 3GPP workflow.
- Local file and metadata management.

AI-dependent features should be deferred because they require LLM setup, provider configuration, prompt design, output validation, and evidence control.

## Product Name Decision

Finalized names:

| Item | Value |
|---|---|
| Project and executable name | `SpectrumPilot` |
| Product subtitle | `Wireless Research Assistant` |

Reasoning:

- `Spectrum` signals the wireless domain.
- `Pilot` signals assistant/workbench behavior.
- The name is broader than `3GPP` or `TDoc`, leaving room for future evidence search, patent drafting, and PPT generation.
- The same name can be used for the GitHub repository, installer, executable, and application title.

## UI Direction

The UI should resemble a management console:

- Left sidebar for feature navigation.
- Top navigation bar for context and global actions.
- Main workspace changes when the user selects a sidebar item.
- First active pages should focus on 3GPP.
- Future AI capabilities can appear as reserved navigation areas but should not be active in v0.1.

## Technology Direction

Recommended stack:

| Layer | Technology |
|---|---|
| Desktop shell | Tauri 2 |
| UI | React, TypeScript, Vite |
| UI components | Ant Design or similar admin-oriented component library |
| Local core | Rust |
| Local database | SQLite |
| Local search | SQLite FTS5 initially |
| Packaging | Tauri Windows installer |
| Updater | Tauri updater plugin |

Windows 10 and Windows 11 are the target platforms. It is acceptable for the installer to handle WebView2 automatically.

## First Active Module

The first active module should be 3GPP because it is useful and does not require AI.

Initial 3GPP feature areas:

- TDoc Downloader.
- Batch Download.
- Meeting Browser.
- Proposal Library.
- Keyword Watchlist.
- Settings.

## Existing Tool

There is already a tool that can download proposals by entering proposal numbers.

The existing tool should be reviewed before implementing the new downloader. The implementation can either reuse, wrap, or rewrite its core logic depending on complexity and reliability.

## Deferred Modules

Deferred modules:

- Evidence Search.
- Patent Drafting.
- PPT Generator.
- LLM Settings.

These are captured in `../../future/ai-assisted-research-roadmap.md`.
