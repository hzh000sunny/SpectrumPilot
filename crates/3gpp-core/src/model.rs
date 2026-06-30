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
pub struct TDocKey {
    pub key: String,
    pub prefix: String,
    pub number_text: String,
    pub year_hint: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileClassification {
    pub is_primary_tdoc: bool,
    pub is_zip: bool,
    pub is_ignored_artifact: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
