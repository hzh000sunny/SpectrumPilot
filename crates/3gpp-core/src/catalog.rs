use std::collections::BTreeSet;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::compact::read_compact_summary;
use crate::error::{GppError, Result};
use crate::model::{
    DirectoryManifest, FileRecord, LookupHistoryRecord, SpecArchiveRecord, TDocIndexShard,
    TDocMeetingRecordShard,
};

#[derive(Debug, Clone)]
pub struct CatalogPaths {
    root: PathBuf,
}

impl CatalogPaths {
    pub fn new(app_cache_3gpp_dir: impl AsRef<Path>) -> Self {
        Self {
            root: app_cache_3gpp_dir.as_ref().join("catalog"),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifests_dir(&self) -> PathBuf {
        self.root.join("manifests")
    }

    pub fn records_dir(&self) -> PathBuf {
        self.root.join("records")
    }

    pub fn indexes_dir(&self) -> PathBuf {
        self.root.join("indexes")
    }

    pub fn tdoc_records_dir(&self) -> PathBuf {
        self.records_dir().join("tdoc")
    }

    pub fn tdoc_indexes_dir(&self) -> PathBuf {
        self.indexes_dir().join("tdoc")
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        for dir in [
            self.root(),
            &self.manifests_dir(),
            &self.records_dir(),
            &self.indexes_dir(),
            &self.tdoc_records_dir(),
            &self.tdoc_indexes_dir(),
        ] {
            fs::create_dir_all(dir).map_err(|source| GppError::Io {
                path: dir.display().to_string(),
                source,
            })?;
        }
        Ok(())
    }
}

pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| GppError::Io {
            path: parent.display().to_string(),
            source,
        })?;
    }

    let tmp_path = path.with_extension("tmp");
    let body = serde_json::to_vec_pretty(value)?;
    fs::write(&tmp_path, body).map_err(|source| GppError::Io {
        path: tmp_path.display().to_string(),
        source,
    })?;
    fs::rename(&tmp_path, path).map_err(|source| GppError::Io {
        path: path.display().to_string(),
        source,
    })?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogSummary {
    pub schema_version: u32,
    pub manifest_count: usize,
    pub record_count: usize,
    pub index_count: usize,
    pub last_checked_at: Option<String>,
}

pub fn manifest_path_for_url(paths: &CatalogPaths, url: &str) -> PathBuf {
    paths
        .manifests_dir()
        .join(format!("{}.json", stable_dir_id(url)))
}

pub fn write_manifest(paths: &CatalogPaths, manifest: &DirectoryManifest) -> Result<PathBuf> {
    paths.ensure_dirs()?;
    let path = manifest_path_for_url(paths, &manifest.url);
    write_json_atomic(&path, manifest)?;
    Ok(path)
}

pub fn file_record_path(paths: &CatalogPaths, record: &FileRecord) -> PathBuf {
    paths.records_dir().join(format!(
        "{}.json",
        windows_safe_file_name_component(&record.id)
    ))
}

pub fn spec_archive_record_path(paths: &CatalogPaths, spec_number: &str) -> PathBuf {
    paths
        .records_dir()
        .join("specs")
        .join(windows_safe_file_name_component(&spec_series_dir(
            spec_number,
        )))
        .join(format!(
            "{}.json",
            windows_safe_file_name_component(spec_number)
        ))
}

pub fn lookup_history_path(paths: &CatalogPaths) -> PathBuf {
    paths.root().join("history").join("lookups.jsonl")
}

pub fn tdoc_meeting_shard_path(
    paths: &CatalogPaths,
    work_group_code: &str,
    meeting_slug: &str,
) -> PathBuf {
    paths
        .tdoc_records_dir()
        .join(windows_safe_file_name_component(work_group_code))
        .join(format!(
            "{}.json",
            windows_safe_file_name_component(meeting_slug)
        ))
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

pub fn merge_tdoc_index_shard(paths: &CatalogPaths, shard: &TDocIndexShard) -> Result<PathBuf> {
    paths.ensure_dirs()?;
    let mut merged = read_tdoc_index_shard(paths, &shard.prefix, shard.year)?
        .unwrap_or_else(|| TDocIndexShard::new(shard.prefix.clone(), shard.year, Vec::new()));
    for (tdoc, entry) in &shard.items {
        merged.items.insert(tdoc.clone(), entry.clone());
    }
    write_tdoc_index_shard(paths, &merged)
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

pub fn write_spec_archive_record(
    paths: &CatalogPaths,
    record: &SpecArchiveRecord,
) -> Result<PathBuf> {
    paths.ensure_dirs()?;
    let path = spec_archive_record_path(paths, &record.spec_number);
    write_json_atomic(&path, record)?;
    Ok(path)
}

pub fn read_spec_archive_record(
    paths: &CatalogPaths,
    spec_number: &str,
) -> Result<Option<SpecArchiveRecord>> {
    paths.ensure_dirs()?;
    let path = spec_archive_record_path(paths, spec_number);
    if !path.exists() {
        return Ok(None);
    }
    let body = fs::read(&path).map_err(|source| GppError::Io {
        path: path.display().to_string(),
        source,
    })?;
    Ok(Some(serde_json::from_slice(&body)?))
}

pub fn append_lookup_history_record(
    paths: &CatalogPaths,
    record: &LookupHistoryRecord,
) -> Result<PathBuf> {
    paths.ensure_dirs()?;
    let path = lookup_history_path(paths);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| GppError::Io {
            path: parent.display().to_string(),
            source,
        })?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|source| GppError::Io {
            path: path.display().to_string(),
            source,
        })?;
    let body = serde_json::to_string(record)?;
    writeln!(file, "{body}").map_err(|source| GppError::Io {
        path: path.display().to_string(),
        source,
    })?;
    Ok(path)
}

pub fn read_lookup_history_records(
    paths: &CatalogPaths,
    limit: usize,
) -> Result<Vec<LookupHistoryRecord>> {
    paths.ensure_dirs()?;
    let path = lookup_history_path(paths);
    if limit == 0 || !path.exists() {
        return Ok(Vec::new());
    }

    let body = fs::read_to_string(&path).map_err(|source| GppError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let mut records = Vec::new();
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        records.push(serde_json::from_str::<LookupHistoryRecord>(line)?);
    }
    records.reverse();
    records.truncate(limit);
    Ok(records)
}

pub fn write_file_records(paths: &CatalogPaths, records: &[FileRecord]) -> Result<Vec<PathBuf>> {
    paths.ensure_dirs()?;
    records
        .iter()
        .map(|record| {
            let path = file_record_path(paths, record);
            write_json_atomic(&path, record)?;
            Ok(path)
        })
        .collect()
}

pub fn read_file_records(paths: &CatalogPaths) -> Result<Vec<FileRecord>> {
    paths.ensure_dirs()?;
    if !paths.records_dir().exists() {
        return Ok(Vec::new());
    }

    let mut records = Vec::new();
    for entry in fs::read_dir(paths.records_dir()).map_err(|source| GppError::Io {
        path: paths.records_dir().display().to_string(),
        source,
    })? {
        let entry = entry.map_err(|source| GppError::Io {
            path: paths.records_dir().display().to_string(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }

        let body = fs::read(&path).map_err(|source| GppError::Io {
            path: path.display().to_string(),
            source,
        })?;
        records.push(serde_json::from_slice(&body)?);
    }
    records.sort_by(|left: &FileRecord, right: &FileRecord| left.id.cmp(&right.id));
    Ok(records)
}

pub fn summarize_catalog(paths: &CatalogPaths) -> Result<CatalogSummary> {
    paths.ensure_dirs()?;

    let mut last_checked_at = None;
    let manifest_count = count_json_files(&paths.manifests_dir(), |path| {
        let body = fs::read(path).map_err(|source| GppError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let manifest: DirectoryManifest = serde_json::from_slice(&body)?;
        last_checked_at = max_timestamp(last_checked_at.take(), Some(manifest.checked_at));
        Ok(())
    })?;
    let mut record_ids = BTreeSet::new();
    visit_legacy_file_records(paths, |record| {
        record_ids.insert(record.id);
        Ok(())
    })?;
    visit_tdoc_meeting_shards(paths, |shard| {
        last_checked_at = max_timestamp(last_checked_at.take(), Some(shard.checked_at));
        for record in shard.files {
            record_ids.insert(record.id);
        }
        Ok(())
    })?;
    let compact_summary = read_compact_summary(paths)?;
    if let Some(summary) = &compact_summary {
        last_checked_at = max_timestamp(last_checked_at.take(), summary.latest_checked_at.clone());
    }

    Ok(CatalogSummary {
        schema_version: 1,
        manifest_count,
        record_count: record_ids.len().saturating_add(
            compact_summary
                .as_ref()
                .map_or(0, |summary| summary.record_count),
        ),
        index_count: count_json_files_recursive(&paths.indexes_dir())?.saturating_add(
            compact_summary
                .as_ref()
                .map_or(0, |summary| summary.index_shard_count),
        ),
        last_checked_at,
    })
}

fn stable_dir_id(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn windows_safe_file_name_component(value: &str) -> String {
    value.replace(':', "-")
}

fn spec_series_dir(spec_number: &str) -> String {
    let series = spec_number
        .split_once('.')
        .map(|(series, _)| series)
        .unwrap_or(spec_number);
    format!("{series}_series")
}

fn max_timestamp(current: Option<String>, candidate: Option<String>) -> Option<String> {
    match (current, candidate) {
        (Some(current), Some(candidate)) => Some(current.max(candidate)),
        (Some(current), None) => Some(current),
        (None, Some(candidate)) => Some(candidate),
        (None, None) => None,
    }
}

fn count_json_files(dir: &Path, mut visit: impl FnMut(&Path) -> Result<()>) -> Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }

    let mut count = 0;
    for entry in fs::read_dir(dir).map_err(|source| GppError::Io {
        path: dir.display().to_string(),
        source,
    })? {
        let entry = entry.map_err(|source| GppError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            visit(&path)?;
            count += 1;
        }
    }

    Ok(count)
}

fn count_json_files_recursive(dir: &Path) -> Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }

    let mut count = 0;
    for entry in fs::read_dir(dir).map_err(|source| GppError::Io {
        path: dir.display().to_string(),
        source,
    })? {
        let entry = entry.map_err(|source| GppError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            count += count_json_files_recursive(&path)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("json") {
            count += 1;
        }
    }

    Ok(count)
}

fn visit_legacy_file_records(
    paths: &CatalogPaths,
    mut visit: impl FnMut(FileRecord) -> Result<()>,
) -> Result<()> {
    if !paths.records_dir().exists() {
        return Ok(());
    }

    for entry in fs::read_dir(paths.records_dir()).map_err(|source| GppError::Io {
        path: paths.records_dir().display().to_string(),
        source,
    })? {
        let entry = entry.map_err(|source| GppError::Io {
            path: paths.records_dir().display().to_string(),
            source,
        })?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let body = fs::read(&path).map_err(|source| GppError::Io {
            path: path.display().to_string(),
            source,
        })?;
        visit(serde_json::from_slice(&body)?)?;
    }

    Ok(())
}

fn visit_tdoc_meeting_shards(
    paths: &CatalogPaths,
    mut visit: impl FnMut(TDocMeetingRecordShard) -> Result<()>,
) -> Result<()> {
    visit_tdoc_meeting_shard_dir(&paths.tdoc_records_dir(), &mut visit)
}

fn visit_tdoc_meeting_shard_dir(
    dir: &Path,
    visit: &mut impl FnMut(TDocMeetingRecordShard) -> Result<()>,
) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir).map_err(|source| GppError::Io {
        path: dir.display().to_string(),
        source,
    })? {
        let entry = entry.map_err(|source| GppError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            visit_tdoc_meeting_shard_dir(&path, visit)?;
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let body = fs::read(&path).map_err(|source| GppError::Io {
            path: path.display().to_string(),
            source,
        })?;
        visit(serde_json::from_slice(&body)?)?;
    }

    Ok(())
}
