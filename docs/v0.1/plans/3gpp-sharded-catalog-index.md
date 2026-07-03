# 3GPP Sharded Catalog Index Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace one-file-per-TDoc cache records with meeting-level record shards and prefix/year lookup index shards, then package the current staged catalog as installable seed data.

**Architecture:** Keep directory manifests as one JSON per remote directory because they support incremental refresh diffing. Store TDoc file records by meeting shard under `records/tdoc/<workgroup>/<meeting>.json`, and store hot lookup data under `indexes/tdoc/<prefix>/<yy>.json` so normal searches read one index shard instead of scanning thousands of record files. Keep legacy per-file records readable during migration, but write new data to the sharded layout.

**Tech Stack:** Rust 2021, `serde`, `serde_json`, existing `spectrum-3gpp-core`, Tauri 2, React/TypeScript, Vitest.

---

## Files

- Modify: `crates/3gpp-core/src/model.rs`
  - Add `TDocMeetingRecordShard`, `TDocIndexEntry`, and `TDocIndexShard`.
- Modify: `crates/3gpp-core/src/catalog.rs`
  - Add shard path helpers, shard read/write APIs, index read/write APIs, and summary counting for indexed TDocs.
  - Keep legacy `read_file_records` and `write_file_records` temporarily for compatibility and migration.
- Modify: `crates/3gpp-core/src/index.rs`
  - Add helpers to build `TDocIndexShard` from records and resolve a single TDoc from a shard.
- Create: `crates/3gpp-core/tests/sharded_catalog_tests.rs`
  - Cover meeting shard writing, index shard writing, and lookup without scanning legacy records.
- Modify: `apps/desktop/src-tauri/src/gpp/workflow.rs`
  - Query `indexes/tdoc/<prefix>/<yy>.json` before falling back to legacy scan or online search.
  - Write meeting shards and index shards when online search discovers Docs records.
- Modify: `apps/desktop/src-tauri/src/lib.rs`
  - Install bundled seed manifests, record shards, and index shards independently.
  - Use sharded writes in `search_online_tdoc`.
- Create: `apps/desktop/src-tauri/src/seed.rs`
  - Hold seed installation helpers using runtime directory copy from source-controlled resource files.
- Create: `apps/desktop/src-tauri/tests` only if command-level integration becomes necessary; otherwise keep Rust unit tests in `lib.rs`.
- Create: `scripts/3gpp/migrate_legacy_records_to_shards.py`
  - Development-only migration script that reads the current local legacy `records/*.json` and writes staged shard/index seed files.
- Modify: `apps/desktop/src/pages/GppPage.tsx`
  - Rename `cached files` to `indexed TDocs` once summary reports index-backed count.
- Modify: `apps/desktop/src/pages/SettingsPage.tsx`
  - Show record shard and index shard counts instead of implying seed status is only manifests.
- Modify tests:
  - `apps/desktop/src/pages/GppPage.test.tsx`
  - `apps/desktop/src/pages/SettingsPage.test.tsx`

---

### Task 1: Core Shard And Index Types

**Files:**
- Modify: `crates/3gpp-core/src/model.rs`
- Test: `crates/3gpp-core/tests/sharded_catalog_tests.rs`

- [ ] **Step 1: Write failing tests for serializable shard models**

Create `crates/3gpp-core/tests/sharded_catalog_tests.rs` with:

```rust
use spectrum_3gpp_core::model::{
    FileRecord, TDocIndexEntry, TDocIndexShard, TDocMeetingRecordShard,
};

mod support {
    include!("./support/file_records.rs");
}

#[test]
fn tdoc_meeting_shard_serializes_records_for_one_meeting() {
    let record = support::ran2_record("R2-2601401", "TSGR2_133bis");
    let shard = TDocMeetingRecordShard::from_records(
        "RAN2",
        "TSGR2_133bis",
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/",
        "2026-07-02T08:00:00Z",
        vec![record.clone()],
    );

    let body = serde_json::to_string(&shard).expect("serialize shard");
    assert!(body.contains("\"recordType\":\"tdoc-meeting-records\""));
    assert_eq!(shard.work_group_code, "RAN2");
    assert_eq!(shard.meeting_slug, "TSGR2_133bis");
    assert_eq!(shard.files, vec![record]);
}

#[test]
fn tdoc_index_shard_serializes_lookup_entries_by_key() {
    let entry = TDocIndexEntry {
        tdoc: "R2-2601401".to_string(),
        file_name: "R2-2601401.zip".to_string(),
        url: "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip".to_string(),
        work_group_code: "RAN2".to_string(),
        meeting_slug: "TSGR2_133bis".to_string(),
        record_shard: "records/tdoc/RAN2/TSGR2_133bis.json".to_string(),
        remote_modified_raw: None,
        size_bytes: None,
    };
    let shard = TDocIndexShard::new("R2", 2026, vec![entry.clone()]);

    assert_eq!(shard.record_type, "tdoc-lookup-index");
    assert_eq!(shard.prefix, "R2");
    assert_eq!(shard.year, 2026);
    assert_eq!(shard.items.get("R2-2601401"), Some(&entry));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core --test sharded_catalog_tests
```

Expected: fail because the shard types and support fixture do not exist.

- [ ] **Step 3: Add shared test fixture helper**

Create `crates/3gpp-core/tests/support/file_records.rs`:

```rust
use spectrum_3gpp_core::model::{
    DirectoryRole, FileClassification, FileRecord, MeetingRecord, TDocKey,
};

pub fn ran2_record(tdoc: &str, meeting_slug: &str) -> FileRecord {
    let url = format!(
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/{meeting_slug}/Docs/{tdoc}.zip"
    );
    FileRecord {
        schema_version: 1,
        record_type: "tdoc-file".to_string(),
        id: FileRecord::stable_id(&url),
        canonical_url: url.clone(),
        parent_directory_url: format!(
            "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/{meeting_slug}/Docs/"
        ),
        root: "tsg_ran".to_string(),
        work_group_path: "WG2_RL2".to_string(),
        work_group_code: Some("RAN2".to_string()),
        meeting_id: Some(MeetingRecord::stable_id("tsg_ran", "WG2_RL2", meeting_slug)),
        meeting_slug: Some(meeting_slug.to_string()),
        container_role: DirectoryRole::Docs,
        file_name: format!("{tdoc}.zip"),
        extension: Some("zip".to_string()),
        remote_modified_raw: Some("2026/05/22 10:14".to_string()),
        size_raw: Some("10 KB".to_string()),
        size_bytes: Some(10_240),
        tdoc: Some(TDocKey {
            key: tdoc.to_string(),
            prefix: "R2".to_string(),
            number_text: tdoc.trim_start_matches("R2-").to_string(),
            year_hint: Some(2026),
        }),
        classification: FileClassification {
            is_primary_tdoc: true,
            is_zip: true,
            is_ignored_artifact: false,
        },
    }
}
```

- [ ] **Step 4: Implement model types**

Add to `crates/3gpp-core/src/model.rs`:

```rust
use std::collections::BTreeMap;
```

Then add:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TDocMeetingRecordShard {
    pub schema_version: u32,
    pub record_type: String,
    pub work_group_code: String,
    pub meeting_slug: String,
    pub docs_url: String,
    pub checked_at: String,
    pub files: Vec<FileRecord>,
}

impl TDocMeetingRecordShard {
    pub fn from_records(
        work_group_code: impl Into<String>,
        meeting_slug: impl Into<String>,
        docs_url: impl Into<String>,
        checked_at: impl Into<String>,
        mut files: Vec<FileRecord>,
    ) -> Self {
        files.sort_by(|left, right| left.file_name.cmp(&right.file_name));
        Self {
            schema_version: 1,
            record_type: "tdoc-meeting-records".to_string(),
            work_group_code: work_group_code.into(),
            meeting_slug: meeting_slug.into(),
            docs_url: docs_url.into(),
            checked_at: checked_at.into(),
            files,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TDocIndexEntry {
    pub tdoc: String,
    pub file_name: String,
    pub url: String,
    pub work_group_code: String,
    pub meeting_slug: String,
    pub record_shard: String,
    pub remote_modified_raw: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TDocIndexShard {
    pub schema_version: u32,
    pub record_type: String,
    pub prefix: String,
    pub year: u32,
    pub items: BTreeMap<String, TDocIndexEntry>,
}

impl TDocIndexShard {
    pub fn new(prefix: impl Into<String>, year: u32, entries: Vec<TDocIndexEntry>) -> Self {
        let items = entries
            .into_iter()
            .map(|entry| (entry.tdoc.clone(), entry))
            .collect::<BTreeMap<_, _>>();
        Self {
            schema_version: 1,
            record_type: "tdoc-lookup-index".to_string(),
            prefix: prefix.into(),
            year,
            items,
        }
    }
}
```

- [ ] **Step 5: Run test to verify it passes**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core --test sharded_catalog_tests
```

Expected: pass.

---

### Task 2: Catalog Shard IO And Summary Counts

**Files:**
- Modify: `crates/3gpp-core/src/catalog.rs`
- Modify: `crates/3gpp-core/tests/sharded_catalog_tests.rs`

- [ ] **Step 1: Add failing tests for shard paths, writes, reads, and summary**

Append to `crates/3gpp-core/tests/sharded_catalog_tests.rs`:

```rust
use spectrum_3gpp_core::catalog::{
    read_tdoc_index_shard, read_tdoc_meeting_shard, summarize_catalog, tdoc_index_shard_path,
    tdoc_meeting_shard_path, write_tdoc_index_shard, write_tdoc_meeting_shard, CatalogPaths,
};

#[test]
fn catalog_writes_meeting_shard_and_index_shard() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = CatalogPaths::new(temp.path().join("3gpp"));
    let record = support::ran2_record("R2-2601401", "TSGR2_133bis");
    let meeting_shard = TDocMeetingRecordShard::from_records(
        "RAN2",
        "TSGR2_133bis",
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/",
        "2026-07-02T08:00:00Z",
        vec![record],
    );

    let meeting_path = write_tdoc_meeting_shard(&paths, &meeting_shard).expect("write meeting");
    assert_eq!(meeting_path, paths.root().join("records/tdoc/RAN2/TSGR2_133bis.json"));
    assert_eq!(tdoc_meeting_shard_path(&paths, "RAN2", "TSGR2_133bis"), meeting_path);

    let index_entry = TDocIndexEntry {
        tdoc: "R2-2601401".to_string(),
        file_name: "R2-2601401.zip".to_string(),
        url: "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip".to_string(),
        work_group_code: "RAN2".to_string(),
        meeting_slug: "TSGR2_133bis".to_string(),
        record_shard: "records/tdoc/RAN2/TSGR2_133bis.json".to_string(),
        remote_modified_raw: Some("2026/05/22 10:14".to_string()),
        size_bytes: Some(10_240),
    };
    let index_shard = TDocIndexShard::new("R2", 2026, vec![index_entry]);
    let index_path = write_tdoc_index_shard(&paths, &index_shard).expect("write index");
    assert_eq!(index_path, paths.root().join("indexes/tdoc/R2/26.json"));
    assert_eq!(tdoc_index_shard_path(&paths, "R2", 2026), index_path);

    let read_meeting = read_tdoc_meeting_shard(&paths, "RAN2", "TSGR2_133bis")
        .expect("read meeting")
        .expect("meeting exists");
    let read_index = read_tdoc_index_shard(&paths, "R2", 2026)
        .expect("read index")
        .expect("index exists");
    assert_eq!(read_meeting.files.len(), 1);
    assert!(read_index.items.contains_key("R2-2601401"));

    let summary = summarize_catalog(&paths).expect("summary");
    assert_eq!(summary.record_count, 1);
    assert_eq!(summary.index_count, 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core --test sharded_catalog_tests
```

Expected: fail because catalog shard functions do not exist.

- [ ] **Step 3: Implement shard IO**

In `crates/3gpp-core/src/catalog.rs`, import new model types:

```rust
use crate::model::{DirectoryManifest, FileRecord, TDocIndexShard, TDocMeetingRecordShard};
```

Add directory helpers:

```rust
pub fn tdoc_records_dir(&self) -> PathBuf {
    self.records_dir().join("tdoc")
}

pub fn tdoc_indexes_dir(&self) -> PathBuf {
    self.indexes_dir().join("tdoc")
}
```

Update `ensure_dirs()` to create those directories.

Add:

```rust
pub fn tdoc_meeting_shard_path(
    paths: &CatalogPaths,
    work_group_code: &str,
    meeting_slug: &str,
) -> PathBuf {
    paths
        .tdoc_records_dir()
        .join(windows_safe_file_name_component(work_group_code))
        .join(format!("{}.json", windows_safe_file_name_component(meeting_slug)))
}

pub fn tdoc_index_shard_path(paths: &CatalogPaths, prefix: &str, year: u32) -> PathBuf {
    paths
        .tdoc_indexes_dir()
        .join(windows_safe_file_name_component(prefix))
        .join(format!("{:02}.json", year % 100))
}

pub fn write_tdoc_meeting_shard(
    paths: &CatalogPaths,
    shard: &TDocMeetingRecordShard,
) -> Result<PathBuf> {
    paths.ensure_dirs()?;
    let path = tdoc_meeting_shard_path(paths, &shard.work_group_code, &shard.meeting_slug);
    write_json_atomic(&path, shard)?;
    Ok(path)
}

pub fn read_tdoc_meeting_shard(
    paths: &CatalogPaths,
    work_group_code: &str,
    meeting_slug: &str,
) -> Result<Option<TDocMeetingRecordShard>> {
    paths.ensure_dirs()?;
    let path = tdoc_meeting_shard_path(paths, work_group_code, meeting_slug);
    if !path.exists() {
        return Ok(None);
    }
    let body = fs::read(&path).map_err(|source| GppError::Io {
        path: path.display().to_string(),
        source,
    })?;
    Ok(Some(serde_json::from_slice(&body)?))
}

pub fn write_tdoc_index_shard(paths: &CatalogPaths, shard: &TDocIndexShard) -> Result<PathBuf> {
    paths.ensure_dirs()?;
    let path = tdoc_index_shard_path(paths, &shard.prefix, shard.year);
    write_json_atomic(&path, shard)?;
    Ok(path)
}

pub fn read_tdoc_index_shard(
    paths: &CatalogPaths,
    prefix: &str,
    year: u32,
) -> Result<Option<TDocIndexShard>> {
    paths.ensure_dirs()?;
    let path = tdoc_index_shard_path(paths, prefix, year);
    if !path.exists() {
        return Ok(None);
    }
    let body = fs::read(&path).map_err(|source| GppError::Io {
        path: path.display().to_string(),
        source,
    })?;
    Ok(Some(serde_json::from_slice(&body)?))
}
```

Update `summarize_catalog` so `record_count` counts files inside meeting shards plus legacy records, and `index_count` recursively counts index JSON files.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core --test sharded_catalog_tests
```

Expected: pass.

---

### Task 3: Build And Resolve Index Shards

**Files:**
- Modify: `crates/3gpp-core/src/index.rs`
- Modify: `crates/3gpp-core/tests/sharded_catalog_tests.rs`

- [ ] **Step 1: Add failing tests for index creation and lookup**

Append:

```rust
use spectrum_3gpp_core::index::{build_tdoc_index_shards, resolve_tdoc_from_index_shard};

#[test]
fn builds_index_shards_by_prefix_and_year() {
    let first = support::ran2_record("R2-2601401", "TSGR2_133bis");
    let second = support::ran2_record("R2-2508956", "TSGR2_132");
    let shards = build_tdoc_index_shards(&[first, second]);

    assert_eq!(shards.len(), 2);
    assert!(shards.iter().any(|shard| shard.prefix == "R2" && shard.year == 2026));
    assert!(shards.iter().any(|shard| shard.prefix == "R2" && shard.year == 2025));
}

#[test]
fn resolves_tdoc_from_single_index_shard() {
    let record = support::ran2_record("R2-2601401", "TSGR2_133bis");
    let shard = build_tdoc_index_shards(&[record])
        .into_iter()
        .next()
        .expect("index shard");

    let resolved = resolve_tdoc_from_index_shard("R2-2601401", &shard).expect("resolved");
    assert_eq!(resolved.url, "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip");
    assert_eq!(resolved.record_shard, "records/tdoc/RAN2/TSGR2_133bis.json");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core --test sharded_catalog_tests
```

Expected: fail because index functions do not exist.

- [ ] **Step 3: Implement index shard helpers**

Add to `crates/3gpp-core/src/index.rs`:

```rust
use std::collections::BTreeMap;

use crate::model::{FileRecord, TDocIndexEntry, TDocIndexShard};

pub fn build_tdoc_index_shards(records: &[FileRecord]) -> Vec<TDocIndexShard> {
    let mut groups: BTreeMap<(String, u32), Vec<TDocIndexEntry>> = BTreeMap::new();
    for record in records {
        let Some(tdoc) = &record.tdoc else {
            continue;
        };
        let Some(year) = tdoc.year_hint else {
            continue;
        };
        let Some(work_group_code) = record.work_group_code.clone() else {
            continue;
        };
        let Some(meeting_slug) = record.meeting_slug.clone() else {
            continue;
        };
        let entry = TDocIndexEntry {
            tdoc: tdoc.key.clone(),
            file_name: record.file_name.clone(),
            url: record.canonical_url.clone(),
            work_group_code: work_group_code.clone(),
            meeting_slug: meeting_slug.clone(),
            record_shard: format!("records/tdoc/{work_group_code}/{meeting_slug}.json"),
            remote_modified_raw: record.remote_modified_raw.clone(),
            size_bytes: record.size_bytes,
        };
        groups
            .entry((tdoc.prefix.clone(), year))
            .or_default()
            .push(entry);
    }

    groups
        .into_iter()
        .map(|((prefix, year), entries)| TDocIndexShard::new(prefix, year, entries))
        .collect()
}

pub fn resolve_tdoc_from_index_shard<'a>(
    query: &str,
    shard: &'a TDocIndexShard,
) -> Option<&'a TDocIndexEntry> {
    shard.items.get(query)
}
```

Keep the existing `TDocLookupIndex` for legacy path compatibility.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core --test sharded_catalog_tests
```

Expected: pass.

---

### Task 4: Write Shards During Online Discovery

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src-tauri/src/gpp/workflow.rs`
- Test: `apps/desktop/src-tauri/src/lib.rs` unit tests or focused core tests where possible

- [ ] **Step 1: Add failing Rust test for writing discovered records to shards**

Add a unit test in `apps/desktop/src-tauri/src/lib.rs` test module:

```rust
#[test]
fn writes_discovered_records_as_meeting_shard_and_index_shard() {
    use spectrum_3gpp_core::catalog::{
        read_tdoc_index_shard, read_tdoc_meeting_shard, CatalogPaths,
    };

    let temp = tempfile::tempdir().expect("tempdir");
    let paths = CatalogPaths::new(temp.path().join("3gpp"));
    let record = test_ran2_record("R2-2601401", "TSGR2_133bis");

    write_discovered_tdoc_records(
        &paths,
        "RAN2",
        "TSGR2_133bis",
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/",
        "2026-07-02T08:00:00Z",
        &[record],
    )
    .expect("write discovered records");

    assert!(
        read_tdoc_meeting_shard(&paths, "RAN2", "TSGR2_133bis")
            .expect("read meeting")
            .is_some()
    );
    assert!(
        read_tdoc_index_shard(&paths, "R2", 2026)
            .expect("read index")
            .expect("index")
            .items
            .contains_key("R2-2601401")
    );
}
```

Add a local `test_ran2_record` helper using the same fields as the core support fixture.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml writes_discovered_records_as_meeting_shard_and_index_shard
```

Expected: fail because `write_discovered_tdoc_records` does not exist.

- [ ] **Step 3: Implement discovered record shard writing**

Add helper in `apps/desktop/src-tauri/src/lib.rs`:

```rust
fn write_discovered_tdoc_records(
    paths: &CatalogPaths,
    work_group_code: &str,
    meeting_slug: &str,
    docs_url: &str,
    checked_at: &str,
    records: &[FileRecord],
) -> std::result::Result<(), String> {
    use spectrumpilot_3gpp_core::catalog::{write_tdoc_index_shard, write_tdoc_meeting_shard};
    use spectrumpilot_3gpp_core::index::build_tdoc_index_shards;
    use spectrumpilot_3gpp_core::model::TDocMeetingRecordShard;

    let shard = TDocMeetingRecordShard::from_records(
        work_group_code,
        meeting_slug,
        docs_url,
        checked_at,
        records.to_vec(),
    );
    write_tdoc_meeting_shard(paths, &shard).map_err(|source| source.to_string())?;
    for index_shard in build_tdoc_index_shards(records) {
        write_tdoc_index_shard(paths, &index_shard).map_err(|source| source.to_string())?;
    }
    Ok(())
}
```

Use this helper in `fetch_meeting_docs_files` or the call site where the docs manifest and records are known. Keep `write_file_records` only as a compatibility fallback until migration is complete.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml writes_discovered_records_as_meeting_shard_and_index_shard
```

Expected: pass.

---

### Task 5: Query Index Shard Before Legacy Scan

**Files:**
- Modify: `apps/desktop/src-tauri/src/gpp/workflow.rs`
- Test: `apps/desktop/src-tauri/src/gpp/workflow.rs`

- [ ] **Step 1: Add failing test for index-backed contribution target**

Add a pure helper test around a new function `resolve_indexed_contribution_record`. The test should create a temp catalog, write one `TDocIndexShard`, and assert resolving `R2-2601401` returns a `DownloadTarget` with the indexed URL without reading legacy records.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml resolves_contribution_from_index_shard
```

Expected: fail because helper does not exist.

- [ ] **Step 3: Implement indexed resolution helper**

Add helper in `workflow.rs`:

```rust
fn resolve_indexed_contribution_record(
    paths: &CatalogPaths,
    workspace_root: &Path,
    tdoc: &TDocKey,
) -> LookupResult<Option<DownloadTarget>> {
    use spectrumpilot_3gpp_core::catalog::read_tdoc_index_shard;
    use spectrumpilot_3gpp_core::index::resolve_tdoc_from_index_shard;

    let Some(year) = tdoc.year_hint else {
        return Ok(None);
    };
    let Some(shard) = read_tdoc_index_shard(paths, &tdoc.prefix, year)
        .map_err(|source| source.to_string())?
    else {
        return Ok(None);
    };
    let Some(entry) = resolve_tdoc_from_index_shard(&tdoc.key, &shard) else {
        return Ok(None);
    };
    Ok(Some(contribution_target_from_direct_url(
        workspace_root,
        &entry.work_group_code,
        &entry.meeting_slug,
        &entry.tdoc,
        &entry.url,
    )))
}
```

Call this helper before legacy `read_file_records`.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml resolves_contribution_from_index_shard
```

Expected: pass.

---

### Task 6: Seed Installer Supports Record And Index Shards

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Possibly create: `apps/desktop/src-tauri/src/seed.rs`
- Test: `apps/desktop/src-tauri/src/lib.rs`

- [ ] **Step 1: Add failing test for independent seed installation**

Add a test proving empty catalog installs manifests, record shards, and index shards; and that a catalog with only manifests still installs missing records and indexes.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml installs_bundled_seed_records_and_indexes_independently
```

Expected: fail because installer only installs manifests.

- [ ] **Step 3: Implement seed file copy by embedded directory listing**

Use either a generated include list or add `include_dir = "0.7"` to `apps/desktop/src-tauri/Cargo.toml`. Prefer `include_dir` to avoid thousands of handwritten `include_str!` entries.

Add:

```rust
static BUNDLED_CATALOG_SEED: include_dir::Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/resources/3gpp/catalog_seed");
```

Then copy subdirectories:

```rust
fn install_seed_subtree_if_empty(
    paths: &CatalogPaths,
    seed_subdir: &str,
    target_dir: &Path,
) -> std::result::Result<usize, String> {
    if count_json_files_recursive(target_dir)? > 0 {
        return Ok(0);
    }
    let Some(seed_dir) = BUNDLED_CATALOG_SEED.get_dir(seed_subdir) else {
        return Ok(0);
    };
    copy_include_dir_json_files(seed_dir, target_dir)
}
```

Install:

```text
manifests -> catalog/manifests
records -> catalog/records
indexes -> catalog/indexes
```

Each subtree must be independent.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml installs_bundled_seed_records_and_indexes_independently
```

Expected: pass.

---

### Task 7: Migrate Current Local Records To Sharded Seed

**Files:**
- Create: `scripts/3gpp/migrate_legacy_records_to_shards.py`
- Generated:
  - `apps/desktop/src-tauri/resources/3gpp/catalog_seed/records/tdoc/**.json`
  - `apps/desktop/src-tauri/resources/3gpp/catalog_seed/indexes/tdoc/**.json`

- [ ] **Step 1: Create migration script**

The script must:

1. Read legacy JSON files from `~/.cache/com.hzh.spectrumpilot/3gpp/catalog/records`.
2. Group records by `(workGroupCode, meetingSlug)`.
3. Write meeting shards to `apps/desktop/src-tauri/resources/3gpp/catalog_seed/records/tdoc/<workGroupCode>/<meetingSlug>.json`.
4. Build index shards by `(tdoc.prefix, tdoc.yearHint)`.
5. Write indexes to `apps/desktop/src-tauri/resources/3gpp/catalog_seed/indexes/tdoc/<prefix>/<yy>.json`.
6. Print counts.

- [ ] **Step 2: Run migration script**

Run:

```bash
python3 scripts/3gpp/migrate_legacy_records_to_shards.py \
  --source ~/.cache/com.hzh.spectrumpilot/3gpp/catalog/records \
  --target apps/desktop/src-tauri/resources/3gpp/catalog_seed
```

Expected: generated meeting shard and index shard counts are printed, and no legacy per-file record JSON is copied into seed.

- [ ] **Step 3: Inspect generated seed size and counts**

Run:

```bash
find apps/desktop/src-tauri/resources/3gpp/catalog_seed/records/tdoc -type f -name '*.json' | wc -l
find apps/desktop/src-tauri/resources/3gpp/catalog_seed/indexes/tdoc -type f -name '*.json' | wc -l
```

Expected: meeting shard count is far smaller than legacy record count; index shard count is grouped by prefix/year.

---

### Task 8: UI Status Wording

**Files:**
- Modify: `apps/desktop/src/pages/GppPage.tsx`
- Modify: `apps/desktop/src/pages/GppPage.test.tsx`
- Modify: `apps/desktop/src/pages/SettingsPage.tsx`
- Modify: `apps/desktop/src/pages/SettingsPage.test.tsx`

- [ ] **Step 1: Update failing UI tests**

Change assertions so the 3GPP page expects `indexed TDocs`, not `cached files`. Change Settings to show `Record shards`, `Index shards`, and `Indexed TDocs`.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
npm test -- --run src/pages/GppPage.test.tsx src/pages/SettingsPage.test.tsx
```

Expected: fail because UI still says `cached files`.

- [ ] **Step 3: Update UI wording**

Change the GppPage pill to:

```tsx
<span>{catalogStatus?.recordCount ?? 0} indexed TDocs</span>
```

Change Settings labels to avoid implying that seed status only covers manifests.

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
npm test -- --run src/pages/GppPage.test.tsx src/pages/SettingsPage.test.tsx
```

Expected: pass.

---

### Task 9: Full Verification

- [ ] **Step 1: Run core tests**

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core
```

Expected: all core tests pass.

- [ ] **Step 2: Run desktop Rust tests**

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml
```

Expected: all desktop Rust tests pass.

- [ ] **Step 3: Run frontend tests**

```bash
npm test -- --run
```

Expected: all frontend tests pass.

- [ ] **Step 4: Run frontend build**

```bash
npm run build
```

Expected: build passes. Vite chunk-size warning is acceptable if unchanged.

- [ ] **Step 5: Report seed counts**

Report:

```bash
find apps/desktop/src-tauri/resources/3gpp/catalog_seed/records/tdoc -type f -name '*.json' | wc -l
find apps/desktop/src-tauri/resources/3gpp/catalog_seed/indexes/tdoc -type f -name '*.json' | wc -l
```

Expected: counts are included in final response.

---

## Self-Review

Spec coverage:
- Replaces one-file-per-TDoc cache with meeting shards: Tasks 1, 2, 4, 7.
- Avoids thousands of IO reads for normal lookup: Tasks 3 and 5.
- Keeps manifests as directory-level JSON for future incremental refresh: Architecture and Task 2.
- Packages staged seed into installable app data: Tasks 6 and 7.
- Updates UI wording to match new semantics: Task 8.

Placeholder scan:
- No unresolved placeholders are intentionally left. Where an implementation choice remains, the task names the preferred approach and exact dependency.

Type consistency:
- `TDocMeetingRecordShard`, `TDocIndexEntry`, and `TDocIndexShard` are introduced in Task 1 and used consistently in later tasks.
- `record_count` remains the UI-facing indexed TDoc count for compatibility with current frontend API.
