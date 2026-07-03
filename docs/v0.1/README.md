# SpectrumPilot Documentation — v0.1

| Field | Value |
|---|---|
| Version | v0.1 |
| Start Date | 2026-06-30 |
| Status | Draft |
| Theme | Product foundation, complete 3GPP Ftp workflow, installer, and updater |

## Directory Structure

```text
v0.1/
├── README.md
├── prd/
│   └── spectrumpilot.md
├── design/
│   ├── overview.md
│   ├── runtime-layout.md
│   ├── 3gpp-local-index.md
│   └── 3gpp-fast-lookup-download.md
├── features/
│   └── 3gpp-search-download.md
├── plans/
│   └── README.md
└── specs/
    └── initial-brainstorm.md
```

## Recommended Reading Order

| Goal | Read First | Then Read |
|---|---|---|
| Understand the product | `prd/spectrumpilot.md` | `design/overview.md` |
| Understand architecture | `design/overview.md` | `design/runtime-layout.md`, `design/3gpp-local-index.md` |
| Understand the implemented 3GPP slice | `features/3gpp-search-download.md` | `design/3gpp-local-index.md` |
| Understand the next 3GPP workflow upgrade | `design/3gpp-fast-lookup-download.md` | `features/3gpp-search-download.md` |
| Understand early decisions | `specs/initial-brainstorm.md` | `prd/spectrumpilot.md` |
| Prepare implementation | `prd/spectrumpilot.md` | `design/overview.md`, then future implementation plans |

## Version Scope

v0.1 establishes the application identity, desktop delivery model, management-console UI direction, technology stack, complete 3GPP Ftp workflow, and install/update path.

The first implementation area is the 3GPP Ftp workflow because it can be built without LLM configuration and can reuse or replace the existing proposal-number-based downloader.

## Current Decisions

| Decision | Value |
|---|---|
| Product name | `SpectrumPilot` |
| Subtitle | `Wireless Research Assistant` |
| Target OS | Windows 10 and Windows 11 |
| Delivery model | Desktop installer, not web deployment |
| Primary stack | Tauri 2, React, TypeScript, Rust, SQLite |
| UI style | Management-console style with sidebar navigation and top bar |
| v0.1 functional focus | Complete 3GPP Ftp workflow, desktop framework, Windows installer, and updater |
| AI scope | Future, not v0.1 |

The 3GPP design work now splits search into two modes:

- foreground user queries should return as fast as possible and may search in parallel
- background index refresh should stay incremental, conservative, and low-volume

The shipped product also uses a bundled seed catalog so first launch does not require the user to initialize anything manually.

## PRD vs Design

The PRD in `prd/spectrumpilot.md` is the product contract. The design in `design/overview.md` describes how the product should be implemented.

If they conflict, update the design to match the PRD.
