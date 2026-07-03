# 3GPP Search & Download

## Current User Workflow

SpectrumPilot now exposes a single-action 3GPP workflow:

```text
query -> resolve URL -> download ZIP -> extract -> open document or folder
```

The primary page is `3GPP Ftp`. Users enter a proposal number or specification number, then click `Find, Download & Open`.

Supported examples:

```text
R2-2601401
R2-2601401 TSGR2_133bis
R2-2601401 from TSGR2_120
38.321
38321
38.321 f
38.321 f10
38.101-1 j50
```

The page is intentionally not a catalog diagnostics surface. Manifest, record, index, runtime, and path details belong in Settings.

## Bundled Seed Catalog

The product should not ask users to initialize the 3GPP catalog after installation.

SpectrumPilot ships with a bundled seed catalog. On first startup, if the local 3GPP catalog directory is empty, the app silently copies the bundled seed into the internal application storage metadata directory. Users can search immediately without clicking an initialization button.

The current repository seed is intentionally small and comes from catalog files already fetched during development. Near the end of a release cycle, the project should generate a larger stage-specific seed from the locally accumulated catalog, then perform one deliberate full refresh before freezing that release seed. This is not a build-time or every-CI-run crawl.

## Lookup Modes

The page has three modes:

| Mode | Behavior |
|---|---|
| `Auto` | Detects whether the query is a proposal or a specification. |
| `Specification` | Rejects proposal-looking input and resolves only the 3GPP spec archive. |
| `Proposal` | Rejects spec-looking input and resolves only meeting contribution packages. |

Advanced scope is collapsed by default. It can force a work group, provide a meeting hint, choose a search window, and control whether the extracted target opens automatically.

## Implemented Behavior

| Area | Behavior |
|---|---|
| Query normalization | Accepts lowercase proposal input and optional `.zip` suffix, then normalizes to canonical form such as `R2-2601401`. |
| Specification parsing | Accepts dotted and compact spec numbers, release-letter filters, and exact versions such as `38.321 f10` or `38321-f10`. |
| Source inference | Maps proposal prefixes including RAN, SA, and CT plenary/workgroup prefixes to 3GPP FTP branches. |
| Local lookup | Reads cached `FileRecord` JSON records and resolves proposals through the local `TDocLookupIndex` before network access. |
| Direct proposal probe | Builds exact candidate URLs such as `.../Docs/R2-2601401.zip`, probes with `HEAD`, requires `200 OK`, and requires the URL basename to exactly match the requested ZIP. |
| Online fallback | If direct probing misses, falls back to the existing listing-based search over meeting roots and `Docs` folders. |
| Cache update | Stores discovered listing records from fallback search and writes direct-probe hits back into the local file-record cache. |
| Specification archive lookup | Uses `Specs/archive/<series>_series/<spec>/`, directly probes exact versions, or reads the archive listing to select the latest matching version. |
| Progress UI | Shows a modal with real stages: resolve, download, extract, open. The modal close button cancels the running job. |
| Cancellation | Tauri jobs use a cancellation token and check it before network batches, before download, before extraction, and before opening. |
| Download and extraction | Downloads the resolved ZIP, extracts with safe enclosed ZIP paths only, and rejects path traversal entries by skipping unsafe names. |
| Repeat lookup cache | If the exact extracted document already exists, SpectrumPilot opens it without re-downloading or re-extracting. If only the ZIP exists, it extracts the local ZIP without re-downloading. |
| Lookup result status | The completed lookup payload includes `cacheStatus` values `cached_document`, `cached_zip`, or `downloaded`; the UI shows this as `Opened cached document`, `Extracted local ZIP`, or `Downloaded from 3GPP FTP`. |
| Open target | Opens the exact `.docx`, `.doc`, or `.pdf` matching the package stem first; if no unambiguous document exists, opens the extraction folder. |
| Seed metadata | The bundled seed includes `seed.json` with seed version, generation timestamp, and seed scope; Settings displays this metadata. |
| Background incremental refresh | On desktop startup, SpectrumPilot waits 15 seconds, then refreshes the six supported `tsg_*` root manifests sequentially with a 2-second delay between requests. It repeats every 60 minutes and does not expose a manual Sync button on the main UI. If a root manifest fingerprint changes, it refreshes changed workgroup manifests, then only the most recent 8 meeting directories per changed workgroup, and writes updated Docs records back to meeting shards and lookup indexes. |
| Background refresh status | The background refresh loop persists `catalog/background-refresh.json` with state, last start time, last successful completion time, last error, and the most recent refreshed manifest count. Settings displays these values for diagnostics. |

## Runtime Workspace Layout

The new workflow writes into stable per-document folders under the user workspace:

```text
SpectrumPilotWorkspace/
  3gpp/
    tdocs/
      RAN2/
        TSGR2_133bis/
          R2-2601401/
            R2-2601401.zip
            R2-2601401.docx
    specs/
      38.321/
        f10/
          38321-f10.zip
          38321-f10.docx
```

The older compatibility command `download_gpp_tdoc` still stores manually selected ZIP files under:

```text
SpectrumPilotWorkspace/3gpp/downloads/
```

## Internal 3GPP Metadata Layout

3GPP JSON catalog data is internal application metadata, not user workspace content:

```text
ApplicationStorage/
  metadata/
    3gpp/
      catalog/
        seed.json
        background-refresh.json
        manifests/
        records/
        indexes/
  logs/
    3gpp-refresh.log
```

The app migrates older catalog, refresh settings, and refresh log files from the previous Tauri
default data/cache/log locations into the unified `SpectrumPilot` application storage root.

## Verified Live Examples

The current live smoke tests verify:

| Test | Verification |
|---|---|
| `live_lookup_download_finds_known_ran2_tdoc` | Downloads and extracts `R2-2601401.zip` from `TSGR2_133bis`. |
| `live_spec_lookup_finds_exact_38321_f10` | Confirms `38321-f10.zip` exists in the `38.321` archive. |
| `live_spec_lookup_finds_latest_38321` | Reads the live `38.321` archive listing and selects the latest visible package. |

These tests are ignored during normal test runs and must be run explicitly because they access the public 3GPP site.

## Current Boundaries

Implemented now:

- Proposal and specification query parsing.
- Local proposal cache-first lookup.
- Fast direct proposal URL probing.
- Targeted listing fallback for proposal misses.
- Specification archive exact and latest-version lookup.
- One-click download, safe extraction, and open.
- Progress modal with cancellation.
- Direct-probe hit caching for faster repeat lookups.
- Bundled seed catalog installation for empty internal metadata catalogs.
- 3GPP storage, seed metadata, background refresh policy, and catalog status in Settings.
- Conservative scheduled refresh of the supported `tsg_*` roots with parent/child manifest diffing and an 8-meeting per changed workgroup window.
- Persisted background refresh state and last-error display in Settings.

Still planned:

- Candidate-selection UI when multiple exact matches are intentionally surfaced.
- User-configurable workspace directory.
- Full historical backfill.
- Release-final stage seed generation from the accumulated local catalog after a deliberate full refresh/backfill pass.
- Batch download queue.
- Proposal library indexing beyond TDoc number lookup.
- User controls for pausing or forcing background refresh.
