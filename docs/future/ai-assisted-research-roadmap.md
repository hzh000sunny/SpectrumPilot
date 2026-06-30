# AI-Assisted Research Roadmap

## Status

Future direction. Not part of v0.1 implementation.

## Context

SpectrumPilot is intended to become a wireless research assistant for pre-research workflows. The long-term product should support more than 3GPP downloading.

Future AI-assisted capabilities include:

- Search related papers, journals, patents, and technical references based on clues or proposal content.
- Build citation-backed evidence records.
- Generate patent disclosure drafts from structured invention points and supporting evidence.
- Generate presentation decks for research review or proposal discussion.
- Summarize and compare technical proposals across meetings, work items, companies, and releases.

## Why This Is Not v0.1

These capabilities require LLM configuration, model-provider abstraction, prompt design, citation control, evidence validation, and higher-risk output review.

The first version should establish the desktop application frame and complete the 3GPP workflow first. That avoids mixing product foundation work with LLM behavior design.

## Future Architecture Direction

Future AI modules should integrate with the same desktop shell and local data model:

```text
3GPP proposal data
  ↓
Local proposal library
  ↓
Evidence and research workspace
  ↓
LLM-assisted analysis
  ↓
Patent disclosure / PPT / research brief output
```

The AI layer should not replace source records. Outputs must retain links to original proposal files, papers, patents, and extracted evidence.

## Candidate Future Modules

| Module | Purpose |
|---|---|
| Evidence Search | Search literature, patents, and web sources from a clue or proposal |
| Evidence Workspace | Organize claims, citations, snippets, and source files |
| Patent Drafting | Generate structured patent disclosure drafts |
| PPT Generator | Generate presentation outlines and slides |
| LLM Settings | Configure providers, models, keys, proxy, and local model options |

## Guardrails

- AI outputs must be source-backed.
- Generated patent or PPT content should be treated as drafts.
- The user must remain able to inspect the evidence behind a generated conclusion.
- Model configuration should be optional and should not block the non-AI 3GPP workflow.
