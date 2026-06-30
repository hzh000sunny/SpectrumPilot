# SpectrumPilot Documentation

## Current Version

[**v0.1**](./v0.1/) — Product foundation and 3GPP-first MVP planning. Status: Draft.

## Future Planning

[**future/**](./future/) — Long-term capabilities that are not part of the first implementation scope, including AI-assisted evidence search, patent disclosure drafting, and PPT generation.

## Project-Level Rules

[**rules/**](./rules/) — Cross-version rules that should remain stable across product iterations.

## Documentation Convention

The documentation structure, versioning rules, file naming rules, and PRD/SDD relationship are defined in [STRUCTURE.md](./STRUCTURE.md).

## Quick Map

```text
docs/
├── README.md
├── STRUCTURE.md
├── rules/
│   ├── README.md
│   └── product-language.md
├── future/
│   ├── README.md
│   └── ai-assisted-research-roadmap.md
└── v0.1/
    ├── README.md
    ├── prd/
    │   └── spectrumpilot.md
    ├── design/
    │   ├── overview.md
    │   └── runtime-layout.md
    ├── plans/
    │   └── README.md
    └── specs/
        └── initial-brainstorm.md
```

## Key Principles

- Keep each version directory as a complete product snapshot.
- Keep PRD documents focused on product contract: who, why, what, and what is out of scope.
- Keep design documents focused on implementation shape: architecture, component boundaries, data flow, packaging, and testing.
- Keep future planning outside version directories until a capability enters an actual release scope.
- Product UI text, project name, executable name, installation name, and public-facing documentation should be English-first.
