# SpectrumPilot Documentation — v0.1

| Field | Value |
|---|---|
| Version | v0.1 |
| Start Date | 2026-06-30 |
| Status | Draft |
| Theme | Product foundation and 3GPP-first MVP |

## Directory Structure

```text
v0.1/
├── README.md
├── prd/
│   └── spectrumpilot.md
├── design/
│   ├── overview.md
│   ├── runtime-layout.md
│   └── 3gpp-local-index.md
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
| Understand early decisions | `specs/initial-brainstorm.md` | `prd/spectrumpilot.md` |
| Prepare implementation | `prd/spectrumpilot.md` | `design/overview.md`, then future implementation plans |

## Version Scope

v0.1 establishes the application identity, desktop delivery model, management-console UI direction, technology stack, and first implementation area.

The first implementation area is the 3GPP workflow because it can be built without LLM configuration and can reuse or replace the existing proposal-number-based downloader.

## Current Decisions

| Decision | Value |
|---|---|
| Product name | `SpectrumPilot` |
| Subtitle | `Wireless Research Assistant` |
| Target OS | Windows 10 and Windows 11 |
| Delivery model | Desktop installer, not web deployment |
| Primary stack | Tauri 2, React, TypeScript, Rust, SQLite |
| UI style | Management-console style with sidebar navigation and top bar |
| v0.1 functional focus | 3GPP proposal and TDoc workflows |
| AI scope | Future, not v0.1 |

The 3GPP design work now splits search into two modes:

- foreground user queries should return as fast as possible and may search in parallel
- background index refresh should stay incremental, conservative, and low-volume

## PRD vs Design

The PRD in `prd/spectrumpilot.md` is the product contract. The design in `design/overview.md` describes how the product should be implemented.

If they conflict, update the design to match the PRD.
