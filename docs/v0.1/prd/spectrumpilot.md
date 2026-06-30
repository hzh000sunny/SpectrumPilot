# SpectrumPilot Product Requirements Document

| Field | Value |
|---|---|
| Version | v0.1 |
| Start Date | 2026-06-30 |
| Status | Draft |
| Product Name | SpectrumPilot |
| Subtitle | Wireless Research Assistant |
| Target Scope | Product foundation and 3GPP-first MVP |

## About This Document

This PRD describes who SpectrumPilot serves, why it exists, what v0.1 should deliver, and what v0.1 should not deliver.

It does not replace the system design documents under `design/`. When the PRD and design conflict, this PRD is authoritative.

## 1. Product Positioning

SpectrumPilot is a desktop research assistant for wireless industry pre-research workflows.

It helps wireless researchers, standardization engineers, and patent/research contributors collect, organize, search, and later analyze wireless technical materials.

The long-term product should support:

- 3GPP proposal and TDoc acquisition.
- Proposal, meeting, company, work item, and keyword tracking.
- Evidence-backed literature and patent research.
- Patent disclosure drafting.
- PPT and research brief generation.

v0.1 focuses on the product foundation and 3GPP workflow only.

## 2. Target Users

| User | Need |
|---|---|
| Wireless pre-research engineer | Track technical proposals and prepare research material |
| 3GPP standards engineer | Download, organize, and search meeting proposals |
| Patent contributor | Collect proposal clues and evidence for later invention drafting |
| Research team lead | Review proposal trends, technical directions, and research outputs |

## 3. Core Value Proposition

SpectrumPilot should become a local desktop workbench for wireless research material.

The first version should prove that the tool can provide a clean desktop application frame and a practical 3GPP workflow without requiring AI configuration.

## 4. Product Identity

| Item | Decision |
|---|---|
| GitHub repository name | `SpectrumPilot` |
| Executable name | `SpectrumPilot.exe` |
| Product name | `SpectrumPilot` |
| Product subtitle | `Wireless Research Assistant` |
| Language | English-first |

The product name and executable name should match. UI text should be English to support international positioning.

## 5. Delivery Model

SpectrumPilot is a Windows desktop application.

v0.1 targets:

- Windows 10.
- Windows 11.
- `.exe` installer delivery.
- No manual dependency installation for end users.
- Installer-managed WebView2 handling.
- Automatic update support as an architectural requirement.

SpectrumPilot is not a website deployment. Users should not need to deploy a frontend, backend, database, or runtime environment.

## 6. v0.1 Functional Scope

v0.1 should establish the framework for the full product and implement the first non-AI workflow around 3GPP material.

### 6.1 Application Shell

The application should use a management-console layout:

- Left sidebar for feature navigation.
- Top navigation bar for current context and global actions.
- Main work area for each feature page.
- Page switching by sidebar navigation.

### 6.2 First Active Module: 3GPP Radar

The first active module is the 3GPP workflow.

Initial pages:

| Page | Purpose |
|---|---|
| Dashboard | Show download state, recent proposals, and local library summary |
| TDoc Downloader | Download proposals by TDoc/proposal number |
| Batch Download | Download multiple proposal numbers with queue, progress, retry, and failure state |
| Meeting Browser | Browse 3GPP meetings and related TDoc lists |
| Proposal Library | Search and manage locally downloaded proposal metadata and files |
| Keyword Watchlist | Track keywords, companies, work items, or meeting topics |
| Settings | Configure download path, proxy, concurrency, update channel, and logs |

### 6.3 Existing Downloader Integration

There is an existing tool that can download proposals by inputting proposal numbers.

v0.1 should inspect the existing tool before implementation and decide whether to:

- Reuse its logic temporarily.
- Wrap it as a sidecar.
- Reimplement the downloader in Rust.

The long-term target is to keep 3GPP download and metadata logic outside UI components.

## 7. Out of Scope for v0.1

The following are intentionally excluded from v0.1 implementation:

- LLM provider configuration.
- AI-assisted proposal analysis.
- Literature, journal, and patent search.
- Patent disclosure generation.
- PPT generation.
- Multi-user collaboration.
- Cloud deployment.
- Web-hosted SaaS deployment.

These capabilities are preserved in `../../future/ai-assisted-research-roadmap.md`.

## 8. Success Criteria

v0.1 is successful if:

- The project has a clean desktop application foundation.
- The installer can install and launch SpectrumPilot on Windows 10/11 without manual runtime setup.
- The UI uses a management-console layout suitable for repeated professional use.
- The 3GPP module can provide a clear path from proposal number input to local file/library management.
- Future AI modules have reserved navigation and architecture space but do not block 3GPP usage.

## 9. Product Non-Goals

SpectrumPilot should not become:

- A generic web portal requiring server deployment.
- A narrow one-off TDoc downloader with no room for research workflows.
- A chat-only AI wrapper.
- A tool that hides sources behind generated summaries.

## 10. Documentation Maintenance

This PRD belongs to `docs/v0.1/`.

Documentation structure and versioning rules are defined in `../../STRUCTURE.md`.

When the product scope changes materially, create a new version directory rather than silently rewriting the historical snapshot.
