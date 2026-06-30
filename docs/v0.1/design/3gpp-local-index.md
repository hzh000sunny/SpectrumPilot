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
- keep background refresh polite and low-volume
- make user-triggered online search as fast as practical when the local index misses
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
6. Separate foreground online search behavior from background sync behavior.
7. Make foreground search fast, parallel, and user-visible.
8. Make background sync conservative, incremental, and low-volume.
9. Allow the index to grow by adding adapters, not by rewriting the whole crawler.

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

For v0.1, the primary source scope is the six root directories under `https://www.3gpp.org/ftp/` whose names begin with `tsg_`:

- `tsg_cn`
- `tsg_ct`
- `tsg_geran`
- `tsg_ran`
- `tsg_sa`
- `tsg_t`

These roots are enough for the first 3GPP proposal and meeting workflows. Other public roots can be added later if a concrete workflow requires them.

Examples of later source families the module may encounter:

- document archive trees such as `Docs`
- specification trees such as `Specs`
- email discussion trees such as `Email_Discussions`
- meeting index trees such as `Meetings_3GPP_SYNC`
- TDoc list pages such as `TdocListDefault`

The exact set of roots can expand over time. The important rule is that each family gets a parser that matches its structure.

## 7. Coverage Priority

The first index build should not assume that a full historical crawl is cheap.

Coverage should be prioritized in this order:

1. active `tsg_` branches with recent child updates
2. recent meeting folders from the last few years
3. older but frequently queried branches
4. full historical backfill, only when it can run safely in the background

The product should expose index coverage clearly. If only recent years have been indexed, the UI should say that directly and allow the user to trigger online search for older items.

## 8. Canonical Record Model

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

## 9. Local Storage Layout

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

## 10. Index Sharding Strategy

Do not store the entire world in one JSON file.

Use small files grouped by responsibility:

- `roots.json` for known entry points and adapter types
- `manifests/` for directory snapshots
- `records/` for normalized item records
- `lookup/` for reverse lookup tables
- `sync-state.json` for refresh progress and backoff state

This makes incremental updates cheaper and keeps the data set manageable when the catalog grows.

## 11. Background Refresh Strategy

Background refresh should be incremental and selective.

### 11.1 Full-site rescans are forbidden as a routine strategy

The module should never scan the entire site on a timer just because the timer fired.

### 11.2 Parent-directory gating

The first refresh check should happen at the parent directory level.

3GPP directory listing pages include child rows with child URLs, names, and update timestamps. The background refresher should parse those child rows first and compare them with the stored parent manifest.

Only children whose row data changed should be scheduled for deeper refresh.

This avoids entering every child directory during each incremental update.

### 11.3 Change detection order

For each known directory or listing page, check in this order:

1. `ETag`
2. `Last-Modified`
3. parsed child-row update timestamps
4. child-list fingerprint
5. child count

If the server supports conditional requests, use them first:

- `If-None-Match`
- `If-Modified-Since`

If the server does not support useful validators, compare the parsed child list against the stored manifest.

Observed 3GPP directory pages may not provide useful `ETag` or `Last-Modified` response headers for directory listings. The parser must therefore support child-row timestamp and fingerprint based detection as the normal path, not only as a fallback.

### 11.4 Directory fingerprinting

Directory fingerprints should be based on the normalized child set, not the page HTML alone.

Fingerprint inputs should include:

- child name
- child type
- child URL
- child size when available
- child last-modified time when available

The order of items should not affect the fingerprint.

### 11.5 Refresh tiers

Use different refresh cadences for different nodes:

- hot nodes: about once per hour
- warm nodes: every few hours
- cold historical nodes: on demand or on a much slower schedule

The refresh scheduler should promote frequently used branches and demote rarely used ones.

## 12. Search and Resolution Flow

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

## 13. Foreground Online Search

Foreground online search is different from background refresh.

When the user is actively waiting for a search result and the local index misses, the system should optimize for response speed:

- do not add artificial sleep between requests
- search likely branches in parallel
- stop remaining searches as soon as a strong match is found
- show progress and searched branches in the UI
- cache every discovered record so future searches become local hits

Foreground search should still use sensible engineering limits:

- cap total concurrency to avoid making the app unstable
- use request timeouts
- cancel work when the user cancels the search
- stop retry loops after a clear failure

The important rule is that anti-bot conservatism belongs to background sync. A user-triggered foreground search should be as fast as the network and the target site allow.

## 14. Targeted Remote Crawl

When the local index misses, the crawler should not restart from the top of the site.

Instead, it should:

- identify the likely source family
- search only the related branches
- run likely branches in parallel during foreground search
- stop as soon as enough information has been gathered to answer the query

This is the main optimization that keeps normal searches fast while still avoiding routine full-site scans.

## 15. Download Flow

Once the resolver produces a file URL:

1. validate the URL lightly
2. inspect `Content-Length` and `Last-Modified` when available
3. download to a temporary file
4. move the completed file into the workspace under `3gpp/downloads/`
5. update the canonical record and download history

The download flow should never require a second discovery pass if the URL is already known.

## 16. Background Rate Limiting and Anti-Bot Protection

The background refresher should assume the site may throttle repeated requests.

Required safeguards for background work:

- keep concurrency very low per host
- add small random delays between requests
- stop aggressive crawling after repeated `403` or `429` responses
- apply exponential backoff after failures
- prefer shallow refreshes over deep recursive scans

These safeguards apply to scheduled refresh and historical backfill. They do not impose artificial sleep on a foreground user search.

## 17. Product Behavior and Usage

The existing screenshot from another tool is only a reference for the old workflow. SpectrumPilot should not copy that tool's interface or exact behavior.

The intended workflow is:

1. user enters a proposal number, meeting code, or keyword
2. SpectrumPilot searches the local index first
3. if local data is insufficient, SpectrumPilot runs a fast online search
4. matching candidates are displayed with source path, meeting, work group, and URL
5. user downloads one or more selected items
6. discovered metadata is kept for future searches

The UI should also provide:

- index coverage status
- last refresh time
- manual refresh controls
- online search progress
- downloaded file location
- errors that explain whether the issue is local indexing, remote lookup, or download

## 18. Failure Handling

The module should handle these failures explicitly:

- directory page format changed
- file no longer exists
- a file URL resolves but returns an unexpected status
- remote validation headers are missing
- partial refresh was interrupted

In each case, keep the last known good data and mark the affected branch as stale rather than deleting it immediately.

## 19. Testing Strategy

The first test set should cover:

- clue normalization
- URL resolution from local records
- manifest diffing
- fingerprint stability
- refresh backoff behavior
- foreground parallel search cancellation
- foreground search result ranking
- download path handling
- adapter parsing for known 3GPP root shapes

Integration tests should use recorded fixtures or mocked HTTP responses so that the crawler logic can be validated without depending on live site behavior.

## 20. Open Questions

These items can be finalized during implementation:

- whether the first index pass should use JSON only or JSON plus SQLite search tables
- whether a warm-start seed list should be hardcoded or generated from a bootstrap crawl
- whether a nightly refresh is needed at v0.1 or only an hourly hot refresh plus on-demand refreshes
- how far back the default recent-year bootstrap should go
- whether a full historical backfill should be optional, hidden behind an advanced setting, or deferred entirely

## 21. Implementation Boundary

This design intentionally stays one layer above code.

It defines:

- the storage model
- the background refresh rules
- the foreground online search rules
- the resolution flow
- the background anti-bot constraints
- the user-facing workflow

It does not yet define:

- concrete Rust module names
- Tauri command names
- UI page wiring
- packaging details

Those belong in the implementation plan.
