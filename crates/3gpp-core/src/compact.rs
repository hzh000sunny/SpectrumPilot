use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::catalog::CatalogPaths;
use crate::error::{GppError, Result};
use crate::model::TDocIndexEntry;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactCatalogSummary {
    pub schema_version: u32,
    pub record_type: String,
    pub catalog_format: String,
    pub record_count: usize,
    pub meeting_count: usize,
    pub record_shard_count: usize,
    pub index_shard_count: usize,
    pub index_item_count: usize,
    pub latest_checked_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompactTDocIndexShard {
    schema_version: u32,
    record_type: String,
    prefix: String,
    year: u32,
    items: BTreeMap<String, CompactTDocIndexPointer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompactTDocIndexPointer(String, usize, usize);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompactWorkGroupRecords {
    schema_version: u32,
    record_type: String,
    work_group_code: String,
    base_url: String,
    meetings: Vec<CompactMeetingRecords>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompactMeetingRecords {
    id: usize,
    meeting_slug: String,
    docs_path: String,
    checked_at: String,
    files: Vec<CompactFileRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompactFileRecord(String, Option<u64>, Option<String>, String);

pub fn compact_summary_path(paths: &CatalogPaths) -> PathBuf {
    paths.root().join("compact").join("summary.json")
}

pub fn read_compact_summary(paths: &CatalogPaths) -> Result<Option<CompactCatalogSummary>> {
    let path = compact_summary_path(paths);
    if !path.exists() {
        return Ok(None);
    }
    read_json(&path).map(Some)
}

pub fn resolve_tdoc_from_compact_catalog(
    paths: &CatalogPaths,
    prefix: &str,
    year: u32,
    tdoc: &str,
) -> Result<Option<TDocIndexEntry>> {
    let index_path = paths.root().join("compact").join("index").join(format!(
        "{}_{:02}.json",
        safe_component(prefix),
        year % 100
    ));
    if !index_path.exists() {
        return Ok(None);
    }

    let index: CompactTDocIndexShard = read_json(&index_path)?;
    let Some(pointer) = index.items.get(&tdoc.to_ascii_uppercase()) else {
        return Ok(None);
    };

    let records_path = paths
        .root()
        .join("compact")
        .join("records")
        .join(format!("{}.json", safe_component(&pointer.0)));
    let records: CompactWorkGroupRecords = read_json(&records_path)?;
    let Some(meeting) = records
        .meetings
        .iter()
        .find(|meeting| meeting.id == pointer.1)
    else {
        return Ok(None);
    };
    let Some(file) = meeting.files.get(pointer.2) else {
        return Ok(None);
    };

    let url = format!(
        "{}/{}/{}",
        records.base_url.trim_end_matches('/'),
        meeting.docs_path.trim_matches('/'),
        file.0
    );

    Ok(Some(TDocIndexEntry {
        tdoc: file.3.clone(),
        file_name: file.0.clone(),
        url,
        work_group_code: records.work_group_code,
        meeting_slug: meeting.meeting_slug.clone(),
        record_shard: format!("compact/records/{}.json", safe_component(&pointer.0)),
        remote_modified_raw: file.2.clone(),
        size_bytes: file.1,
    }))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &PathBuf) -> Result<T> {
    let body = fs::read(path).map_err(|source| GppError::Io {
        path: path.display().to_string(),
        source,
    })?;
    Ok(serde_json::from_slice(&body)?)
}

fn safe_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
