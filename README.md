# SpectrumPilot

**Wireless Research Assistant**

SpectrumPilot is a desktop research assistant for wireless industry pre-research workflows. The first release focuses on a complete 3GPP FTP desktop workflow, then leaves room for future AI-assisted research, patent disclosure drafting, and presentation generation.

## Project Status

SpectrumPilot is in the product foundation stage.

The current v0.1 work focuses on:

- Product identity and documentation.
- A Windows desktop application framework.
- Management-console style UI.
- Complete 3GPP Ftp lookup, download, extraction, local cache, seed catalog, and background refresh behavior.
- Windows installer packaging.
- Automatic update support.

No production application build has been released yet.

## First MVP Scope

The first functional version focuses on the 3GPP Ftp workflow because it can deliver practical value without requiring LLM configuration.

v0.1 release scope:

- Desktop shell and Settings.
- 3GPP Ftp query parsing for specifications and TDoc proposals.
- Local catalog seed installation.
- Local indexed lookup before online probing.
- Online fallback search for misses.
- ZIP download, extraction, cache reuse, and document opening.
- Conservative background catalog refresh with visible status.
- Windows installer packaging.
- Automatic update support.

Future modules such as Evidence Search, Patent Drafting, PPT Generator, and LLM Settings are intentionally deferred.

## Technology Direction

The planned stack is:

| Layer | Technology |
|---|---|
| Desktop shell | Tauri 2 |
| UI | React, TypeScript, Vite |
| UI components | Ant Design or equivalent admin UI library |
| Local core | Rust |
| Local database | SQLite |
| Local search | SQLite FTS5 initially |
| Packaging | Tauri Windows installer |
| Updates | Tauri updater plugin |

The target delivery model is a Windows `.exe` installer for Windows 10 and Windows 11. Users should not need to manually install Node.js, Rust, Python, SQLite, or a separate frontend/backend service.

## Documentation

Start here:

- [Documentation Index](./docs/README.md)
- [Documentation Structure](./docs/STRUCTURE.md)
- [v0.1 Product Requirements](./docs/v0.1/prd/spectrumpilot.md)
- [v0.1 Design Overview](./docs/v0.1/design/overview.md)
- [Future AI-Assisted Research Roadmap](./docs/future/ai-assisted-research-roadmap.md)

## Repository Description

```text
A desktop wireless research assistant for 3GPP FTP workflows and future AI-assisted wireless research.
```

## Data Source Notice

SpectrumPilot does not redistribute 3GPP documents or third-party research materials. Users are responsible for complying with the terms of the original data sources.

## License

SpectrumPilot is licensed under the [Apache License 2.0](./LICENSE).
