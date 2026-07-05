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

## Compact Seed Catalog

The product should not ask users to initialize the 3GPP catalog after installation.

SpectrumPilot supports a compact read-only seed catalog. On first startup, the app silently copies a small bootstrap seed into the internal application storage metadata directory. The bootstrap seed does not contain the full TDoc index. It exists so Settings can show seed/download state while the full compact catalog is installed asynchronously.

The 2024-2026 staged baseline was converted from the already fetched 350 MB sharded catalog without crawling 3GPP again. The compact release seed stores the same recent proposal coverage as 77 JSON files and about 18 MB:

```text
recordCount: 215,379
indexItemCount: 215,338
meetingCount: 232
recordShardCount: 16
indexShardCount: 59
```

The compact format removes repeated full URLs and repeated JSON field names from per-file entries. Workgroup record shards share the base URL and meeting `Docs` path, while prefix/year index shards store only pointers into those records.

The full compact catalog is committed under `data/3gpp/catalog_seed/` in the repository and includes `download-manifest.json` with file sizes and SHA-256 hashes. The installer resources under `apps/desktop/src-tauri/resources/3gpp/catalog_seed/` intentionally contain only bootstrap metadata and the default download manifest URL, so the desktop installer does not need to embed the full catalog. At runtime, SpectrumPilot downloads/copies every listed JSON file into a staging directory, validates each hash, and atomically activates the compact catalog.

If the download source is unavailable or fails, lookup still falls back to direct online probing and targeted listing search.

Release-stage compact seed generation is a manual, offline tooling step. It reads the already accumulated local catalog, including both legacy `records/*.json` file records and `records/tdoc/**/*.json` meeting shards, then writes the compact release artifact and download manifest:

```bash
python3 scripts/3gpp/build_compact_seed.py \
  --source /path/to/SpectrumPilot/metadata/3gpp/catalog \
  --target data/3gpp/catalog_seed \
  --seed-version compact-stage-seed-YYYY-MM-DD \
  --generated-at YYYY-MM-DDT00:00:00Z \
  --scope "2024-2026 staged 3GPP catalog" \
  --force
```

This command is not intended to run during every build or every CI job. Near a release boundary, the project should first refresh or backfill the local catalog deliberately, then regenerate and review `data/3gpp/catalog_seed/`. The Tauri resource seed remains bootstrap-only unless the packaging strategy is explicitly changed.

Release packaging, pinned catalog URL, Windows installer, and updater details are tracked in [release-packaging.md](./release-packaging.md).

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
| Local lookup | Checks runtime overlay shards first, then the compact read-only seed, then legacy `FileRecord` records before network access. |
| Direct proposal probe | Builds exact candidate URLs such as `.../Docs/R2-2601401.zip`, probes with `HEAD`, requires `200 OK`, and requires the URL basename to exactly match the requested ZIP. |
| Online fallback | If direct probing misses, falls back to the existing listing-based search over meeting roots and `Docs` folders. |
| Cache update | Stores discovered listing records from fallback search and writes direct-probe hits back into the local file-record cache. |
| Specification archive lookup | Uses `Specs/archive/<series>_series/<spec>/`, directly probes exact versions, or reads the archive listing to select the latest matching version. |
| Progress UI | Shows a modal with real stages: resolve, download, extract, open. The modal close button cancels the running job. |
| Cancellation | Tauri jobs use a cancellation token and check it before network batches, before download, before extraction, and before opening. |
| Download and extraction | Downloads the resolved ZIP, extracts with safe enclosed ZIP paths only, and rejects path traversal entries by skipping unsafe names. |
| Repeat lookup cache | If the exact extracted document already exists, SpectrumPilot opens it without re-downloading or re-extracting. If only the ZIP exists, it extracts the local ZIP without re-downloading. |
| Candidate selection | When multiple exact proposal candidates are intentionally surfaced by online probing or listing fallback, the job emits a candidate-selection event and waits for the user to choose the ZIP before downloading. |
| Batch lookup queue | The main 3GPP Ftp page accepts one query per line, runs lookups sequentially, shows pending/running/done/error/cancelled rows, and lets cancelling the active row continue with the remaining pending rows. |
| Lookup history | Completed lookups append a JSONL history record in internal catalog metadata. Proposal Library reads the latest local history records and displays query, cache status, source URL, ZIP path, opened path, and formatted completion time. |
| Lookup result status | The completed lookup payload includes `cacheStatus` values `cached_document`, `cached_zip`, or `downloaded`; the UI shows this as `Opened cached document`, `Extracted local ZIP`, or `Downloaded from 3GPP FTP`. |
| Open target | Opens the exact `.docx`, `.doc`, or `.pdf` matching the package stem first; if no unambiguous document exists, opens the extraction folder. |
| Seed metadata | The bundled seed includes `seed.json` with seed version, generation timestamp, and seed scope; Settings displays this metadata. |
| Catalog install state | Settings displays compact seed install/download status from `catalog-download.json` or infers `Ready` only when local compact data is present. |
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
        catalog-download.json
        background-refresh.json
        staging/
        compact/
          summary.json
          records/
            RAN2.json
            SA2.json
          index/
            R2_26.json
            S2_26.json
        manifests/      # runtime overlay and scheduled refresh
        records/        # runtime overlay discoveries
        indexes/        # runtime overlay lookup shards
        history/
          lookups.jsonl
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
- Candidate-selection UI for multiple exact proposal matches.
- Batch lookup queue for sequential multi-query runs.
- Proposal Library history from completed 3GPP lookups.
- Direct-probe hit caching for faster repeat lookups.
- Bundled seed catalog installation for empty internal metadata catalogs.
- Compact 2024-2026 seed generated from the existing staged baseline without a new crawl.
- Compact seed builder supports mixed local catalog sources: legacy single-file records and sharded meeting records.
- Compact seed lookup before online fallback.
- Asynchronous compact seed installation from a manifest URL with size and SHA-256 validation.
- 3GPP storage, seed metadata, background refresh policy, and catalog status in Settings.
- Conservative scheduled refresh of the supported `tsg_*` roots with parent/child manifest diffing and an 8-meeting per changed workgroup window.
- Persisted background refresh state and last-error display in Settings.

Still planned:

- Configure the final GitHub Release catalog manifest URL for production releases.
- Full historical backfill beyond the current recent-years scope.
- Release-final stage seed regeneration after a deliberate full refresh/backfill pass.
- Proposal Library indexing and filtering beyond latest lookup history.
- User controls for pausing or forcing background refresh.
