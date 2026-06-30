# SpectrumPilot 3GPP Local Index Design

This document defines the local indexing and download strategy for the initial `3gpp` module in SpectrumPilot.

It focuses on the first useful workflow: resolve a proposal, meeting item, or related document to a stable URL quickly, cache the result locally, and download it with minimal repeated access to the 3GPP site.

## 1. Purpose

The current problem is not just file download. The real problem is discovery.

Users often start with incomplete clues:

- a proposal number
- a meeting code
- a work group
- a title fragment
- an author or company name

The module should turn those clues into a local match whenever possible. Remote lookup should be the fallback, not the default path.

## 2. Goals

The module should:

- resolve common 3GPP proposal and meeting queries from local data first
- keep a structured local index of known 3GPP content
- refresh the index incrementally instead of rescanning the full site
- reduce the chance of triggering anti-bot or rate-limit behavior
- support direct download once the target URL is known
- keep the implementation compatible with future AI-assisted research features

## 3. Non-Goals

This design does not try to:

- mirror the full 3GPP site
- build LLM-based evidence analysis
- generate patent disclosures
- generate PPT decks
- solve every possible 3GPP page shape with one universal parser

The first release should be practical, not exhaustive.

## 4. Design Principles

1. Prefer local index hits over live crawling.
2. Keep user-visible files separate from application state.
3. Treat different 3GPP root structures as different source families.
4. Store canonical records in a normalized format.
5. Make change detection cheap enough to run on a timer.
6. Limit remote requests aggressively.
7. Allow the index to grow by adding adapters, not by rewriting the whole crawler.

## 5. Recommended Architecture

The module should be split into four layers:

```text
UI
  -> query / browse / download actions

Resolver
  -> normalize the user's clue
  -> search local indexes
  -> decide whether remote refresh is needed

Source Adapters
  -> parse specific 3GPP root families
  -> extract directory entries and file metadata

Local Storage
  -> canonical records
  -> lookup indexes
  -> sync state
  -> download history
```

The UI must not know how 3GPP URLs are derived. It only submits a request and receives either:

- one resolved target
- a ranked candidate list
- or a refresh-needed result

## 6. Source Families

3GPP is not a single uniform tree. The module should register multiple source families and parse each one with the adapter that fits it.

Examples of families the module is expected to encounter:

- document archive trees such as `Docs`
- specification trees such as `Specs`
- email discussion trees such as `Email_Discussions`
- meeting index trees such as `Meetings_3GPP_SYNC`
- TDoc list pages such as `TdocListDefault`
- organizational branches such as `tsg_ran`, `tsg_sa`, and `tsg_ct`

The exact set of roots can expand over time. The important rule is that each family gets a parser that matches its structure.

## 7. Canonical Record Model

The local index should not store raw HTML as its primary data model.

Instead, the crawler should normalize each discovered item into a canonical record.

Minimum fields:

```json
{
  "id": "string",
  "kind": "file",
  "sourceType": "tdoc-archive",
  "sourceUrl": "https://example.org/file.zip",
  "parentUrl": "https://example.org/Docs/",
  "displayName": "R2-167140.zip",
  "canonicalKey": "R2-167140",
  "workGroup": "RAN WG2",
  "meetingCode": "TSGR2_95bis",
  "title": "string",
  "size": 55823,
  "lastModified": "2016-10-01T06:38:01Z",
  "fingerprint": "sha256:..."
}
```

The schema can grow, but the first version should keep the record small and query-friendly.

## 8. Local Storage Layout

The runtime layout should stay consistent with the existing workspace rules:

- application state belongs under `%APPDATA%\\SpectrumPilot\\`
- user-managed content belongs under the configurable workspace root
- the workspace feature directory is `3gpp`

Recommended state layout:

```text
%APPDATA%\SpectrumPilot\
  config\
  db\
  logs\
  cache\
    3gpp\
      roots.json
      manifests\
      records\
      lookup\
      sync-state.json
```

Recommended workspace layout:

```text
<workspace>\3gpp\
  meetings\
  tdocs\
  downloads\
  library\
  watchlists\
```

The local index lives in application state because it is internal product data, not user content.

## 9. Index Sharding Strategy

Do not store the entire world in one JSON file.

Use small files grouped by responsibility:

- `roots.json` for known entry points and adapter types
- `manifests/` for directory snapshots
- `records/` for normalized item records
- `lookup/` for reverse lookup tables
- `sync-state.json` for refresh progress and backoff state

This makes incremental updates cheaper and keeps the data set manageable when the catalog grows.

## 10. Refresh Strategy

Refresh should be incremental and selective.

### 10.1 Full-site rescans are forbidden as a routine strategy

The module should never scan the entire site on a timer just because the timer fired.

### 10.2 Change detection order

For each known directory or listing page, check in this order:

1. `ETag`
2. `Last-Modified`
3. child-list fingerprint
4. child count

If the server supports conditional requests, use them first:

- `If-None-Match`
- `If-Modified-Since`

If the server does not support useful validators, compare the parsed child list against the stored manifest.

### 10.3 Directory fingerprinting

Directory fingerprints should be based on the normalized child set, not the page HTML alone.

Fingerprint inputs should include:

- child name
- child type
- child URL
- child size when available
- child last-modified time when available

The order of items should not affect the fingerprint.

### 10.4 Refresh tiers

Use different refresh cadences for different nodes:

- hot nodes: about once per hour
- warm nodes: every few hours
- cold historical nodes: on demand or on a much slower schedule

The refresh scheduler should promote frequently used branches and demote rarely used ones.

## 11. Search and Resolution Flow

The resolver should follow this order:

1. normalize the user input
2. detect known patterns such as proposal numbers or meeting codes
3. query the local reverse index
4. return a resolved URL if there is a strong match
5. return a short candidate list if there are multiple plausible matches
6. trigger a targeted remote refresh only if the local index does not contain enough information

Examples of normalized clues:

- `R2-167140`
- `167140`
- `TSGR2_95bis`
- `RAN WG2`
- a title fragment
- a company name

The resolver should use clue type to choose the most likely root family before crawling remotely.

## 12. Targeted Remote Crawl

When the local index misses, the crawler should not restart from the top of the site.

Instead, it should:

- identify the likely source family
- refresh only the related branch
- stop as soon as enough information has been gathered to answer the query

This is the main optimization that keeps the module fast and safe.

## 13. Download Flow

Once the resolver produces a file URL:

1. validate the URL lightly
2. inspect `Content-Length` and `Last-Modified` when available
3. download to a temporary file
4. move the completed file into the workspace under `3gpp/downloads/`
5. update the canonical record and download history

The download flow should never require a second discovery pass if the URL is already known.

## 14. Rate Limiting and Anti-Bot Protection

The crawler should assume the site may throttle repeated requests.

Required safeguards:

- keep concurrency very low per host
- add small random delays between requests
- stop aggressive crawling after repeated `403` or `429` responses
- apply exponential backoff after failures
- prefer shallow refreshes over deep recursive scans

The goal is to be a polite consumer of a public site, not a mirror service.

## 15. Failure Handling

The module should handle these failures explicitly:

- directory page format changed
- file no longer exists
- a file URL resolves but returns an unexpected status
- remote validation headers are missing
- partial refresh was interrupted

In each case, keep the last known good data and mark the affected branch as stale rather than deleting it immediately.

## 16. Testing Strategy

The first test set should cover:

- clue normalization
- URL resolution from local records
- manifest diffing
- fingerprint stability
- refresh backoff behavior
- download path handling
- adapter parsing for known 3GPP root shapes

Integration tests should use recorded fixtures or mocked HTTP responses so that the crawler logic can be validated without depending on live site behavior.

## 17. Open Questions

These items can be finalized during implementation:

- whether the first index pass should use JSON only or JSON plus SQLite search tables
- how many source families should be supported in the first milestone
- whether a warm-start seed list should be hardcoded or generated from a bootstrap crawl
- whether a nightly refresh is needed at v0.1 or only an hourly hot refresh plus on-demand refreshes

## 18. Implementation Boundary

This design intentionally stays one layer above code.

It defines:

- the storage model
- the refresh rules
- the resolution flow
- the anti-bot constraints

It does not yet define:

- concrete Rust module names
- Tauri command names
- UI page wiring
- packaging details

Those belong in the implementation plan.
