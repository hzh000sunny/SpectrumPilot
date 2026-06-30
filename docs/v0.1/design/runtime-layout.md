# SpectrumPilot Runtime Layout

This document defines how SpectrumPilot should organize files at runtime on Windows.

## 1. Goal

SpectrumPilot is a desktop application. It should keep application state separate from user-managed workspace content.

The runtime layout should be predictable, easy to back up, and easy to extend as new features are added.

## 2. Two-Layer Model

SpectrumPilot should use two separate locations:

### 2.1 Application State Directory

Used for internal application state that should not be treated as user content.

Recommended location:

```text
%APPDATA%\SpectrumPilot\
```

Suggested subdirectories:

```text
%APPDATA%\SpectrumPilot\
  config\
  db\
  logs\
  cache\
```

This location should store:

- Configuration files.
- SQLite database files.
- Logs.
- Temporary cache data.

### 2.2 Workspace Directory

Used for user-managed business content such as downloaded files and exports.

The workspace root should be configurable, with a sensible default selected during setup or first run.

Example:

```text
D:\SpectrumPilotWorkspace\
```

## 3. Feature-Based Workspace Layout

Inside the workspace root, use one directory per feature.

The first feature directory is `3gpp`.

Example:

```text
D:\SpectrumPilotWorkspace\
  3gpp\
  evidence\
  drafts\
  exports\
```

For v0.1, only `3gpp/` is required.

Example v0.1 layout:

```text
D:\SpectrumPilotWorkspace\
  3gpp\
    meetings\
    tdocs\
    downloads\
    library\
    watchlists\
```

## 4. 3gpp Directory Rules

The `3gpp/` directory should hold all downloaded and organized 3GPP content.

Suggested contents:

| Subdirectory | Purpose |
|---|---|
| `meetings/` | Meeting-level grouping or cached meeting metadata |
| `tdocs/` | Proposal files and TDoc-specific artifacts |
| `downloads/` | Temporary or in-progress downloads |
| `library/` | Normalized local library views or reorganized content |
| `watchlists/` | Saved keyword/company/watch configurations |

The exact internal structure can evolve, but the top-level feature directory should remain stable.

## 5. Principles

- Keep app state out of the workspace.
- Keep user files out of AppData.
- Use feature directories for content that users may inspect, export, or back up.
- Use AppData for content that the user should not need to manage manually.
- Preserve `3gpp` as the feature directory name, not `threegpp`.

## 6. Future Expansion

When future modules are added, they should each get their own workspace directory if they manage user-visible content.

Possible future directories:

```text
evidence/
patents/
drafts/
ppt/
```

The first release should not require these directories to exist.
