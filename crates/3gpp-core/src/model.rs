use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectoryRole {
    Root,
    WorkGroup,
    MeetingRoot,
    Docs,
    Inbox,
    Report,
    Agenda,
    Auxiliary,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntryKind {
    Directory,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocsState {
    Unknown,
    Available,
    Empty,
    Missing,
    Forbidden,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryChild {
    pub name: String,
    pub kind: EntryKind,
    pub url: String,
    pub role: DirectoryRole,
    pub remote_modified_raw: Option<String>,
    pub size_raw: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryManifest {
    pub schema_version: u32,
    pub record_type: String,
    pub url: String,
    pub path_segments: Vec<String>,
    pub directory_role: DirectoryRole,
    pub checked_at: String,
    pub child_fingerprint: String,
    pub children: Vec<DirectoryChild>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingRecord {
    pub schema_version: u32,
    pub record_type: String,
    pub id: String,
    pub root: String,
    pub work_group_path: String,
    pub work_group_code: Option<String>,
    pub work_group_label: Option<String>,
    pub meeting_slug: String,
    pub meeting_series: Option<String>,
    pub meeting_number: Option<u32>,
    pub meeting_variant: Option<String>,
    pub location: Option<String>,
    pub scheduled_month: Option<String>,
    pub url: String,
    pub docs_url: Option<String>,
    pub docs_state: DocsState,
    pub last_seen_remote_modified_raw: Option<String>,
}

impl MeetingRecord {
    pub fn stable_id(root: &str, work_group_path: &str, meeting_slug: &str) -> String {
        format!("meeting:{root}/{work_group_path}/{meeting_slug}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TDocKey {
    pub key: String,
    pub prefix: String,
    pub number_text: String,
    pub year_hint: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileClassification {
    pub is_primary_tdoc: bool,
    pub is_zip: bool,
    pub is_ignored_artifact: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRecord {
    pub schema_version: u32,
    pub record_type: String,
    pub id: String,
    pub canonical_url: String,
    pub parent_directory_url: String,
    pub root: String,
    pub work_group_path: String,
    pub work_group_code: Option<String>,
    pub meeting_id: Option<String>,
    pub meeting_slug: Option<String>,
    pub container_role: DirectoryRole,
    pub file_name: String,
    pub extension: Option<String>,
    pub remote_modified_raw: Option<String>,
    pub size_raw: Option<String>,
    pub size_bytes: Option<u64>,
    pub tdoc: Option<TDocKey>,
    pub classification: FileClassification,
}

impl FileRecord {
    pub fn stable_id(canonical_url: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(canonical_url.as_bytes());
        format!("file-url-sha256:{:x}", hasher.finalize())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecArchiveRecord {
    pub schema_version: u32,
    pub record_type: String,
    pub spec_number: String,
    pub archive_url: String,
    pub checked_at: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LookupHistoryRecord {
    pub schema_version: u32,
    pub record_type: String,
    pub query: String,
    pub source_url: String,
    pub zip_path: String,
    pub extracted_path: String,
    pub opened_path: Option<String>,
    pub cache_status: String,
    pub message: String,
    pub completed_at: String,
}

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
