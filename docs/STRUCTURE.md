# SpectrumPilot Documentation Structure

This document is the authoritative convention for SpectrumPilot documentation.

It describes the documentation system itself. It is not tied to a specific product version.

## 1. Top-Level Structure

```text
docs/
├── README.md
├── STRUCTURE.md
├── rules/
├── future/
├── v0.1/
│   ├── README.md
│   ├── prd/
│   ├── design/
│   ├── plans/
│   └── specs/
├── v0.2/
└── v1.0/
```

Each `vN.M/` directory is a complete product snapshot. Old version directories must remain readable and should not be overwritten by later product changes.

`future/` stores stable product or architecture directions that are not yet assigned to a version.

`rules/` stores stable cross-version decisions that should continue to apply unless explicitly changed.

## 2. Version Directory Naming

Use semantic version directories:

| Format | Example | Meaning |
|---|---|---|
| `vMAJOR.MINOR` | `v0.1`, `v1.0` | Normal product milestone |
| `vMAJOR.MINOR.PATCH` | `v0.1.1` | Optional patch snapshot when a patch must be preserved |

Rules:

- Use a lowercase `v` prefix.
- Use dots between version numbers.
- Do not include dates or topic suffixes in version directory names.

## 3. Version Subdirectories

Each version directory uses these subdirectories:

| Directory | Purpose | Required |
|---|---|---|
| `prd/` | Product requirements: who, why, what, and non-goals | Yes |
| `design/` | System design: how it should be implemented | Yes |
| `plans/` | Implementation plans and acceptance checklists | Optional before implementation |
| `specs/` | Brainstorming notes, decision snapshots, and process artifacts | Optional |

Do not add version numbers or date folders under these subdirectories.

## 4. File Naming

Use lowercase kebab-case file names:

```text
spectrumpilot.md
initial-brainstorm.md
ai-assisted-research-roadmap.md
product-language.md
```

Do not use date prefixes in file names. Version directories and git history carry timeline information.

`README.md` is the only uppercase file-name exception.

## 5. PRD, SDD, Plans, and Specs

| Type | Location | Answers | Change Frequency | Conflict Rule |
|---|---|---|---|---|
| PRD | `vN.M/prd/` | Who, why, what, and non-goals | Low | PRD wins |
| SDD | `vN.M/design/` | How it is built | Medium | Update to match PRD |
| Plans | `vN.M/plans/` | How to execute implementation | High during implementation | Update to match SDD |
| Specs | `vN.M/specs/` | Historical notes and decision records | Append-oriented | Reference only |

If the PRD and design disagree, the PRD is authoritative. The design must be updated.

## 6. Reference Paths

Use relative paths inside docs.

Same-version references:

```text
prd/spectrumpilot.md
design/overview.md
specs/initial-brainstorm.md
```

Cross-version references:

```text
../v0.1/prd/spectrumpilot.md
```

Top-level references from a version directory:

```text
../README.md
../STRUCTURE.md
```

## 7. Version Bump Rules

| Change | Version Handling |
|---|---|
| Product positioning changes, primary workflow changes, or major architecture replacement | Create a new major or minor version directory |
| New significant capability that does not invalidate the current direction | Create a new minor version directory |
| Wording fixes, clarifications, or small alignment updates | Edit in place and rely on git history |
| Public patch snapshot needed | Optionally create a patch version directory |

## 8. Language Rule

SpectrumPilot is an English-first product. Product name, executable name, repository name, menus, settings, dialogs, logs intended for users, and user-facing documentation should be English.

Internal engineering discussion can be bilingual when useful, but stable product documents should prefer English to keep the project internationally readable.
