# 3GPP Fast Lookup, Download, Extract, and Open Design

This document defines the next 3GPP workflow upgrade for SpectrumPilot.

The goal is to replace the current two-step `search -> manual download` workflow with a single user-facing action:

```text
query -> resolve -> download -> extract -> open
```

The design covers both 3GPP specification archives and meeting contribution packages.

## 1. Product Goal

The 3GPP page should let a wireless researcher type the document clue they already have and get to the document with minimal waiting and minimal manual choices.

The default workflow should be:

1. The user enters a query such as `R2-2601401`, `R2-2601401 TSGR2_133bis`, `38.321`, or `38.321 f10`.
2. SpectrumPilot detects whether the query is a contribution or a specification.
3. SpectrumPilot resolves the exact downloadable zip URL.
4. SpectrumPilot downloads the zip package.
5. SpectrumPilot extracts it into the workspace.
6. SpectrumPilot opens the best matching document with the operating system default app.

The user should not need to select a work group for common proposal numbers. The app should infer the work group from the prefix and expose manual filters only as advanced scope controls.

## 2. Current Problems

The current contribution lookup works, but it is slower than necessary:

```text
local records miss
-> fetch workgroup directory
-> filter meeting folders
-> fetch meeting root directory
-> find Docs child
-> fetch Docs listing
-> parse every zip entry
-> exact file match
```

For a full contribution number such as `R2-2601401`, the final file name is already known:

```text
R2-2601401.zip
```

When candidate meeting directories are known, SpectrumPilot can directly probe:

```text
.../Docs/R2-2601401.zip
```

This avoids fetching large Docs listings for common exact queries.

The current workflow also stops too early. It displays a result and waits for a manual download click. The reference tool downloads and opens the extracted Word document directly, which is the more useful workflow for repeated research.

## 3. Query Types

The resolver should support two primary query families.

### 3.1 Specification Queries

Specification archives live under:

```text
https://www.3gpp.org/ftp/Specs/archive/
```

The series directory is derived from the first two digits:

```text
38.321 -> Specs/archive/38_series/38.321/
38.101-1 -> Specs/archive/38_series/38.101-1/
```

Archive file names remove the dot from the specification number:

```text
38.321 f10 -> 38321-f10.zip
38.101-1 j50 -> 38101-1-j50.zip
```

Supported inputs:

| Input | Meaning |
|---|---|
| `38.321` | Latest available version of TS 38.321 |
| `38321` | Same as `38.321` |
| `38.321 f` | Latest Release 15 package for TS 38.321 |
| `38.321 f10` | Exact `38321-f10.zip` package |
| `38321-f10` | Same exact package |
| `38.101-1 j50` | Exact multipart spec package |

Version sorting must be semantic, not simple string sorting. The version code should be parsed as:

```text
<release-letter><major><minor>
```

Known release letters:

| Letter | Release |
|---|---|
| `f` | Release 15 |
| `g` | Release 16 |
| `h` | Release 17 |
| `i` | Release 18 |
| `j` | Release 19 |

Letters before `f` and after `j` should still be accepted and sorted alphabetically for historical and future releases.

### 3.2 Contribution Queries

Contribution packages live under a work group or plenary branch, a meeting folder, and a `Docs` folder:

```text
https://www.3gpp.org/ftp/<root>/<workgroup>/<meeting>/Docs/<tdoc>.zip
```

Supported inputs:

| Input | Meaning |
|---|---|
| `R2-2601401` | Infer RAN2, search likely RAN2 meetings |
| `r2-2601401.zip` | Normalize to `R2-2601401` |
| `R2-2601401 TSGR2_133bis` | Search that meeting first |
| `R2-2601401 133bis` | Infer `TSGR2_133bis` from the R2 prefix |
| `R2-2601401 from TSGR2_120` | Start from the specified meeting and search forward |

The resolver must require exact file-name matches for automatic download:

```text
R2-2601401 -> R2-2601401.zip
```

If multiple exact candidates are found, the app must show a candidate list and avoid automatic open until the user chooses one.

## 4. Work Group Mapping

The work group should be inferred from the contribution prefix whenever possible.

| Group | Prefix | Path | Meeting Series |
|---|---|---|---|
| RAN | `RP` | `tsg_ran/TSG_RAN` | `TSGR` |
| RAN1 | `R1` | `tsg_ran/WG1_RL1` | `TSGR1` |
| RAN2 | `R2` | `tsg_ran/WG2_RL2` | `TSGR2` |
| RAN3 | `R3` | `tsg_ran/WG3_Iu` | `TSGR3` |
| RAN4 | `R4` | `tsg_ran/WG4_Radio` | `TSGR4` |
| RAN5 | `R5` | `tsg_ran/WG5_Test_ex-T1` | `TSGR5` |
| SA | `SP` | `tsg_sa/TSG_SA` | `TSGS` |
| SA1 | `S1` | `tsg_sa/WG1_Serv` | `TSGS1` |
| SA2 | `S2` | `tsg_sa/WG2_Arch` | `TSGS2` |
| SA3 | `S3` | `tsg_sa/WG3_Security` | `TSGS3` |
| SA4 | `S4` | `tsg_sa/WG4_CODEC` | `TSGS4` |
| SA5 | `S5` | `tsg_sa/WG5_TM` | `TSGS5` |
| SA6 | `S6` | `tsg_sa/WG6_MissionCritical` | `TSGS6` |
| CT | `CP` | `tsg_ct/TSG_CT` | `TSGC` |
| CT1 | `C1` | `tsg_ct/WG1_mm-cc-sm_ex-CN1` | `TSGC1` |
| CT2 | `C2` | `tsg_ct/WG2_capability_ex-T2` | `TSGC2` |
| CT3 | `C3` | `tsg_ct/WG3_interworking_ex-CN3` | `TSGC3` |
| CT4 | `C4` | `tsg_ct/WG4_protocollars_ex-CN4` | `TSGC4` |
| CT5 | `C5` | `tsg_ct/WG5_osa_ex-CN5` | `TSGC5` |
| CT6 | `C6` | `tsg_ct/WG6_Smartcard_Ex-T3` | `TSGC6` |

The existing implementation already parses many prefixes, but it must add complete path inference for plenary prefixes and CT2/CT5.

## 5. Search Strategy

The resolver should use a three-stage strategy.

### 5.1 Local First

Read local records and indexes before any network access.

If a strong local match exists:

```text
local match -> download -> extract -> open
```

No remote discovery should be required for a known URL.

### 5.2 Direct Probe

When the query is a full contribution number, generate candidate meeting URLs and probe exact zip URLs concurrently.

Example:

```text
https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip
```

The probe should prefer `HEAD` and accept a candidate only when the status is `200 OK` and the URL basename exactly matches the normalized contribution number. If a server or path behaves poorly with `HEAD`, the resolver may fall back to a ranged or normal `GET`.

Candidate meeting order:

1. Explicit meeting hint, when provided.
2. Meetings whose number is consistent with the contribution year.
3. Recent meetings at or above the default start meeting for that work group.
4. Older meetings only in deep fallback mode.

Foreground probing should be parallel and fast:

- no artificial sleep
- capped concurrency
- timeout per request
- stop remaining probes once a strong match is found
- report progress after each candidate batch

### 5.3 Deep Fallback

If direct probing misses, fallback to the current listing-based approach:

```text
fetch meeting root -> find Docs -> fetch Docs listing -> parse files -> exact match
```

This is slower, but it covers historical oddities and directories where direct probe does not behave as expected.

## 6. Download, Extract, and Open

The primary command should perform the whole workflow:

```text
resolve -> download zip -> extract -> choose document -> open
```

Workspace layout:

```text
SpectrumPilotWorkspace/
  3gpp/
    downloads/
      R2-2601401.zip
      38321-j30.zip
    tdocs/
      RAN2/
        TSGR2_133bis/
          R2-2601401/
            R2-2601401.docx
    specs/
      38.321/
        j30/
          38321-j30.docx
```

Extraction rules:

1. Download to a temporary file.
2. Move the complete zip into `downloads`.
3. Extract into the stable `tdocs` or `specs` folder.
4. Choose the best document to open.

Document choice priority:

1. Exact stem `.docx`
2. Exact stem `.doc`
3. Exact stem `.pdf`
4. Single `.docx` in the extracted folder
5. Single `.doc` in the extracted folder
6. Single `.pdf` in the extracted folder
7. If ambiguous, open the extracted folder instead of guessing.

The app should use the operating system default opener through Tauri opener support.

## 7. Progress and Cancellation

The frontend should show a modal during the workflow. It should not be a fake timer. It must reflect backend job progress.

Suggested stages:

```text
Resolving query
Checking local index
Building candidate meetings
Probing candidate URLs
Downloading package
Extracting archive
Opening document
Completed
```

The modal should include:

- current stage
- progress percent when known
- searched URL count
- current work group or meeting
- concise live log
- close button in the top-right corner

Close behavior:

- clicking the top-right close button cancels the current job
- pending network probes stop
- incomplete temporary download files are deleted
- already completed zip files are kept
- UI shows `Canceled by user`

Backend shape:

```text
start_gpp_lookup_job(request) -> jobId
cancel_gpp_lookup_job(jobId)
event: gpp-job-progress
event: gpp-job-complete
event: gpp-job-error
event: gpp-job-cancelled
```

The backend should keep a small job registry:

```text
jobId -> CancellationToken
```

Only foreground jobs need this cancellation model. Background refresh should remain conservative and separate.

## 8. 3GPP Page UI

The main page should remain task-focused.

Recommended layout:

```text
3GPP Search & Download
[Auto | Specification | Proposal]
[query input                                      ] [Find, Download & Open]

Advanced scope
  Work group: Auto / RAN / RAN1 / RAN2 / ... / CT6
  Meeting hint: optional
  Search window: Fast recent / From meeting / Deep search
  Open extracted document after download: on

Lookup Rules
  Specification examples
  Proposal examples

Results
Download Status
```

The advanced scope should be collapsed by default. The normal user should only need the query input.

The `Lookup Rules` section is allowed on the page because it teaches search syntax. It must not display internal catalog status or diagnostics. Storage diagnostics stay in Settings.

## 9. Lookup Rules Copy

The homepage rule section should use concise English copy.

Specification examples:

| Query | Meaning |
|---|---|
| `38.321` | Latest TS 38.321 package |
| `38321` | Same as `38.321` |
| `38.321 f` | Latest Release 15 package |
| `38.321 f10` | Exact v15.1.0 package |

Proposal examples:

| Query | Meaning |
|---|---|
| `R2-2601401` | Auto-detect RAN2 and find the package |
| `R2-2601401 TSGR2_133bis` | Search that meeting first |
| `R2-2601401 from TSGR2_120` | Search forward from a meeting |

Short guidance:

```text
Use Advanced scope only when you need to force a work group or search older meetings.
```

## 10. Dependencies

Rust dependencies:

| Dependency | Purpose |
|---|---|
| `zip` | Extract downloaded 3GPP packages |
| `tokio-util` | `CancellationToken` for foreground jobs |
| `futures-util` | Stream downloads and report byte progress |

Frontend dependencies should not be needed. Ant Design already provides modal, progress, steps, buttons, segmented controls, checkboxes, and form controls.

## 11. Testing Strategy

Core Rust tests:

- parse specification queries
- parse contribution queries with optional meeting hints
- infer all listed work group paths
- sort specification version codes
- build specification archive URLs
- build contribution direct probe URLs
- choose extracted document by priority
- cancel a job and clean temporary files

Desktop command tests:

- local hit avoids remote discovery
- direct probe finds a known candidate
- deep fallback is used when direct probe misses
- download writes to the expected workspace path
- extraction writes to `tdocs` or `specs`
- open target selection is deterministic

Frontend tests:

- page shows lookup rules
- advanced scope is collapsed by default
- starting a job opens the progress modal
- progress events update the modal text and progress
- close button calls cancel
- completed job shows opened/saved path
- multiple candidates require user selection

Live smoke tests:

- known proposal: `R2-2601401`
- known spec exact version: `38.321 f10`
- known spec latest query: `38.321`

Live tests should stay ignored by default.

## 12. Implementation Phases

Phase 1:

- Add query parsers and URL builders.
- Complete contribution prefix path mapping.
- Add specification archive lookup.
- Add direct probe for exact contribution numbers.

Phase 2:

- Add foreground job model with progress events and cancellation.
- Add progress modal UI.
- Add `Find, Download & Open` action.

Phase 3:

- Add zip extraction and document selection.
- Open extracted document or folder.
- Add lookup rules section to the 3GPP page.

Phase 4:

- Update docs and live smoke tests.
- Keep old manual result download as a fallback path when there are multiple candidates.

## 13. Non-Goals

This work does not include:

- full historical backfill
- scheduled incremental refresh
- AI analysis of downloaded documents
- batch download queue
- patent disclosure generation
- PPT generation

Those remain future 3GPP and research-assistant capabilities.
