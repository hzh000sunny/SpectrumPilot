use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use serde::Deserialize;
use spectrumpilot_3gpp_core::catalog::{
    merge_tdoc_index_shard, read_file_records, read_tdoc_index_shard, write_file_records,
    write_manifest, write_tdoc_meeting_shard, CatalogPaths,
};
use spectrumpilot_3gpp_core::index::{
    build_tdoc_index_shards, resolve_tdoc_from_index_shard, TDocLookupIndex,
};
use spectrumpilot_3gpp_core::model::{
    DirectoryRole, EntryKind, FileClassification, FileRecord, MeetingRecord, TDocKey,
    TDocMeetingRecordShard,
};
use spectrumpilot_3gpp_core::query::{
    parse_gpp_query, ContributionQuery, GppQuery, SpecificationQuery,
};
use spectrumpilot_3gpp_core::resolver::resolve_tdoc;
use spectrumpilot_3gpp_core::specs::{
    archive_directory_url, archive_file_name, select_latest_spec_file,
};
use spectrumpilot_3gpp_core::tdoc::{direct_probe_url, source_for_tdoc_prefix};
use tauri::{AppHandle, Emitter};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use super::download::{
    cached_open_path, download_zip, extract_zip, has_cached_zip, resolve_open_path,
    spec_extract_dir, tdoc_extract_dir, zip_path_for_extract_dir,
};
use super::jobs::{GppLookupComplete, GppLookupJobStarted, GppLookupProgress, JobRegistry};

type LookupResult<T> = Result<T, LookupError>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GppLookupRequest {
    pub query: String,
    pub mode: String,
    pub work_group: Option<String>,
    pub meeting_hint: Option<String>,
    pub search_window: String,
    pub open_after_download: bool,
}

#[derive(Debug)]
enum LookupError {
    Cancelled,
    Message(String),
}

impl From<String> for LookupError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for LookupError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}

#[derive(Debug, Clone)]
struct DownloadTarget {
    source_url: String,
    zip_file_name: String,
    extract_dir: PathBuf,
    open_stem: String,
    message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LocalDownloadCache {
    ExtractedDocument(PathBuf),
    ZipOnly,
    Missing,
}

#[tauri::command]
pub async fn start_gpp_lookup_job(
    app: AppHandle,
    registry: tauri::State<'_, JobRegistry>,
    request: GppLookupRequest,
) -> Result<GppLookupJobStarted, String> {
    let job = registry.create_job();
    let job_id = job.id.clone();
    let task_job_id = job.id.clone();
    let registry = registry.inner().clone();
    let token = job.token.clone();

    tauri::async_runtime::spawn(async move {
        run_lookup_job(app, registry, task_job_id, token, request).await;
    });

    Ok(GppLookupJobStarted { job_id })
}

#[tauri::command]
pub fn cancel_gpp_lookup_job(
    registry: tauri::State<'_, JobRegistry>,
    job_id: String,
) -> Result<bool, String> {
    Ok(registry.cancel_job(job_id))
}

async fn run_lookup_job(
    app: AppHandle,
    registry: JobRegistry,
    job_id: String,
    token: CancellationToken,
    request: GppLookupRequest,
) {
    let result = run_lookup_job_inner(&app, &job_id, &token, request.clone()).await;

    match result {
        Ok(complete) => {
            let _ = app.emit("gpp-job-complete", complete);
        }
        Err(LookupError::Cancelled) => {
            emit_progress(
                &app,
                &job_id,
                "cancelled",
                "Lookup cancelled.",
                Some(100),
                0,
            );
        }
        Err(LookupError::Message(message)) => {
            emit_progress(&app, &job_id, "error", message, Some(100), 0);
        }
    }

    registry.finish_job(&job_id);
}

async fn run_lookup_job_inner(
    app: &AppHandle,
    job_id: &str,
    token: &CancellationToken,
    request: GppLookupRequest,
) -> LookupResult<GppLookupComplete> {
    emit_progress(app, job_id, "starting", "Parsing query...", Some(8), 0);
    check_cancelled(token)?;

    let workspace_root = workspace_root(app)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("SpectrumPilot/0.1 3GPP lookup")
        .build()
        .map_err(|source| format!("failed to build HTTP client: {source}"))?;
    let parsed_query = parse_lookup_query(&request)?;

    match parsed_query {
        GppQuery::Specification(query) => {
            resolve_specification(app, job_id, token, request, query, &client, workspace_root).await
        }
        GppQuery::Contribution(query) => {
            resolve_contribution(app, job_id, token, request, query, &client, workspace_root).await
        }
    }
}

fn parse_lookup_query(request: &GppLookupRequest) -> LookupResult<GppQuery> {
    let parsed = parse_gpp_query(&request.query)
        .ok_or_else(|| "Enter a specification number or proposal number.".to_string())?;

    match request.mode.as_str() {
        "auto" => Ok(parsed),
        "specification" => match parsed {
            GppQuery::Specification(_) => Ok(parsed),
            GppQuery::Contribution(_) => {
                Err("Query mode is Specification, but the query looks like a proposal.".into())
            }
        },
        "proposal" => match parsed {
            GppQuery::Contribution(_) => Ok(parsed),
            GppQuery::Specification(_) => {
                Err("Query mode is Proposal, but the query looks like a specification.".into())
            }
        },
        other => Err(format!("unsupported 3GPP lookup mode: {other}").into()),
    }
}

async fn resolve_specification(
    app: &AppHandle,
    job_id: &str,
    token: &CancellationToken,
    request: GppLookupRequest,
    query: SpecificationQuery,
    client: &reqwest::Client,
    workspace_root: PathBuf,
) -> LookupResult<GppLookupComplete> {
    emit_progress(
        app,
        job_id,
        "resolving",
        format!("Resolving specification {}...", query.spec_number),
        Some(22),
        0,
    );
    check_cancelled(token)?;

    let archive_url = archive_directory_url(&query);
    let (file_name, version, searched_url_count) = if let Some(exact_version) = &query.exact_version
    {
        let file_name = archive_file_name(&query, exact_version);
        let url = format!("{archive_url}{file_name}");
        let status = head_status(client, &url).await.unwrap_or(0);
        if !is_successful_exact_probe(status, &url, &file_name) {
            return Err(format!(
                "{} was not found in the 3GPP specification archive.",
                file_name
            )
            .into());
        }
        (file_name, exact_version.clone(), 1)
    } else {
        emit_progress(
            app,
            job_id,
            "listing",
            format!("Reading {} archive listing...", query.spec_number),
            Some(32),
            1,
        );
        let checked_at = Utc::now().to_rfc3339();
        let manifest = crate::fetch_directory_manifest(
            client,
            &archive_url,
            DirectoryRole::Unknown,
            &checked_at,
        )
        .await?;
        let files = manifest
            .children
            .iter()
            .filter(|child| child.kind == EntryKind::File)
            .map(|child| child.name.clone())
            .collect::<Vec<_>>();
        let file_name =
            select_latest_spec_file(&query.archive_stem, query.version_prefix.as_deref(), &files)
                .ok_or_else(|| {
                format!(
                    "No archive file matched {}{}.",
                    query.spec_number,
                    query
                        .version_prefix
                        .as_deref()
                        .map(|value| format!(" {value}"))
                        .unwrap_or_default()
                )
            })?;
        let version = spec_version_from_file(&query.archive_stem, &file_name)
            .ok_or_else(|| format!("failed to parse version from {file_name}"))?;
        (file_name, version, 1)
    };

    let target = DownloadTarget {
        source_url: format!("{archive_url}{file_name}"),
        zip_file_name: file_name.clone(),
        extract_dir: spec_extract_dir(&workspace_root, &query.spec_number, &version),
        open_stem: file_name.trim_end_matches(".zip").to_string(),
        message: format!(
            "Downloaded and extracted {} {}.",
            query.spec_number, version
        ),
    };

    download_extract_open(
        app,
        job_id,
        token,
        &request,
        client,
        target,
        searched_url_count,
    )
    .await
}

async fn resolve_contribution(
    app: &AppHandle,
    job_id: &str,
    token: &CancellationToken,
    request: GppLookupRequest,
    query: ContributionQuery,
    client: &reqwest::Client,
    workspace_root: PathBuf,
) -> LookupResult<GppLookupComplete> {
    let source = source_for_tdoc_prefix(&query.tdoc.prefix).ok_or_else(|| {
        format!(
            "{} is a valid-looking proposal number, but its prefix is not mapped yet.",
            query.tdoc.key
        )
    })?;

    if let Some(forced_work_group) = request
        .work_group
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !forced_work_group.eq_ignore_ascii_case(&source.work_group_code) {
            return Err(format!(
                "{} belongs to {}, but Advanced Scope forces {}.",
                query.tdoc.key, source.work_group_code, forced_work_group
            )
            .into());
        }
    }

    emit_progress(
        app,
        job_id,
        "resolving",
        format!("Checking local catalog for {}...", query.tdoc.key),
        Some(18),
        0,
    );
    check_cancelled(token)?;

    let catalog_paths = crate::app_catalog_paths(app)?;
    if let Some(target) =
        resolve_indexed_contribution_record(&catalog_paths, &workspace_root, &query.tdoc)?
    {
        return download_extract_open(app, job_id, token, &request, client, target, 0).await;
    }

    let records = read_file_records(&catalog_paths).map_err(|source| source.to_string())?;
    let index = TDocLookupIndex::from_files(&records);
    if let Some(record) = resolve_tdoc(&query.tdoc.key, &index, &records) {
        let target = contribution_target_from_record(record, &workspace_root)?;
        return download_extract_open(app, job_id, token, &request, client, target, 0).await;
    }

    let explicit_hint = request
        .meeting_hint
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or(query.meeting_hint.as_deref());

    let mut searched_url_count = 0;
    if let Some(hint) = explicit_hint {
        let meeting_slug = normalize_meeting_hint(&source.meeting_series_prefix, hint);
        emit_progress(
            app,
            job_id,
            "probing",
            format!("Probing {} directly...", meeting_slug),
            Some(30),
            searched_url_count,
        );
        let expected_file_name = format!("{}.zip", query.tdoc.key);
        let url = direct_probe_url(&source, &meeting_slug, &query.tdoc);
        searched_url_count += 1;
        if probe_exact_file(client, &url, &expected_file_name).await? {
            cache_direct_contribution_record(app, &source, &meeting_slug, &query.tdoc, &url);
            let target = contribution_target_from_direct_url(
                &workspace_root,
                &source.work_group_code,
                &meeting_slug,
                &query.tdoc.key,
                &url,
            );
            return download_extract_open(
                app,
                job_id,
                token,
                &request,
                client,
                target,
                searched_url_count,
            )
            .await;
        }
    }

    check_cancelled(token)?;
    if let Some((meeting_slug, url, probed_count)) = probe_meeting_candidates(
        app,
        job_id,
        token,
        client,
        &request,
        &query,
        &source,
        searched_url_count,
    )
    .await?
    {
        searched_url_count = probed_count;
        cache_direct_contribution_record(app, &source, &meeting_slug, &query.tdoc, &url);
        let target = contribution_target_from_direct_url(
            &workspace_root,
            &source.work_group_code,
            &meeting_slug,
            &query.tdoc.key,
            &url,
        );
        return download_extract_open(
            app,
            job_id,
            token,
            &request,
            client,
            target,
            searched_url_count,
        )
        .await;
    }

    emit_progress(
        app,
        job_id,
        "listing",
        format!("Scanning Docs listings for {}...", query.tdoc.key),
        Some(52),
        searched_url_count,
    );
    check_cancelled(token)?;
    let checked_at = Utc::now().to_rfc3339();
    let (records, listing_count) =
        crate::search_online_tdoc(&catalog_paths, &query.tdoc.key, &checked_at, client).await?;
    searched_url_count += listing_count;
    let Some(record) = records.first() else {
        return Err(format!("No matching proposal was found for {}.", query.tdoc.key).into());
    };
    let target = contribution_target_from_record(record, &workspace_root)?;
    download_extract_open(
        app,
        job_id,
        token,
        &request,
        client,
        target,
        searched_url_count,
    )
    .await
}

async fn probe_meeting_candidates(
    app: &AppHandle,
    job_id: &str,
    token: &CancellationToken,
    client: &reqwest::Client,
    request: &GppLookupRequest,
    query: &ContributionQuery,
    source: &spectrumpilot_3gpp_core::normalize::TDocSource,
    searched_url_count: usize,
) -> LookupResult<Option<(String, String, usize)>> {
    emit_progress(
        app,
        job_id,
        "listing",
        format!("Reading {} meeting list...", source.work_group_code),
        Some(36),
        searched_url_count,
    );
    let checked_at = Utc::now().to_rfc3339();
    let manifest = crate::fetch_directory_manifest(
        client,
        &source.work_group_url,
        DirectoryRole::WorkGroup,
        &checked_at,
    )
    .await?;
    if let Ok(paths) = crate::app_catalog_paths(app) {
        let _ = write_manifest(&paths, &manifest);
    }

    let mut meetings = manifest
        .children
        .iter()
        .filter(|child| child.kind == EntryKind::Directory)
        .filter(|child| child.name.starts_with(&source.meeting_series_prefix))
        .map(|child| child.name.clone())
        .collect::<Vec<_>>();
    let min_meeting_number = requested_min_meeting_number(request, query, &source.work_group_code);
    meetings.retain(|meeting| {
        meeting_number_from_slug(meeting)
            .map(|number| number >= min_meeting_number)
            .unwrap_or(true)
    });
    meetings.sort_by(|left, right| right.cmp(left));
    if request.search_window == "fast-recent" {
        meetings.truncate(36);
    } else if request.search_window == "from-meeting" {
        meetings.truncate(72);
    }

    let expected_file_name = format!("{}.zip", query.tdoc.key);
    let mut searched = searched_url_count + 1;

    for chunk in meetings.chunks(12) {
        check_cancelled(token)?;
        let progress = 40u8.saturating_add(((searched.min(80) as u8) / 4).min(15));
        emit_progress(
            app,
            job_id,
            "probing",
            format!("Probing {} candidate meetings...", chunk.len()),
            Some(progress),
            searched,
        );

        let mut tasks = JoinSet::new();
        for meeting_slug in chunk {
            let meeting_slug = meeting_slug.clone();
            let url = direct_probe_url(source, &meeting_slug, &query.tdoc);
            let client = client.clone();
            let expected_file_name = expected_file_name.clone();
            tasks.spawn(async move {
                let matched = probe_exact_file(&client, &url, &expected_file_name)
                    .await
                    .unwrap_or(false);
                (meeting_slug, url, matched)
            });
        }

        while let Some(joined) = tasks.join_next().await {
            searched += 1;
            check_cancelled(token)?;
            let Ok((meeting_slug, url, matched)) = joined else {
                continue;
            };
            if matched {
                tasks.abort_all();
                return Ok(Some((meeting_slug, url, searched)));
            }
        }
    }

    Ok(None)
}

async fn download_extract_open(
    app: &AppHandle,
    job_id: &str,
    token: &CancellationToken,
    request: &GppLookupRequest,
    client: &reqwest::Client,
    target: DownloadTarget,
    searched_url_count: usize,
) -> LookupResult<GppLookupComplete> {
    check_cancelled(token)?;
    let zip_path = zip_path_for_extract_dir(&target.extract_dir, &target.zip_file_name);

    let cache_status = match classify_local_download_cache(
        &target.extract_dir,
        &target.open_stem,
        &target.zip_file_name,
    )
    .map_err(LookupError::from)?
    {
        LocalDownloadCache::ExtractedDocument(open_path) => {
            let mut opened_path = None;
            if request.open_after_download {
                emit_progress(
                    app,
                    job_id,
                    "opening",
                    "Opening cached document...".to_string(),
                    Some(92),
                    searched_url_count,
                );
                if tauri_plugin_opener::open_path(&open_path, None::<&str>).is_ok() {
                    opened_path = Some(open_path.display().to_string());
                }
            }
            let cached_message = if opened_path.is_some() {
                format!("Opened cached {}.", target.open_stem)
            } else {
                format!("Cached {} is ready.", target.open_stem)
            };
            emit_progress(
                app,
                job_id,
                "complete",
                cached_message.clone(),
                Some(100),
                searched_url_count,
            );
            return Ok(GppLookupComplete {
                job_id: job_id.to_string(),
                query: request.query.clone(),
                source_url: target.source_url,
                zip_path: zip_path.display().to_string(),
                extracted_path: target.extract_dir.display().to_string(),
                opened_path,
                cache_status: "cached_document".to_string(),
                message: cached_message,
            });
        }
        LocalDownloadCache::ZipOnly => {
            emit_progress(
                app,
                job_id,
                "extracting",
                format!("Extracting cached {}...", target.zip_file_name),
                Some(72),
                searched_url_count,
            );
            "cached_zip"
        }
        LocalDownloadCache::Missing => {
            emit_progress(
                app,
                job_id,
                "downloading",
                format!("Downloading {}...", target.zip_file_name),
                Some(62),
                searched_url_count,
            );
            download_zip(client, &target.source_url, &zip_path).await?;
            "downloaded"
        }
    };

    check_cancelled(token)?;
    emit_progress(
        app,
        job_id,
        "extracting",
        format!("Extracting {}...", target.zip_file_name),
        Some(80),
        searched_url_count,
    );
    let relative_files = extract_zip(&zip_path, &target.extract_dir)?;

    check_cancelled(token)?;
    let mut opened_path = None;
    if request.open_after_download {
        emit_progress(
            app,
            job_id,
            "opening",
            "Opening extracted document...".to_string(),
            Some(92),
            searched_url_count,
        );
        let open_path = resolve_open_path(&target.extract_dir, &target.open_stem, &relative_files);
        if tauri_plugin_opener::open_path(&open_path, None::<&str>).is_ok() {
            opened_path = Some(open_path.display().to_string());
        }
    }

    emit_progress(
        app,
        job_id,
        "complete",
        target.message.clone(),
        Some(100),
        searched_url_count,
    );

    Ok(GppLookupComplete {
        job_id: job_id.to_string(),
        query: request.query.clone(),
        source_url: target.source_url,
        zip_path: zip_path.display().to_string(),
        extracted_path: target.extract_dir.display().to_string(),
        opened_path,
        cache_status: cache_status.to_string(),
        message: target.message,
    })
}

fn classify_local_download_cache(
    extract_dir: &Path,
    open_stem: &str,
    zip_file_name: &str,
) -> std::result::Result<LocalDownloadCache, String> {
    if let Some(open_path) = cached_open_path(extract_dir, open_stem, zip_file_name)? {
        return Ok(LocalDownloadCache::ExtractedDocument(open_path));
    }

    let zip_path = zip_path_for_extract_dir(extract_dir, zip_file_name);
    if has_cached_zip(&zip_path) {
        Ok(LocalDownloadCache::ZipOnly)
    } else {
        Ok(LocalDownloadCache::Missing)
    }
}

fn contribution_target_from_record(
    record: &FileRecord,
    workspace_root: &std::path::Path,
) -> LookupResult<DownloadTarget> {
    let tdoc_key = record
        .tdoc
        .as_ref()
        .map(|tdoc| tdoc.key.clone())
        .unwrap_or_else(|| record.file_name.trim_end_matches(".zip").to_string());
    let work_group = record
        .work_group_code
        .clone()
        .unwrap_or_else(|| "unknown-workgroup".to_string());
    let meeting = record
        .meeting_slug
        .clone()
        .unwrap_or_else(|| "unknown-meeting".to_string());

    Ok(DownloadTarget {
        source_url: record.canonical_url.clone(),
        zip_file_name: record.file_name.clone(),
        extract_dir: tdoc_extract_dir(workspace_root, &work_group, &meeting, &tdoc_key),
        open_stem: tdoc_key.clone(),
        message: format!("Downloaded and extracted {tdoc_key}."),
    })
}

fn contribution_target_from_direct_url(
    workspace_root: &std::path::Path,
    work_group: &str,
    meeting_slug: &str,
    tdoc_key: &str,
    url: &str,
) -> DownloadTarget {
    DownloadTarget {
        source_url: url.to_string(),
        zip_file_name: format!("{tdoc_key}.zip"),
        extract_dir: tdoc_extract_dir(workspace_root, work_group, meeting_slug, tdoc_key),
        open_stem: tdoc_key.to_string(),
        message: format!("Downloaded and extracted {tdoc_key}."),
    }
}

fn cache_direct_contribution_record(
    app: &AppHandle,
    source: &spectrumpilot_3gpp_core::normalize::TDocSource,
    meeting_slug: &str,
    tdoc: &TDocKey,
    url: &str,
) {
    let Ok(paths) = crate::app_catalog_paths(app) else {
        return;
    };
    let file_name = format!("{}.zip", tdoc.key);
    let parent_directory_url = url
        .rsplit_once('/')
        .map(|(parent, _)| format!("{parent}/"))
        .unwrap_or_else(|| source.work_group_url.clone());
    let record = FileRecord {
        schema_version: 1,
        record_type: "tdoc-file".to_string(),
        id: FileRecord::stable_id(url),
        canonical_url: url.to_string(),
        parent_directory_url,
        root: source.root.clone(),
        work_group_path: source.work_group_path.clone(),
        work_group_code: Some(source.work_group_code.clone()),
        meeting_id: Some(MeetingRecord::stable_id(
            &source.root,
            &source.work_group_path,
            meeting_slug,
        )),
        meeting_slug: Some(meeting_slug.to_string()),
        container_role: DirectoryRole::Docs,
        file_name,
        extension: Some("zip".to_string()),
        remote_modified_raw: None,
        size_raw: None,
        size_bytes: None,
        tdoc: Some(tdoc.clone()),
        classification: FileClassification {
            is_primary_tdoc: true,
            is_zip: true,
            is_ignored_artifact: false,
        },
    };
    let parent_directory_url = record.parent_directory_url.clone();
    let _ = write_file_records(&paths, std::slice::from_ref(&record));
    let _ = write_tdoc_shards_for_records(
        &paths,
        &source.work_group_code,
        meeting_slug,
        &parent_directory_url,
        &Utc::now().to_rfc3339(),
        &[record],
    );
}

fn resolve_indexed_contribution_record(
    paths: &CatalogPaths,
    workspace_root: &Path,
    tdoc: &TDocKey,
) -> LookupResult<Option<DownloadTarget>> {
    let Some(year) = tdoc.year_hint else {
        return Ok(None);
    };
    let Some(shard) =
        read_tdoc_index_shard(paths, &tdoc.prefix, year).map_err(|source| source.to_string())?
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

fn write_tdoc_shards_for_records(
    paths: &CatalogPaths,
    work_group_code: &str,
    meeting_slug: &str,
    docs_url: &str,
    checked_at: &str,
    records: &[FileRecord],
) -> std::result::Result<(), String> {
    let shard = TDocMeetingRecordShard::from_records(
        work_group_code,
        meeting_slug,
        docs_url,
        checked_at,
        records.to_vec(),
    );
    write_tdoc_meeting_shard(paths, &shard).map_err(|source| source.to_string())?;
    for index_shard in build_tdoc_index_shards(records) {
        merge_tdoc_index_shard(paths, &index_shard).map_err(|source| source.to_string())?;
    }
    Ok(())
}

async fn probe_exact_file(
    client: &reqwest::Client,
    url: &str,
    expected_file_name: &str,
) -> LookupResult<bool> {
    let response = client
        .head(url)
        .send()
        .await
        .map_err(|source| format!("failed to probe {url}: {source}"))?;
    Ok(is_successful_exact_probe(
        response.status().as_u16(),
        response.url().as_str(),
        expected_file_name,
    ))
}

async fn head_status(client: &reqwest::Client, url: &str) -> Result<u16, reqwest::Error> {
    let response = client.head(url).send().await?;
    Ok(response.status().as_u16())
}

pub fn is_successful_exact_probe(status: u16, url: &str, expected_file_name: &str) -> bool {
    if status != 200 {
        return false;
    }

    url.rsplit('/')
        .next()
        .is_some_and(|file_name| file_name.eq_ignore_ascii_case(expected_file_name))
}

fn normalize_meeting_hint(series_prefix: &str, hint: &str) -> String {
    let trimmed = hint.trim();
    if trimmed
        .to_ascii_uppercase()
        .starts_with(&series_prefix.to_ascii_uppercase())
    {
        trimmed.to_string()
    } else {
        format!("{series_prefix}_{trimmed}")
    }
}

fn requested_min_meeting_number(
    request: &GppLookupRequest,
    query: &ContributionQuery,
    work_group_code: &str,
) -> u32 {
    let explicit = request
        .meeting_hint
        .as_deref()
        .or(query.meeting_hint.as_deref())
        .or(query.start_meeting.as_deref())
        .and_then(meeting_number_from_slug);
    explicit
        .or_else(|| default_start_meeting(work_group_code).and_then(meeting_number_from_slug))
        .unwrap_or(0)
}

fn default_start_meeting(work_group_code: &str) -> Option<&'static str> {
    match work_group_code {
        "RAN" => Some("TSGR_100"),
        "RAN1" => Some("TSGR1_105"),
        "RAN2" => Some("TSGR2_120"),
        "RAN3" => Some("TSGR3_120"),
        "RAN4" => Some("TSGR4_100"),
        "RAN5" => Some("TSGR5_100"),
        _ => None,
    }
}

fn meeting_number_from_slug(slug: &str) -> Option<u32> {
    let rest = slug.split_once('_').map(|(_, rest)| rest).unwrap_or(slug);
    let digits = rest
        .chars()
        .take_while(|value| value.is_ascii_digit())
        .collect::<String>();
    digits.parse().ok()
}

fn spec_version_from_file(archive_stem: &str, file_name: &str) -> Option<String> {
    file_name
        .strip_prefix(&format!("{archive_stem}-"))?
        .strip_suffix(".zip")
        .map(str::to_string)
}

fn workspace_root(app: &AppHandle) -> LookupResult<PathBuf> {
    crate::app_workspace_root(app).map_err(LookupError::from)
}

fn check_cancelled(token: &CancellationToken) -> LookupResult<()> {
    if token.is_cancelled() {
        Err(LookupError::Cancelled)
    } else {
        Ok(())
    }
}

fn emit_progress(
    app: &AppHandle,
    job_id: &str,
    stage: impl Into<String>,
    message: impl Into<String>,
    progress: Option<u8>,
    searched_url_count: usize,
) {
    let _ = app.emit(
        "gpp-job-progress",
        GppLookupProgress {
            job_id: job_id.to_string(),
            stage: stage.into(),
            message: message.into(),
            progress,
            searched_url_count,
        },
    );
}

#[cfg(test)]
mod tests {
    use spectrumpilot_3gpp_core::catalog::CatalogPaths;
    use spectrumpilot_3gpp_core::model::{
        DirectoryRole, EntryKind, FileClassification, FileRecord, MeetingRecord, TDocKey,
    };
    use spectrumpilot_3gpp_core::query::{parse_gpp_query, GppQuery};
    use spectrumpilot_3gpp_core::specs::{
        archive_directory_url, archive_file_name, select_latest_spec_file,
    };

    use super::super::download::{download_zip, extract_zip, resolve_open_path, tdoc_extract_dir};
    use super::{
        classify_local_download_cache, is_successful_exact_probe, probe_exact_file,
        resolve_indexed_contribution_record, write_tdoc_shards_for_records, LocalDownloadCache,
    };

    #[test]
    fn exact_probe_requires_success_and_exact_file_name() {
        assert!(is_successful_exact_probe(
            200,
            "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip",
            "R2-2601401.zip"
        ));
        assert!(!is_successful_exact_probe(
            403,
            "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_132/Docs/R2-2601401.zip",
            "R2-2601401.zip"
        ));
        assert!(!is_successful_exact_probe(
            200,
            "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601402.zip",
            "R2-2601401.zip"
        ));
    }

    #[test]
    fn resolves_contribution_from_index_shard() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = CatalogPaths::new(temp.path().join("3gpp"));
        let workspace_root = temp.path().join("workspace");
        let record = ran2_record("R2-2601401", "TSGR2_133bis");
        write_tdoc_shards_for_records(
            &paths,
            "RAN2",
            "TSGR2_133bis",
            "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/",
            "2026-07-02T08:00:00Z",
            std::slice::from_ref(&record),
        )
        .expect("write shards");

        let target = resolve_indexed_contribution_record(
            &paths,
            &workspace_root,
            record.tdoc.as_ref().expect("tdoc"),
        )
        .expect("resolve")
        .expect("indexed target");

        assert_eq!(
            target.source_url,
            "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip"
        );
        assert_eq!(target.zip_file_name, "R2-2601401.zip");
        assert!(target
            .extract_dir
            .ends_with("3gpp/tdocs/RAN2/TSGR2_133bis/R2-2601401"));
    }

    #[test]
    fn indexed_resolution_returns_none_when_shard_is_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = CatalogPaths::new(temp.path().join("3gpp"));
        let workspace_root = temp.path().join("workspace");
        let record = ran2_record("R2-2601401", "TSGR2_133bis");

        assert!(resolve_indexed_contribution_record(
            &paths,
            &workspace_root,
            record.tdoc.as_ref().expect("tdoc"),
        )
        .expect("resolve")
        .is_none());
    }

    #[test]
    fn local_cache_classification_prefers_extracted_document_over_zip() {
        let temp = tempfile::tempdir().expect("tempdir");
        let extract_dir = temp.path().join("R2-2601401");
        std::fs::create_dir_all(&extract_dir).expect("extract dir");
        std::fs::write(extract_dir.join("R2-2601401.zip"), b"zip").expect("zip");
        std::fs::write(extract_dir.join("R2-2601401.docx"), b"docx").expect("docx");

        assert_eq!(
            classify_local_download_cache(&extract_dir, "R2-2601401", "R2-2601401.zip")
                .expect("classify"),
            LocalDownloadCache::ExtractedDocument(extract_dir.join("R2-2601401.docx"))
        );
    }

    #[test]
    fn local_cache_classification_uses_zip_when_document_is_not_extracted() {
        let temp = tempfile::tempdir().expect("tempdir");
        let extract_dir = temp.path().join("R2-2601401");
        std::fs::create_dir_all(&extract_dir).expect("extract dir");
        std::fs::write(extract_dir.join("R2-2601401.zip"), b"zip").expect("zip");

        assert_eq!(
            classify_local_download_cache(&extract_dir, "R2-2601401", "R2-2601401.zip")
                .expect("classify"),
            LocalDownloadCache::ZipOnly
        );
    }

    #[test]
    fn local_cache_classification_reports_missing_without_document_or_zip() {
        let temp = tempfile::tempdir().expect("tempdir");
        let extract_dir = temp.path().join("R2-2601401");

        assert_eq!(
            classify_local_download_cache(&extract_dir, "R2-2601401", "R2-2601401.zip")
                .expect("classify"),
            LocalDownloadCache::Missing
        );
    }

    #[tokio::test]
    #[ignore]
    async fn live_lookup_download_finds_known_ran2_tdoc() {
        let client = live_client();
        let workspace = tempfile::tempdir().expect("tempdir");
        let url = "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip";
        let extract_dir = tdoc_extract_dir(workspace.path(), "RAN2", "TSGR2_133bis", "R2-2601401");
        let zip_path = extract_dir.join("R2-2601401.zip");

        assert!(probe_exact_file(&client, url, "R2-2601401.zip")
            .await
            .expect("probe"));
        let size = download_zip(&client, url, &zip_path)
            .await
            .expect("download");
        let files = extract_zip(&zip_path, &extract_dir).expect("extract");
        let open_path = resolve_open_path(&extract_dir, "R2-2601401", &files);

        assert!(size > 0);
        assert!(!files.is_empty());
        assert!(open_path.exists());
    }

    #[tokio::test]
    #[ignore]
    async fn live_spec_lookup_finds_exact_38321_f10() {
        let client = live_client();
        let query = match parse_gpp_query("38.321 f10").expect("query") {
            GppQuery::Specification(query) => query,
            GppQuery::Contribution(_) => panic!("expected spec query"),
        };
        let archive_url = archive_directory_url(&query);
        let file_name = archive_file_name(&query, "f10");
        let url = format!("{archive_url}{file_name}");

        assert!(probe_exact_file(&client, &url, &file_name)
            .await
            .expect("probe"));
    }

    #[tokio::test]
    #[ignore]
    async fn live_spec_lookup_finds_latest_38321() {
        let client = live_client();
        let query = match parse_gpp_query("38.321").expect("query") {
            GppQuery::Specification(query) => query,
            GppQuery::Contribution(_) => panic!("expected spec query"),
        };
        let archive_url = archive_directory_url(&query);
        let html = client
            .get(&archive_url)
            .send()
            .await
            .expect("fetch listing")
            .error_for_status()
            .expect("status")
            .text()
            .await
            .expect("body");
        let manifest = spectrumpilot_3gpp_core::manifest::build_manifest_from_html(
            &archive_url,
            DirectoryRole::Unknown,
            "live-smoke",
            &html,
        )
        .expect("manifest");
        let files = manifest
            .children
            .iter()
            .filter(|child| child.kind == EntryKind::File)
            .map(|child| child.name.clone())
            .collect::<Vec<_>>();
        let latest = select_latest_spec_file(&query.archive_stem, None, &files).expect("latest");

        assert!(latest.starts_with("38321-"));
        assert!(latest.ends_with(".zip"));
    }

    fn live_client() -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("SpectrumPilot/0.1 live smoke test")
            .build()
            .expect("client")
    }

    fn ran2_record(tdoc: &str, meeting_slug: &str) -> FileRecord {
        let url =
            format!("https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/{meeting_slug}/Docs/{tdoc}.zip");
        FileRecord {
            schema_version: 1,
            record_type: "tdoc-file".to_string(),
            id: FileRecord::stable_id(&url),
            canonical_url: url,
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
}
