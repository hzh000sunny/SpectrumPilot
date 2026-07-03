use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::{FileRecord, TDocIndexEntry, TDocIndexShard};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TDocLookupIndex {
    pub schema_version: u32,
    pub index_type: String,
    pub items: BTreeMap<String, Vec<String>>,
}

impl TDocLookupIndex {
    pub fn from_files(files: &[FileRecord]) -> Self {
        let mut items: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for file in files {
            if !file.classification.is_primary_tdoc {
                continue;
            }
            let Some(tdoc) = &file.tdoc else {
                continue;
            };
            items
                .entry(tdoc.key.to_ascii_uppercase())
                .or_default()
                .push(file.id.clone());
        }

        Self {
            schema_version: 1,
            index_type: "by-tdoc".to_string(),
            items,
        }
    }
}

pub fn build_tdoc_index_shards(records: &[FileRecord]) -> Vec<TDocIndexShard> {
    let mut groups: BTreeMap<(String, u32), Vec<TDocIndexEntry>> = BTreeMap::new();

    for record in records {
        if !record.classification.is_primary_tdoc {
            continue;
        }
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
    shard.items.get(&query.to_ascii_uppercase())
}
