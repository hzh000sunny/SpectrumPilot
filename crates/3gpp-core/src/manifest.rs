use sha2::{Digest, Sha256};
use url::Url;

use crate::error::{GppError, Result};
use crate::model::{
    DirectoryChild, DirectoryManifest, DirectoryRole, DocsState, FileClassification, FileRecord,
    MeetingRecord,
};
use crate::normalize::{infer_work_group, parse_meeting_slug, parse_tdoc_key};
use crate::parser::parse_directory_listing;

pub fn build_manifest_from_html(
    directory_url: &str,
    directory_role: DirectoryRole,
    checked_at: &str,
    html: &str,
) -> Result<DirectoryManifest> {
    let parsed_url =
        Url::parse(directory_url).map_err(|_| GppError::InvalidUrl(directory_url.to_string()))?;
    let path_segments = parsed_url
        .path_segments()
        .map(|segments| {
            segments
                .filter(|segment| !segment.is_empty() && *segment != "ftp")
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let children = parse_directory_listing(html)?
        .into_iter()
        .map(|row| DirectoryChild {
            name: row.name,
            kind: row.kind,
            url: row.url,
            role: row.role,
            remote_modified_raw: row.remote_modified_raw,
            size_raw: row.size_raw,
            size_bytes: row.size_bytes,
        })
        .collect::<Vec<_>>();

    let child_fingerprint = child_fingerprint(&children);

    Ok(DirectoryManifest {
        schema_version: 1,
        record_type: "directory-manifest".to_string(),
        url: directory_url.to_string(),
        path_segments,
        directory_role,
        checked_at: checked_at.to_string(),
        child_fingerprint,
        children,
    })
}

pub fn child_fingerprint(children: &[DirectoryChild]) -> String {
    let mut normalized = children
        .iter()
        .map(|child| {
            format!(
                "{}\t{:?}\t{}\t{}\t{}",
                child.name,
                child.kind,
                child.url,
                child.remote_modified_raw.as_deref().unwrap_or(""),
                child.size_raw.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>();
    normalized.sort();

    let mut hasher = Sha256::new();
    for item in normalized {
        hasher.update(item.as_bytes());
        hasher.update(b"\n");
    }

    format!("sha256:{:x}", hasher.finalize())
}

pub fn meeting_from_manifest(manifest: &DirectoryManifest) -> Result<MeetingRecord> {
    if manifest.path_segments.len() < 3 {
        return Err(GppError::Parse(format!(
            "meeting path needs at least 3 segments: {}",
            manifest.url
        )));
    }

    let root = manifest.path_segments[0].clone();
    let work_group_path = manifest.path_segments[1].clone();
    let meeting_slug = manifest.path_segments[2].clone();
    let parsed = parse_meeting_slug(&meeting_slug);
    let wg = infer_work_group(&root, &work_group_path);
    let docs_child = manifest
        .children
        .iter()
        .find(|child| child.role == DirectoryRole::Docs);
    let docs_state = if docs_child.is_some() {
        DocsState::Available
    } else {
        DocsState::Missing
    };

    Ok(MeetingRecord {
        schema_version: 1,
        record_type: "meeting".to_string(),
        id: MeetingRecord::stable_id(&root, &work_group_path, &meeting_slug),
        root,
        work_group_path,
        work_group_code: wg.code,
        work_group_label: wg.label,
        meeting_slug,
        meeting_series: parsed.series,
        meeting_number: parsed.number,
        meeting_variant: parsed.variant,
        location: parsed.location,
        scheduled_month: parsed.scheduled_month,
        url: manifest.url.clone(),
        docs_url: docs_child.map(|child| child.url.clone()),
        docs_state,
        last_seen_remote_modified_raw: None,
    })
}

pub fn file_records_from_docs_manifest(manifest: &DirectoryManifest) -> Result<Vec<FileRecord>> {
    if manifest.path_segments.len() < 4 {
        return Err(GppError::Parse(format!(
            "docs path needs at least 4 segments: {}",
            manifest.url
        )));
    }

    let root = manifest.path_segments[0].clone();
    let work_group_path = manifest.path_segments[1].clone();
    let meeting_slug = manifest.path_segments[2].clone();
    let wg = infer_work_group(&root, &work_group_path);
    let meeting_id = MeetingRecord::stable_id(&root, &work_group_path, &meeting_slug);

    Ok(manifest
        .children
        .iter()
        .map(|child| {
            let extension = child
                .name
                .rsplit_once('.')
                .map(|(_, ext)| ext.to_ascii_lowercase());
            let is_zip = extension.as_deref() == Some("zip");
            let tdoc = if is_zip {
                parse_tdoc_key(&child.name)
            } else {
                None
            };
            let is_ignored_artifact = child.name == "__MACOSX";
            let is_primary_tdoc = is_zip && tdoc.is_some() && !is_ignored_artifact;

            FileRecord {
                schema_version: 1,
                record_type: if is_primary_tdoc {
                    "tdoc-file".to_string()
                } else {
                    "auxiliary-file".to_string()
                },
                id: FileRecord::stable_id(&child.url),
                canonical_url: child.url.clone(),
                parent_directory_url: manifest.url.clone(),
                root: root.clone(),
                work_group_path: work_group_path.clone(),
                work_group_code: wg.code.clone(),
                meeting_id: Some(meeting_id.clone()),
                meeting_slug: Some(meeting_slug.clone()),
                container_role: DirectoryRole::Docs,
                file_name: child.name.clone(),
                extension,
                remote_modified_raw: child.remote_modified_raw.clone(),
                size_raw: child.size_raw.clone(),
                size_bytes: child.size_bytes,
                tdoc,
                classification: FileClassification {
                    is_primary_tdoc,
                    is_zip,
                    is_ignored_artifact,
                },
            }
        })
        .collect())
}
