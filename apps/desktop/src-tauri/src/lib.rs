use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use include_dir::{include_dir, Dir};
use serde::{Deserialize, Serialize};
use spectrumpilot_3gpp_core::catalog::{
    manifest_path_for_url, merge_tdoc_index_shard, read_file_records, summarize_catalog,
    write_file_records, write_json_atomic, write_manifest, write_tdoc_meeting_shard, CatalogPaths,
    CatalogSummary,
};
use spectrumpilot_3gpp_core::index::{build_tdoc_index_shards, TDocLookupIndex};
use spectrumpilot_3gpp_core::manifest::{
    build_manifest_from_html, file_records_from_docs_manifest,
};
use spectrumpilot_3gpp_core::model::{
    DirectoryChild, DirectoryManifest, DirectoryRole, EntryKind, FileRecord, TDocMeetingRecordShard,
};
use spectrumpilot_3gpp_core::normalize::{
    infer_tdoc_sources, normalize_tdoc_query, parse_meeting_slug,
};
use spectrumpilot_3gpp_core::resolver::resolve_tdoc;
use spectrumpilot_3gpp_core::tdoc::source_for_tdoc_prefix;
use tauri::{AppHandle, Manager};
use tokio::task::JoinSet;
use url::Url;

mod gpp;

static BUNDLED_CATALOG_SEED: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/resources/3gpp/catalog_seed");

const GPP_BACKGROUND_REFRESH_INTERVAL_MINUTES: u64 = 60;
const GPP_BACKGROUND_REFRESH_START_DELAY_SECS: u64 = 15;
const GPP_BACKGROUND_REFRESH_REQUEST_DELAY_SECS: u64 = 2;
const GPP_BACKGROUND_REFRESH_MAX_MEETINGS_PER_WORKGROUP: usize = 8;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimePaths {
    app_storage_dir: String,
    config_dir: String,
    metadata_dir: String,
    internal_cache_dir: String,
    logs_dir: String,
    workspace_root: String,
    three_gpp_workspace_dir: String,
    three_gpp_internal_cache_dir: String,
    three_gpp_catalog_dir: String,
    // Transitional aliases kept for the frontend while the Settings UI moves to product terms.
    app_data_dir: String,
    app_cache_dir: String,
    app_log_dir: String,
    three_gpp_cache_dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeLayout {
    app_storage_dir: PathBuf,
    config_dir: PathBuf,
    metadata_dir: PathBuf,
    internal_cache_dir: PathBuf,
    logs_dir: PathBuf,
    default_workspace_root: PathBuf,
    three_gpp_workspace_dir_for_default: PathBuf,
    three_gpp_metadata_dir: PathBuf,
    three_gpp_internal_cache_dir: PathBuf,
    three_gpp_catalog_dir: PathBuf,
    legacy_app_data_dir: PathBuf,
    legacy_app_cache_dir: PathBuf,
    legacy_app_log_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    record_type: String,
    workspace_root: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GppCatalogStatus {
    catalog_root: String,
    manifest_count: usize,
    record_count: usize,
    index_count: usize,
    seed_version: String,
    seed_generated_at: Option<String>,
    seed_scope: String,
    background_refresh_enabled: bool,
    background_refresh_interval_minutes: u64,
    background_refresh_tracked_roots: usize,
    background_refresh_meeting_window: usize,
    background_refresh_state: String,
    background_refresh_last_started_at: Option<String>,
    background_refresh_last_completed_at: Option<String>,
    background_refresh_last_error: Option<String>,
    background_refresh_last_refreshed_manifest_count: usize,
    background_refresh_log_path: String,
    last_checked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogSeedMetadata {
    record_type: String,
    seed_version: String,
    seed_generated_at: Option<String>,
    seed_scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackgroundRefreshStatus {
    record_type: String,
    state: String,
    last_started_at: Option<String>,
    last_completed_at: Option<String>,
    last_error: Option<String>,
    last_refreshed_manifest_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackgroundRefreshSettings {
    record_type: String,
    enabled: bool,
    interval_minutes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GppBootstrapReport {
    fetched_url_count: usize,
    manifest_count: usize,
    child_entry_count: usize,
    target_roots: Vec<String>,
    checked_at: String,
    catalog_root: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GppTdocSearchRequest {
    query: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GppTdocFileResult {
    tdoc: String,
    file_name: String,
    url: String,
    source: String,
    root: String,
    work_group: Option<String>,
    meeting: Option<String>,
    remote_modified_raw: Option<String>,
    size_raw: Option<String>,
    size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GppTdocSearchReport {
    query: String,
    normalized_query: String,
    source: String,
    searched_url_count: usize,
    results: Vec<GppTdocFileResult>,
    message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GppTdocDownloadRequest {
    url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GppTdocDownloadReport {
    file_name: String,
    source_url: String,
    saved_path: String,
    size_bytes: u64,
}

#[tauri::command]
fn app_status() -> String {
    "SpectrumPilot desktop shell is ready".to_string()
}

fn build_runtime_layout(
    home_dir: &Path,
    legacy_app_data_dir: &Path,
    legacy_app_cache_dir: &Path,
    legacy_app_log_dir: &Path,
) -> RuntimeLayout {
    let app_storage_dir = product_storage_root(legacy_app_data_dir);
    let config_dir = app_storage_dir.join("config");
    let metadata_dir = app_storage_dir.join("metadata");
    let internal_cache_dir = app_storage_dir.join("cache");
    let logs_dir = app_storage_dir.join("logs");
    let default_workspace_root = home_dir.join("SpectrumPilotWorkspace");
    let three_gpp_workspace_dir_for_default = default_workspace_root.join("3gpp");
    let three_gpp_metadata_dir = metadata_dir.join("3gpp");
    let three_gpp_internal_cache_dir = internal_cache_dir.join("3gpp");
    let catalog_paths = CatalogPaths::new(&three_gpp_metadata_dir);

    RuntimeLayout {
        app_storage_dir,
        config_dir,
        metadata_dir,
        internal_cache_dir,
        logs_dir,
        default_workspace_root,
        three_gpp_workspace_dir_for_default,
        three_gpp_metadata_dir,
        three_gpp_internal_cache_dir,
        three_gpp_catalog_dir: catalog_paths.root().to_path_buf(),
        legacy_app_data_dir: legacy_app_data_dir.to_path_buf(),
        legacy_app_cache_dir: legacy_app_cache_dir.to_path_buf(),
        legacy_app_log_dir: legacy_app_log_dir.to_path_buf(),
    }
}

fn product_storage_root(legacy_app_data_dir: &Path) -> PathBuf {
    if legacy_app_data_dir
        .file_name()
        .and_then(|value| value.to_str())
        == Some("SpectrumPilot")
    {
        return legacy_app_data_dir.to_path_buf();
    }

    legacy_app_data_dir
        .parent()
        .map(|parent| parent.join("SpectrumPilot"))
        .unwrap_or_else(|| legacy_app_data_dir.join("SpectrumPilot"))
}

fn build_runtime_paths(layout: &RuntimeLayout, settings: &AppSettings) -> RuntimePaths {
    let workspace_root = PathBuf::from(&settings.workspace_root);
    let three_gpp_workspace_dir = workspace_root.join("3gpp");
    RuntimePaths {
        app_storage_dir: layout.app_storage_dir.display().to_string(),
        config_dir: layout.config_dir.display().to_string(),
        metadata_dir: layout.metadata_dir.display().to_string(),
        internal_cache_dir: layout.internal_cache_dir.display().to_string(),
        logs_dir: layout.logs_dir.display().to_string(),
        workspace_root: workspace_root.display().to_string(),
        three_gpp_workspace_dir: three_gpp_workspace_dir.display().to_string(),
        three_gpp_internal_cache_dir: layout.three_gpp_internal_cache_dir.display().to_string(),
        three_gpp_catalog_dir: layout.three_gpp_catalog_dir.display().to_string(),
        app_data_dir: layout.app_storage_dir.display().to_string(),
        app_cache_dir: layout.internal_cache_dir.display().to_string(),
        app_log_dir: layout.logs_dir.display().to_string(),
        three_gpp_cache_dir: layout.three_gpp_internal_cache_dir.display().to_string(),
    }
}

fn ensure_dir(path: &str) -> std::result::Result<(), String> {
    fs::create_dir_all(path).map_err(|source| format!("failed to create {path}: {source}"))
}

fn ensure_runtime_layout_dirs(layout: &RuntimeLayout) -> std::result::Result<(), String> {
    for dir in [
        &layout.app_storage_dir,
        &layout.config_dir,
        &layout.metadata_dir,
        &layout.internal_cache_dir,
        &layout.logs_dir,
        &layout.three_gpp_metadata_dir,
        &layout.three_gpp_internal_cache_dir,
    ] {
        fs::create_dir_all(dir)
            .map_err(|source| format!("failed to create {}: {source}", dir.display()))?;
    }
    Ok(())
}

fn default_app_settings(layout: &RuntimeLayout) -> AppSettings {
    AppSettings {
        record_type: "spectrumpilot-settings".to_string(),
        workspace_root: layout.default_workspace_root.display().to_string(),
    }
}

fn app_settings_path(layout: &RuntimeLayout) -> PathBuf {
    layout.config_dir.join("settings.json")
}

fn read_app_settings(layout: &RuntimeLayout) -> std::result::Result<AppSettings, String> {
    let path = app_settings_path(layout);
    if !path.exists() {
        return Ok(default_app_settings(layout));
    }
    let body =
        fs::read(&path).map_err(|source| format!("failed to read {}: {source}", path.display()))?;
    let mut settings: AppSettings = serde_json::from_slice(&body)
        .map_err(|source| format!("failed to parse {}: {source}", path.display()))?;
    if settings.workspace_root.trim().is_empty() {
        settings.workspace_root = layout.default_workspace_root.display().to_string();
    }
    Ok(settings)
}

fn read_or_create_app_settings(layout: &RuntimeLayout) -> std::result::Result<AppSettings, String> {
    let path = app_settings_path(layout);
    let settings = read_app_settings(layout)?;
    if !path.exists() {
        write_app_settings(layout, &settings)?;
    }
    Ok(settings)
}

fn write_app_settings(
    layout: &RuntimeLayout,
    settings: &AppSettings,
) -> std::result::Result<(), String> {
    fs::create_dir_all(&layout.config_dir)
        .map_err(|source| format!("failed to create {}: {source}", layout.config_dir.display()))?;
    write_json_atomic(&app_settings_path(layout), settings).map_err(|source| source.to_string())
}

fn migrate_runtime_layout(layout: &RuntimeLayout) -> std::result::Result<(), String> {
    ensure_runtime_layout_dirs(layout)?;

    let legacy_catalog = layout.legacy_app_cache_dir.join("3gpp").join("catalog");
    copy_dir_if_target_missing(&legacy_catalog, &layout.three_gpp_catalog_dir)?;

    copy_file_if_target_missing(
        &layout
            .legacy_app_data_dir
            .join("config")
            .join("3gpp-settings.json"),
        &background_refresh_settings_path(&layout.config_dir),
    )?;
    copy_file_if_target_missing(
        &layout.legacy_app_log_dir.join("3gpp-refresh.log"),
        &background_refresh_log_path(&layout.logs_dir),
    )?;

    Ok(())
}

fn copy_dir_if_target_missing(source: &Path, target: &Path) -> std::result::Result<(), String> {
    if !source.exists() || target.exists() {
        return Ok(());
    }
    copy_dir_recursive(source, target)
}

fn copy_dir_recursive(source: &Path, target: &Path) -> std::result::Result<(), String> {
    fs::create_dir_all(target)
        .map_err(|source_error| format!("failed to create {}: {source_error}", target.display()))?;
    for entry in fs::read_dir(source)
        .map_err(|source_error| format!("failed to read {}: {source_error}", source.display()))?
    {
        let entry = entry.map_err(|source_error| {
            format!("failed to read {}: {source_error}", source.display())
        })?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else if !target_path.exists() {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).map_err(|source_error| {
                    format!("failed to create {}: {source_error}", parent.display())
                })?;
            }
            fs::copy(&source_path, &target_path).map_err(|source_error| {
                format!(
                    "failed to copy {} to {}: {source_error}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn copy_file_if_target_missing(source: &Path, target: &Path) -> std::result::Result<(), String> {
    if !source.exists() || target.exists() {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|source_error| {
            format!("failed to create {}: {source_error}", parent.display())
        })?;
    }
    fs::copy(source, target).map_err(|source_error| {
        format!(
            "failed to copy {} to {}: {source_error}",
            source.display(),
            target.display()
        )
    })?;
    Ok(())
}

fn app_catalog_paths(app: &AppHandle) -> std::result::Result<CatalogPaths, String> {
    let layout = app_runtime_layout(app)?;
    let paths = CatalogPaths::new(&layout.three_gpp_metadata_dir);
    paths.ensure_dirs().map_err(|source| source.to_string())?;
    install_bundled_catalog_seed_if_empty(&paths)?;
    Ok(paths)
}

fn app_runtime_layout(app: &AppHandle) -> std::result::Result<RuntimeLayout, String> {
    let home_dir = app
        .path()
        .home_dir()
        .map_err(|source| format!("failed to resolve home directory: {source}"))?;
    let legacy_app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|source| format!("failed to resolve app data directory: {source}"))?;
    let legacy_app_cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|source| format!("failed to resolve app cache directory: {source}"))?;
    let legacy_app_log_dir = app
        .path()
        .app_log_dir()
        .map_err(|source| format!("failed to resolve app log directory: {source}"))?;
    let layout = build_runtime_layout(
        &home_dir,
        &legacy_app_data_dir,
        &legacy_app_cache_dir,
        &legacy_app_log_dir,
    );
    migrate_runtime_layout(&layout)?;
    Ok(layout)
}

fn app_background_refresh_settings(
    app: &AppHandle,
) -> std::result::Result<BackgroundRefreshSettings, String> {
    let layout = app_runtime_layout(app)?;
    read_background_refresh_settings(&layout.config_dir)
}

fn app_workspace_root(app: &AppHandle) -> std::result::Result<PathBuf, String> {
    let layout = app_runtime_layout(app)?;
    let settings = read_or_create_app_settings(&layout)?;
    Ok(PathBuf::from(settings.workspace_root))
}

fn install_bundled_catalog_seed_if_empty(
    paths: &CatalogPaths,
) -> std::result::Result<usize, String> {
    paths.ensure_dirs().map_err(|source| source.to_string())?;
    let mut installed = 0;
    installed += install_seed_metadata_if_missing(paths.root())?;
    installed += install_seed_subtree_if_empty("manifests", &paths.manifests_dir())?;
    installed += install_seed_subtree_if_empty("records", &paths.records_dir())?;
    installed += install_seed_subtree_if_empty("indexes", &paths.indexes_dir())?;
    Ok(installed)
}

fn install_seed_metadata_if_missing(catalog_root: &Path) -> std::result::Result<usize, String> {
    fs::create_dir_all(catalog_root)
        .map_err(|source| format!("failed to create {}: {source}", catalog_root.display()))?;
    let target = catalog_root.join("seed.json");
    if target.exists() {
        return Ok(0);
    }
    let Some(seed_file) = BUNDLED_CATALOG_SEED.get_file("seed.json") else {
        return Ok(0);
    };
    fs::write(&target, seed_file.contents()).map_err(|source| {
        format!(
            "failed to install bundled 3GPP seed metadata {}: {source}",
            target.display()
        )
    })?;
    Ok(1)
}

fn install_seed_subtree_if_empty(
    seed_subdir: &str,
    target_dir: &Path,
) -> std::result::Result<usize, String> {
    fs::create_dir_all(target_dir)
        .map_err(|source| format!("failed to create {}: {source}", target_dir.display()))?;
    if count_json_files_recursive(target_dir)? > 0 {
        return Ok(0);
    }
    let Some(seed_dir) = BUNDLED_CATALOG_SEED.get_dir(seed_subdir) else {
        return Ok(0);
    };
    copy_seed_dir_json_files(seed_dir, seed_dir.path(), target_dir)
}

fn count_json_files_recursive(dir: &Path) -> std::result::Result<usize, String> {
    if !dir.exists() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in
        fs::read_dir(dir).map_err(|source| format!("failed to read {}: {source}", dir.display()))?
    {
        let entry =
            entry.map_err(|source| format!("failed to read {}: {source}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            count += count_json_files_recursive(&path)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("json") {
            count += 1;
        }
    }
    Ok(count)
}

fn copy_seed_dir_json_files(
    seed_dir: &Dir<'_>,
    seed_root: &Path,
    target_dir: &Path,
) -> std::result::Result<usize, String> {
    let mut installed = 0;
    for file in seed_dir.files() {
        if file.path().extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let relative_path = file.path().strip_prefix(seed_root).unwrap_or(file.path());
        let target = target_dir.join(relative_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|source| format!("failed to create {}: {source}", parent.display()))?;
        }
        fs::write(&target, file.contents()).map_err(|source| {
            format!(
                "failed to install bundled 3GPP seed {}: {source}",
                target.display()
            )
        })?;
        installed += 1;
    }
    for dir in seed_dir.dirs() {
        installed += copy_seed_dir_json_files(dir, seed_root, target_dir)?;
    }
    Ok(installed)
}

fn to_catalog_status(
    paths: &CatalogPaths,
    summary: CatalogSummary,
    refresh_settings: &BackgroundRefreshSettings,
    refresh_log_path: &Path,
) -> std::result::Result<GppCatalogStatus, String> {
    let seed_metadata = read_catalog_seed_metadata(paths)?;
    let refresh_status =
        read_background_refresh_status(paths)?.unwrap_or_else(default_background_refresh_status);
    let refresh_state = if refresh_settings.enabled {
        refresh_status.state
    } else {
        "disabled".to_string()
    };
    Ok(GppCatalogStatus {
        catalog_root: paths.root().display().to_string(),
        manifest_count: summary.manifest_count,
        record_count: summary.record_count,
        index_count: summary.index_count,
        seed_version: seed_metadata.seed_version,
        seed_generated_at: seed_metadata.seed_generated_at,
        seed_scope: seed_metadata.seed_scope,
        background_refresh_enabled: refresh_settings.enabled,
        background_refresh_interval_minutes: refresh_settings.interval_minutes,
        background_refresh_tracked_roots: background_refresh_targets().len(),
        background_refresh_meeting_window: GPP_BACKGROUND_REFRESH_MAX_MEETINGS_PER_WORKGROUP,
        background_refresh_state: refresh_state,
        background_refresh_last_started_at: refresh_status.last_started_at,
        background_refresh_last_completed_at: refresh_status.last_completed_at,
        background_refresh_last_error: refresh_status.last_error,
        background_refresh_last_refreshed_manifest_count: refresh_status
            .last_refreshed_manifest_count,
        background_refresh_log_path: refresh_log_path.display().to_string(),
        last_checked_at: summary.last_checked_at,
    })
}

fn read_catalog_seed_metadata(
    paths: &CatalogPaths,
) -> std::result::Result<CatalogSeedMetadata, String> {
    let runtime_path = paths.root().join("seed.json");
    if runtime_path.exists() {
        let body = fs::read(&runtime_path)
            .map_err(|source| format!("failed to read {}: {source}", runtime_path.display()))?;
        return serde_json::from_slice(&body)
            .map_err(|source| format!("failed to parse {}: {source}", runtime_path.display()));
    }

    let Some(seed_file) = BUNDLED_CATALOG_SEED.get_file("seed.json") else {
        return Ok(CatalogSeedMetadata {
            record_type: "3gpp-catalog-seed".to_string(),
            seed_version: "unversioned-seed".to_string(),
            seed_generated_at: None,
            seed_scope: "Bundled 3GPP catalog seed".to_string(),
        });
    };
    serde_json::from_slice(seed_file.contents())
        .map_err(|source| format!("failed to parse bundled 3GPP seed metadata: {source}"))
}

fn background_refresh_status_path(paths: &CatalogPaths) -> PathBuf {
    paths.root().join("background-refresh.json")
}

fn background_refresh_settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join("3gpp-settings.json")
}

fn background_refresh_log_path(app_log_dir: &Path) -> PathBuf {
    app_log_dir.join("3gpp-refresh.log")
}

fn default_background_refresh_settings() -> BackgroundRefreshSettings {
    BackgroundRefreshSettings {
        record_type: "3gpp-background-refresh-settings".to_string(),
        enabled: true,
        interval_minutes: GPP_BACKGROUND_REFRESH_INTERVAL_MINUTES,
    }
}

fn read_background_refresh_settings(
    config_dir: &Path,
) -> std::result::Result<BackgroundRefreshSettings, String> {
    let path = background_refresh_settings_path(config_dir);
    if !path.exists() {
        return Ok(default_background_refresh_settings());
    }
    let body =
        fs::read(&path).map_err(|source| format!("failed to read {}: {source}", path.display()))?;
    let mut settings: BackgroundRefreshSettings = serde_json::from_slice(&body)
        .map_err(|source| format!("failed to parse {}: {source}", path.display()))?;
    if settings.interval_minutes == 0 {
        settings.interval_minutes = GPP_BACKGROUND_REFRESH_INTERVAL_MINUTES;
    }
    Ok(settings)
}

fn write_background_refresh_settings(
    config_dir: &Path,
    settings: &BackgroundRefreshSettings,
) -> std::result::Result<(), String> {
    let path = background_refresh_settings_path(config_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|source| format!("failed to create {}: {source}", parent.display()))?;
    }
    write_json_atomic(&path, settings).map_err(|source| source.to_string())
}

fn append_background_refresh_log(
    app_log_dir: &Path,
    message: &str,
) -> std::result::Result<(), String> {
    fs::create_dir_all(app_log_dir)
        .map_err(|source| format!("failed to create {}: {source}", app_log_dir.display()))?;
    let path = background_refresh_log_path(app_log_dir);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|source| format!("failed to open {}: {source}", path.display()))?;
    writeln!(file, "{} {message}", Utc::now().to_rfc3339())
        .map_err(|source| format!("failed to write {}: {source}", path.display()))
}

fn default_background_refresh_status() -> BackgroundRefreshStatus {
    BackgroundRefreshStatus {
        record_type: "3gpp-background-refresh-status".to_string(),
        state: "not_started".to_string(),
        last_started_at: None,
        last_completed_at: None,
        last_error: None,
        last_refreshed_manifest_count: 0,
    }
}

fn read_background_refresh_status(
    paths: &CatalogPaths,
) -> std::result::Result<Option<BackgroundRefreshStatus>, String> {
    let path = background_refresh_status_path(paths);
    if !path.exists() {
        return Ok(None);
    }
    let body =
        fs::read(&path).map_err(|source| format!("failed to read {}: {source}", path.display()))?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|source| format!("failed to parse {}: {source}", path.display()))
}

fn write_background_refresh_status(
    paths: &CatalogPaths,
    status: &BackgroundRefreshStatus,
) -> std::result::Result<(), String> {
    paths.ensure_dirs().map_err(|source| source.to_string())?;
    write_json_atomic(&background_refresh_status_path(paths), status)
        .map_err(|source| source.to_string())
}

fn bootstrap_targets() -> Vec<&'static str> {
    vec![
        "https://www.3gpp.org/ftp/",
        "https://www.3gpp.org/ftp/tsg_cn/",
        "https://www.3gpp.org/ftp/tsg_ct/",
        "https://www.3gpp.org/ftp/tsg_geran/",
        "https://www.3gpp.org/ftp/tsg_ran/",
        "https://www.3gpp.org/ftp/tsg_sa/",
        "https://www.3gpp.org/ftp/tsg_t/",
    ]
}

fn background_refresh_targets() -> Vec<&'static str> {
    vec![
        "https://www.3gpp.org/ftp/tsg_cn/",
        "https://www.3gpp.org/ftp/tsg_ct/",
        "https://www.3gpp.org/ftp/tsg_geran/",
        "https://www.3gpp.org/ftp/tsg_ran/",
        "https://www.3gpp.org/ftp/tsg_sa/",
        "https://www.3gpp.org/ftp/tsg_t/",
    ]
}

fn likely_meeting_filter(tdoc_query: &str) -> Option<impl Fn(&str) -> bool> {
    let tdoc = normalize_tdoc_query(tdoc_query)?;
    let series_prefix = match tdoc.prefix.as_str() {
        "R1" => "TSGR1",
        "R2" => "TSGR2",
        "R3" => "TSGR3",
        "R4" => "TSGR4",
        "R5" => "TSGR5",
        "S1" => "TSGS1",
        "S2" => "TSGS2",
        "S3" => "TSGS3",
        "S4" => "TSGS4",
        "S5" => "TSGS5",
        "S6" => "TSGS6",
        "C1" => "TSGC1",
        "C3" => "TSGC3",
        "C4" => "TSGC4",
        "C6" => "TSGC6",
        _ => return None,
    }
    .to_string();
    let min_meeting_number = tdoc
        .year_hint
        .map(|year| match year {
            2026..=2099 => 120,
            2020..=2025 => 100,
            2015..=2019 => 80,
            2010..=2014 => 60,
            _ => 0,
        })
        .unwrap_or(0);

    Some(move |meeting_slug: &str| {
        if !meeting_slug.starts_with(&series_prefix) {
            return false;
        }

        meeting_number_from_slug(meeting_slug)
            .map(|number| number >= min_meeting_number)
            .unwrap_or(true)
    })
}

fn meeting_number_from_slug(slug: &str) -> Option<u32> {
    let (_, rest) = slug.split_once('_')?;
    let digits: String = rest
        .chars()
        .take_while(|value| value.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

fn download_file_name_for_url(url: &str) -> Option<String> {
    Url::parse(url)
        .ok()?
        .path_segments()?
        .last()
        .filter(|name| !name.is_empty())
        .map(str::to_string)
}

fn download_target_path(workspace_root: &Path, url: &str) -> PathBuf {
    workspace_root
        .join("3gpp")
        .join("downloads")
        .join(download_file_name_for_url(url).unwrap_or_else(|| "download.zip".to_string()))
}

fn file_record_to_result(record: &FileRecord, source: &str) -> GppTdocFileResult {
    GppTdocFileResult {
        tdoc: record
            .tdoc
            .as_ref()
            .map(|tdoc| tdoc.key.clone())
            .unwrap_or_else(|| record.file_name.trim_end_matches(".zip").to_string()),
        file_name: record.file_name.clone(),
        url: record.canonical_url.clone(),
        source: source.to_string(),
        root: record.root.clone(),
        work_group: record.work_group_code.clone(),
        meeting: record.meeting_slug.clone(),
        remote_modified_raw: record.remote_modified_raw.clone(),
        size_raw: record.size_raw.clone(),
        size_bytes: record.size_bytes,
    }
}

async fn fetch_directory_manifest(
    client: &reqwest::Client,
    url: &str,
    role: DirectoryRole,
    checked_at: &str,
) -> std::result::Result<spectrumpilot_3gpp_core::model::DirectoryManifest, String> {
    let html = client
        .get(url)
        .send()
        .await
        .map_err(|source| format!("failed to fetch {url}: {source}"))?
        .error_for_status()
        .map_err(|source| format!("failed to fetch {url}: {source}"))?
        .text()
        .await
        .map_err(|source| format!("failed to read {url}: {source}"))?;

    build_manifest_from_html(url, role, checked_at, &html)
        .map_err(|source| format!("failed to parse {url}: {source}"))
}

fn read_manifest_by_url(
    paths: &CatalogPaths,
    url: &str,
) -> std::result::Result<Option<DirectoryManifest>, String> {
    let path = manifest_path_for_url(paths, url);
    if !path.exists() {
        return Ok(None);
    }
    let body =
        fs::read(&path).map_err(|source| format!("failed to read {}: {source}", path.display()))?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|source| format!("failed to parse {}: {source}", path.display()))
}

fn should_refresh_manifest_children(
    previous: Option<&DirectoryManifest>,
    current: &DirectoryManifest,
) -> bool {
    previous
        .map(|previous| previous.child_fingerprint != current.child_fingerprint)
        .unwrap_or(true)
}

fn directory_children(manifest: &DirectoryManifest) -> impl Iterator<Item = &DirectoryChild> {
    manifest
        .children
        .iter()
        .filter(|child| child.kind == EntryKind::Directory)
}

fn recent_meeting_children(manifest: &DirectoryManifest, limit: usize) -> Vec<DirectoryChild> {
    let mut meetings = directory_children(manifest)
        .filter_map(|child| {
            let parsed = parse_meeting_slug(&child.name);
            parsed
                .number
                .map(|number| (number, child.name.clone(), child))
        })
        .collect::<Vec<_>>();
    meetings.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));
    meetings
        .into_iter()
        .take(limit)
        .map(|(_, _, child)| child.clone())
        .collect()
}

fn spawn_background_gpp_catalog_refresh(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(GPP_BACKGROUND_REFRESH_START_DELAY_SECS)).await;

        loop {
            let sleep_minutes = match app_background_refresh_settings(&app) {
                Ok(settings) => {
                    if settings.enabled {
                        if let Err(source) = refresh_gpp_root_manifests_for_app(&app).await {
                            eprintln!("SpectrumPilot 3GPP background refresh failed: {source}");
                        }
                    } else if let Err(source) = skip_disabled_background_refresh_for_app(&app) {
                        eprintln!("SpectrumPilot 3GPP background refresh skip failed: {source}");
                    }
                    settings.interval_minutes.max(1)
                }
                Err(source) => {
                    eprintln!("SpectrumPilot failed to read 3GPP refresh settings: {source}");
                    GPP_BACKGROUND_REFRESH_INTERVAL_MINUTES
                }
            };
            tokio::time::sleep(Duration::from_secs(sleep_minutes * 60)).await;
        }
    });
}

fn skip_disabled_background_refresh_for_app(app: &AppHandle) -> std::result::Result<(), String> {
    let paths = app_catalog_paths(app)?;
    let log_dir = app_runtime_layout(app)?.logs_dir;
    write_background_refresh_status(
        &paths,
        &BackgroundRefreshStatus {
            record_type: "3gpp-background-refresh-status".to_string(),
            state: "disabled".to_string(),
            last_started_at: None,
            last_completed_at: Some(Utc::now().to_rfc3339()),
            last_error: None,
            last_refreshed_manifest_count: 0,
        },
    )?;
    append_background_refresh_log(&log_dir, "skipped scheduled refresh; reason=disabled")
}

async fn refresh_gpp_root_manifests_for_app(app: &AppHandle) -> std::result::Result<usize, String> {
    let paths = app_catalog_paths(app)?;
    let log_dir = app_runtime_layout(app)?.logs_dir;
    let started_at = Utc::now().to_rfc3339();
    let _ = append_background_refresh_log(
        &log_dir,
        &format!(
            "started scheduled refresh; interval_minutes={}; tracked_roots={}; meeting_window={}",
            GPP_BACKGROUND_REFRESH_INTERVAL_MINUTES,
            background_refresh_targets().len(),
            GPP_BACKGROUND_REFRESH_MAX_MEETINGS_PER_WORKGROUP
        ),
    );
    write_background_refresh_status(
        &paths,
        &BackgroundRefreshStatus {
            record_type: "3gpp-background-refresh-status".to_string(),
            state: "running".to_string(),
            last_started_at: Some(started_at.clone()),
            last_completed_at: None,
            last_error: None,
            last_refreshed_manifest_count: 0,
        },
    )?;

    let result = match reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("SpectrumPilot/0.1 3GPP background refresh")
        .build()
    {
        Ok(client) => refresh_gpp_root_manifests(&paths, &client, &started_at, &log_dir).await,
        Err(source) => Err(format!("failed to build HTTP client: {source}")),
    };

    match result {
        Ok(refreshed_manifest_count) => {
            let _ = append_background_refresh_log(
                &log_dir,
                &format!(
                    "succeeded scheduled refresh; refreshed_manifest_count={refreshed_manifest_count}"
                ),
            );
            write_background_refresh_status(
                &paths,
                &BackgroundRefreshStatus {
                    record_type: "3gpp-background-refresh-status".to_string(),
                    state: "succeeded".to_string(),
                    last_started_at: Some(started_at),
                    last_completed_at: Some(Utc::now().to_rfc3339()),
                    last_error: None,
                    last_refreshed_manifest_count: refreshed_manifest_count,
                },
            )?;
            Ok(refreshed_manifest_count)
        }
        Err(source) => {
            let _ = append_background_refresh_log(
                &log_dir,
                &format!("failed scheduled refresh; error={source}"),
            );
            let _ = write_background_refresh_status(
                &paths,
                &BackgroundRefreshStatus {
                    record_type: "3gpp-background-refresh-status".to_string(),
                    state: "failed".to_string(),
                    last_started_at: Some(started_at),
                    last_completed_at: None,
                    last_error: Some(source.clone()),
                    last_refreshed_manifest_count: 0,
                },
            );
            Err(source)
        }
    }
}

async fn refresh_gpp_root_manifests(
    paths: &CatalogPaths,
    client: &reqwest::Client,
    checked_at: &str,
    log_dir: &Path,
) -> std::result::Result<usize, String> {
    let mut refreshed = 0;
    let mut errors = Vec::new();

    for (index, target) in background_refresh_targets().into_iter().enumerate() {
        if index > 0 {
            tokio::time::sleep(Duration::from_secs(
                GPP_BACKGROUND_REFRESH_REQUEST_DELAY_SECS,
            ))
            .await;
        }

        match fetch_directory_manifest(client, target, DirectoryRole::Root, checked_at).await {
            Ok(manifest) => {
                let previous = read_manifest_by_url(paths, target)?;
                let should_descend = should_refresh_manifest_children(previous.as_ref(), &manifest);
                write_manifest(paths, &manifest).map_err(|source| source.to_string())?;
                let _ = append_background_refresh_log(
                    log_dir,
                    &format!(
                        "fetched root manifest; url={target}; changed={should_descend}; children={}",
                        manifest.children.len()
                    ),
                );
                refreshed += 1;
                if should_descend {
                    refreshed += refresh_changed_work_group_manifests(
                        paths, client, checked_at, &manifest, log_dir,
                    )
                    .await?;
                }
            }
            Err(source) => {
                let _ = append_background_refresh_log(
                    log_dir,
                    &format!("failed root manifest; url={target}; error={source}"),
                );
                errors.push(source);
            }
        }
    }

    if refreshed == 0 && !errors.is_empty() {
        return Err(errors.join("; "));
    }

    Ok(refreshed)
}

async fn refresh_changed_work_group_manifests(
    paths: &CatalogPaths,
    client: &reqwest::Client,
    checked_at: &str,
    root_manifest: &DirectoryManifest,
    log_dir: &Path,
) -> std::result::Result<usize, String> {
    let mut refreshed = 0;
    let mut errors = Vec::new();

    for child in directory_children(root_manifest) {
        tokio::time::sleep(Duration::from_secs(
            GPP_BACKGROUND_REFRESH_REQUEST_DELAY_SECS,
        ))
        .await;
        let work_group_url = ensure_trailing_slash(&child.url);
        let previous = read_manifest_by_url(paths, &work_group_url)?;
        match fetch_directory_manifest(
            client,
            &work_group_url,
            DirectoryRole::WorkGroup,
            checked_at,
        )
        .await
        {
            Ok(manifest) => {
                let should_descend = should_refresh_manifest_children(previous.as_ref(), &manifest);
                write_manifest(paths, &manifest).map_err(|source| source.to_string())?;
                let _ = append_background_refresh_log(
                    log_dir,
                    &format!(
                        "fetched workgroup manifest; url={work_group_url}; changed={should_descend}; children={}",
                        manifest.children.len()
                    ),
                );
                refreshed += 1;
                if should_descend {
                    refreshed +=
                        refresh_recent_meeting_docs(paths, client, checked_at, &manifest, log_dir)
                            .await?;
                }
            }
            Err(source) => {
                let _ = append_background_refresh_log(
                    log_dir,
                    &format!("failed workgroup manifest; url={work_group_url}; error={source}"),
                );
                errors.push(source);
            }
        }
    }

    if refreshed == 0 && !errors.is_empty() {
        return Err(errors.join("; "));
    }

    Ok(refreshed)
}

async fn refresh_recent_meeting_docs(
    paths: &CatalogPaths,
    client: &reqwest::Client,
    checked_at: &str,
    work_group_manifest: &DirectoryManifest,
    log_dir: &Path,
) -> std::result::Result<usize, String> {
    let mut refreshed = 0;
    let mut errors = Vec::new();

    for meeting_child in recent_meeting_children(
        work_group_manifest,
        GPP_BACKGROUND_REFRESH_MAX_MEETINGS_PER_WORKGROUP,
    ) {
        tokio::time::sleep(Duration::from_secs(
            GPP_BACKGROUND_REFRESH_REQUEST_DELAY_SECS,
        ))
        .await;
        let meeting_url = ensure_trailing_slash(&meeting_child.url);
        let previous_meeting = read_manifest_by_url(paths, &meeting_url)?;
        match fetch_directory_manifest(client, &meeting_url, DirectoryRole::MeetingRoot, checked_at)
            .await
        {
            Ok(meeting_manifest) => {
                let should_descend =
                    should_refresh_manifest_children(previous_meeting.as_ref(), &meeting_manifest);
                write_manifest(paths, &meeting_manifest).map_err(|source| source.to_string())?;
                let _ = append_background_refresh_log(
                    log_dir,
                    &format!(
                        "fetched meeting manifest; url={meeting_url}; changed={should_descend}; children={}",
                        meeting_manifest.children.len()
                    ),
                );
                refreshed += 1;
                if should_descend {
                    refreshed += refresh_docs_manifest(
                        paths,
                        client,
                        checked_at,
                        &meeting_manifest,
                        log_dir,
                    )
                    .await?;
                }
            }
            Err(source) => {
                let _ = append_background_refresh_log(
                    log_dir,
                    &format!("failed meeting manifest; url={meeting_url}; error={source}"),
                );
                errors.push(source);
            }
        }
    }

    if refreshed == 0 && !errors.is_empty() {
        return Err(errors.join("; "));
    }

    Ok(refreshed)
}

async fn refresh_docs_manifest(
    paths: &CatalogPaths,
    client: &reqwest::Client,
    checked_at: &str,
    meeting_manifest: &DirectoryManifest,
    log_dir: &Path,
) -> std::result::Result<usize, String> {
    let Some(docs_child) = meeting_manifest
        .children
        .iter()
        .find(|child| child.role == DirectoryRole::Docs)
    else {
        return Ok(0);
    };

    tokio::time::sleep(Duration::from_secs(
        GPP_BACKGROUND_REFRESH_REQUEST_DELAY_SECS,
    ))
    .await;
    let docs_url = ensure_trailing_slash(&docs_child.url);
    let previous_docs = read_manifest_by_url(paths, &docs_url)?;
    let docs_manifest =
        fetch_directory_manifest(client, &docs_url, DirectoryRole::Docs, checked_at).await?;
    let should_write_records =
        should_refresh_manifest_children(previous_docs.as_ref(), &docs_manifest);
    write_manifest(paths, &docs_manifest).map_err(|source| source.to_string())?;

    if should_write_records {
        write_docs_tdoc_records(paths, &docs_manifest, checked_at)?;
    }
    let _ = append_background_refresh_log(
        log_dir,
        &format!(
            "fetched docs manifest; url={docs_url}; changed={should_write_records}; children={}",
            docs_manifest.children.len()
        ),
    );

    Ok(1)
}

fn write_docs_tdoc_records(
    paths: &CatalogPaths,
    docs_manifest: &DirectoryManifest,
    checked_at: &str,
) -> std::result::Result<(), String> {
    let records =
        file_records_from_docs_manifest(docs_manifest).map_err(|source| source.to_string())?;
    let tdoc_records = records
        .into_iter()
        .filter(|record| record.classification.is_primary_tdoc)
        .collect::<Vec<_>>();
    if tdoc_records.is_empty() {
        return Ok(());
    }

    let Some(meeting_slug) = docs_manifest.path_segments.get(2) else {
        return Ok(());
    };
    let Some(work_group_code) = tdoc_records
        .iter()
        .find_map(|record| record.work_group_code.clone())
        .or_else(|| {
            tdoc_records.iter().find_map(|record| {
                record
                    .tdoc
                    .as_ref()
                    .and_then(|tdoc| source_for_tdoc_prefix(&tdoc.prefix))
                    .map(|source| source.work_group_code)
            })
        })
    else {
        return Ok(());
    };

    write_discovered_tdoc_records(
        paths,
        &work_group_code,
        meeting_slug,
        &ensure_trailing_slash(&docs_manifest.url),
        checked_at,
        &tdoc_records,
    )
}

async fn search_online_tdoc(
    paths: &CatalogPaths,
    query: &str,
    checked_at: &str,
    client: &reqwest::Client,
) -> std::result::Result<(Vec<FileRecord>, usize), String> {
    let tdoc = normalize_tdoc_query(query)
        .ok_or_else(|| "Enter a TDoc number such as R2-2601401 or R2-2601401.zip".to_string())?;
    let sources = infer_tdoc_sources(&tdoc);
    if sources.is_empty() {
        return Err(format!(
            "{} is a valid-looking TDoc number, but its prefix is not mapped yet",
            tdoc.key
        ));
    }

    let mut searched_url_count = 0;
    for source in sources {
        let work_group_manifest = fetch_directory_manifest(
            client,
            &source.work_group_url,
            DirectoryRole::WorkGroup,
            checked_at,
        )
        .await?;
        searched_url_count += 1;
        write_manifest(paths, &work_group_manifest).map_err(|source| source.to_string())?;

        let Some(filter) = likely_meeting_filter(&tdoc.key) else {
            continue;
        };
        let mut meetings = work_group_manifest
            .children
            .iter()
            .filter(|child| child.kind == EntryKind::Directory)
            .filter(|child| filter(&child.name))
            .collect::<Vec<_>>();
        meetings.sort_by(|left, right| right.name.cmp(&left.name));

        for chunk in meetings.into_iter().take(24).collect::<Vec<_>>().chunks(8) {
            let mut tasks = JoinSet::new();
            for meeting in chunk {
                let client = client.clone();
                let checked_at = checked_at.to_string();
                let meeting_url = ensure_trailing_slash(&meeting.url);
                tasks.spawn(async move {
                    fetch_meeting_docs_files(&client, &meeting_url, &checked_at).await
                });
            }

            while let Some(joined) = tasks.join_next().await {
                let Ok(Ok((manifests, files, url_count))) = joined else {
                    continue;
                };
                searched_url_count += url_count;
                for manifest in &manifests {
                    write_manifest(paths, manifest).map_err(|source| source.to_string())?;
                }
                if let (Some(work_group_code), Some(meeting_slug)) = (
                    files
                        .first()
                        .and_then(|file| file.work_group_code.as_deref()),
                    files.first().and_then(|file| file.meeting_slug.as_deref()),
                ) {
                    let docs_url = files
                        .first()
                        .map(|file| file.parent_directory_url.as_str())
                        .unwrap_or_default();
                    write_discovered_tdoc_records(
                        paths,
                        work_group_code,
                        meeting_slug,
                        docs_url,
                        &checked_at,
                        &files,
                    )?;
                } else {
                    write_file_records(paths, &files).map_err(|source| source.to_string())?;
                }
                let index = TDocLookupIndex::from_files(&files);
                if let Some(record) = resolve_tdoc(&tdoc.key, &index, &files) {
                    tasks.abort_all();
                    return Ok((vec![record.clone()], searched_url_count));
                }
            }
        }
    }

    Ok((Vec::new(), searched_url_count))
}

async fn fetch_meeting_docs_files(
    client: &reqwest::Client,
    meeting_url: &str,
    checked_at: &str,
) -> std::result::Result<(Vec<DirectoryManifest>, Vec<FileRecord>, usize), String> {
    let meeting_manifest =
        fetch_directory_manifest(client, meeting_url, DirectoryRole::MeetingRoot, checked_at)
            .await?;
    let mut manifests = vec![meeting_manifest.clone()];
    let mut url_count = 1;
    let Some(docs_child) = meeting_manifest
        .children
        .iter()
        .find(|child| child.role == DirectoryRole::Docs)
    else {
        return Ok((manifests, Vec::new(), url_count));
    };

    let docs_url = ensure_trailing_slash(&docs_child.url);
    let docs_manifest =
        fetch_directory_manifest(client, &docs_url, DirectoryRole::Docs, checked_at).await?;
    url_count += 1;
    let files =
        file_records_from_docs_manifest(&docs_manifest).map_err(|source| source.to_string())?;
    manifests.push(docs_manifest);

    Ok((manifests, files, url_count))
}

fn write_discovered_tdoc_records(
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

fn ensure_trailing_slash(url: &str) -> String {
    if url.ends_with('/') {
        url.to_string()
    } else {
        format!("{url}/")
    }
}

#[tauri::command]
fn runtime_paths(app: AppHandle) -> std::result::Result<RuntimePaths, String> {
    let layout = app_runtime_layout(&app)?;
    let settings = read_or_create_app_settings(&layout)?;
    let paths = build_runtime_paths(&layout, &settings);
    ensure_runtime_layout_dirs(&layout)?;
    ensure_dir(&paths.workspace_root)?;
    ensure_dir(&paths.three_gpp_workspace_dir)?;
    CatalogPaths::new(&layout.three_gpp_metadata_dir)
        .ensure_dirs()
        .map_err(|source| source.to_string())?;

    Ok(paths)
}

#[tauri::command]
fn gpp_catalog_status(app: AppHandle) -> std::result::Result<GppCatalogStatus, String> {
    let layout = app_runtime_layout(&app)?;
    let paths = app_catalog_paths(&app)?;
    let summary = summarize_catalog(&paths).map_err(|source| source.to_string())?;
    let refresh_settings = app_background_refresh_settings(&app)?;
    let refresh_log_path = background_refresh_log_path(&layout.logs_dir);
    to_catalog_status(&paths, summary, &refresh_settings, &refresh_log_path)
}

#[tauri::command]
fn set_gpp_background_refresh_enabled(
    app: AppHandle,
    enabled: bool,
) -> std::result::Result<GppCatalogStatus, String> {
    let layout = app_runtime_layout(&app)?;
    let mut settings = read_background_refresh_settings(&layout.config_dir)?;
    settings.enabled = enabled;
    append_background_refresh_log(
        &layout.logs_dir,
        &format!("updated scheduled refresh setting; enabled={enabled}"),
    )?;
    write_background_refresh_settings(&layout.config_dir, &settings)?;
    gpp_catalog_status(app)
}

#[tauri::command]
fn set_workspace_root(
    app: AppHandle,
    workspace_root: String,
) -> std::result::Result<RuntimePaths, String> {
    let normalized = workspace_root.trim();
    if normalized.is_empty() {
        return Err("workspace root cannot be empty".to_string());
    }

    let layout = app_runtime_layout(&app)?;
    let settings = AppSettings {
        record_type: "spectrumpilot-settings".to_string(),
        workspace_root: normalized.to_string(),
    };
    write_app_settings(&layout, &settings)?;
    let paths = build_runtime_paths(&layout, &settings);
    ensure_dir(&paths.workspace_root)?;
    ensure_dir(&paths.three_gpp_workspace_dir)?;
    Ok(paths)
}

#[tauri::command]
async fn bootstrap_gpp_catalog(app: AppHandle) -> std::result::Result<GppBootstrapReport, String> {
    let paths = app_catalog_paths(&app)?;
    let targets = bootstrap_targets();
    let checked_at = Utc::now().to_rfc3339();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("SpectrumPilot/0.1 3GPP catalog bootstrap")
        .build()
        .map_err(|source| format!("failed to build HTTP client: {source}"))?;

    let mut child_entry_count = 0;
    for target in &targets {
        let html = client
            .get(*target)
            .send()
            .await
            .map_err(|source| format!("failed to fetch {target}: {source}"))?
            .error_for_status()
            .map_err(|source| format!("failed to fetch {target}: {source}"))?
            .text()
            .await
            .map_err(|source| format!("failed to read {target}: {source}"))?;
        let manifest = build_manifest_from_html(target, DirectoryRole::Root, &checked_at, &html)
            .map_err(|source| format!("failed to parse {target}: {source}"))?;
        child_entry_count += manifest.children.len();
        write_manifest(&paths, &manifest).map_err(|source| source.to_string())?;
    }

    let summary = summarize_catalog(&paths).map_err(|source| source.to_string())?;
    Ok(GppBootstrapReport {
        fetched_url_count: targets.len(),
        manifest_count: summary.manifest_count,
        child_entry_count,
        target_roots: targets.into_iter().map(str::to_string).collect(),
        checked_at,
        catalog_root: paths.root().display().to_string(),
    })
}

#[tauri::command]
async fn search_gpp_tdoc(
    app: AppHandle,
    request: GppTdocSearchRequest,
) -> std::result::Result<GppTdocSearchReport, String> {
    let paths = app_catalog_paths(&app)?;
    let checked_at = Utc::now().to_rfc3339();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("SpectrumPilot/0.1 3GPP tdoc search")
        .build()
        .map_err(|source| format!("failed to build HTTP client: {source}"))?;

    let normalized = normalize_tdoc_query(&request.query)
        .ok_or_else(|| "Enter a TDoc number such as R2-2601401 or R2-2601401.zip".to_string())?;
    let records = read_file_records(&paths).map_err(|source| source.to_string())?;
    let index = TDocLookupIndex::from_files(&records);
    if let Some(record) = resolve_tdoc(&normalized.key, &index, &records) {
        return Ok(GppTdocSearchReport {
            query: request.query,
            normalized_query: normalized.key,
            source: "local-index".to_string(),
            searched_url_count: 0,
            results: vec![file_record_to_result(record, "local-index")],
            message: "Resolved from the local catalog cache.".to_string(),
        });
    }

    let (results, searched_url_count) =
        search_online_tdoc(&paths, &normalized.key, &checked_at, &client).await?;
    let message = if results.is_empty() {
        "No matching TDoc was found in the indexed 3GPP branches.".to_string()
    } else {
        "Resolved from a targeted 3GPP online search.".to_string()
    };

    Ok(GppTdocSearchReport {
        query: request.query,
        normalized_query: normalized.key,
        source: if results.is_empty() {
            "online-search-miss".to_string()
        } else {
            "online-search".to_string()
        },
        searched_url_count,
        results: results
            .into_iter()
            .map(|record| file_record_to_result(&record, "online-search"))
            .collect(),
        message,
    })
}

#[tauri::command]
async fn download_gpp_tdoc(
    app: AppHandle,
    request: GppTdocDownloadRequest,
) -> std::result::Result<GppTdocDownloadReport, String> {
    let workspace_root = app_workspace_root(&app)?;
    fs::create_dir_all(workspace_root.join("3gpp").join("downloads"))
        .map_err(|source| format!("failed to create workspace: {source}"))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("SpectrumPilot/0.1 3GPP download")
        .build()
        .map_err(|source| format!("failed to build HTTP client: {source}"))?;

    let response = client
        .get(&request.url)
        .send()
        .await
        .map_err(|source| format!("failed to download {}: {source}", request.url))?
        .error_for_status()
        .map_err(|source| format!("failed to download {}: {source}", request.url))?;
    let size_bytes = response.content_length().unwrap_or(0);
    let bytes = response
        .bytes()
        .await
        .map_err(|source| format!("failed to read download {}: {source}", request.url))?;
    let file_name = download_file_name_for_url(&request.url)
        .ok_or_else(|| format!("invalid download URL: {}", request.url))?;
    let saved_path = download_target_path(&workspace_root, &request.url);
    fs::write(&saved_path, &bytes)
        .map_err(|source| format!("failed to write {}: {source}", saved_path.display()))?;

    Ok(GppTdocDownloadReport {
        file_name,
        source_url: request.url,
        saved_path: saved_path.display().to_string(),
        size_bytes: if size_bytes == 0 {
            bytes.len() as u64
        } else {
            size_bytes
        },
    })
}

#[tauri::command]
fn catalog_root(app_cache_dir: String) -> String {
    let paths = CatalogPaths::new(app_cache_dir);
    paths.root().display().to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(gpp::jobs::JobRegistry::default())
        .setup(|app| {
            spawn_background_gpp_catalog_refresh(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_status,
            runtime_paths,
            set_workspace_root,
            gpp_catalog_status,
            set_gpp_background_refresh_enabled,
            bootstrap_gpp_catalog,
            search_gpp_tdoc,
            download_gpp_tdoc,
            gpp::workflow::start_gpp_lookup_job,
            gpp::workflow::cancel_gpp_lookup_job,
            catalog_root
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::app_settings_path;
    use super::append_background_refresh_log;
    use super::background_refresh_log_path;
    use super::background_refresh_targets;
    use super::bootstrap_targets;
    use super::build_runtime_layout;
    use super::build_runtime_paths;
    use super::download_file_name_for_url;
    use super::download_target_path;
    use super::install_bundled_catalog_seed_if_empty;
    use super::likely_meeting_filter;
    use super::migrate_runtime_layout;
    use super::read_app_settings;
    use super::read_background_refresh_settings;
    use super::read_background_refresh_status;
    use super::read_or_create_app_settings;
    use super::recent_meeting_children;
    use super::search_online_tdoc;
    use super::should_refresh_manifest_children;
    use super::to_catalog_status;
    use super::write_app_settings;
    use super::write_background_refresh_settings;
    use super::write_background_refresh_status;
    use super::write_discovered_tdoc_records;
    use super::write_docs_tdoc_records;
    use super::AppSettings;
    use super::BackgroundRefreshSettings;
    use super::BackgroundRefreshStatus;
    use super::GPP_BACKGROUND_REFRESH_INTERVAL_MINUTES;
    use chrono::Utc;
    use spectrumpilot_3gpp_core::catalog::{
        read_tdoc_index_shard, read_tdoc_meeting_shard, write_manifest, CatalogPaths,
    };
    use spectrumpilot_3gpp_core::model::{
        DirectoryChild, DirectoryManifest, DirectoryRole, EntryKind, FileClassification,
        FileRecord, MeetingRecord, TDocKey,
    };

    #[test]
    fn runtime_layout_uses_single_spectrumpilot_storage_root() {
        let layout = build_runtime_layout(
            &PathBuf::from("/home/alice"),
            &PathBuf::from("/home/alice/.local/share/com.hzh.spectrumpilot"),
            &PathBuf::from("/home/alice/.cache/com.hzh.spectrumpilot"),
            &PathBuf::from("/home/alice/.local/share/com.hzh.spectrumpilot/logs"),
        );

        assert_eq!(
            layout.app_storage_dir,
            PathBuf::from("/home/alice/.local/share/SpectrumPilot")
        );
        assert_eq!(
            layout.config_dir,
            PathBuf::from("/home/alice/.local/share/SpectrumPilot/config")
        );
        assert_eq!(
            layout.metadata_dir,
            PathBuf::from("/home/alice/.local/share/SpectrumPilot/metadata")
        );
        assert_eq!(
            layout.internal_cache_dir,
            PathBuf::from("/home/alice/.local/share/SpectrumPilot/cache")
        );
        assert_eq!(
            layout.logs_dir,
            PathBuf::from("/home/alice/.local/share/SpectrumPilot/logs")
        );
        assert_eq!(
            layout.three_gpp_catalog_dir,
            PathBuf::from("/home/alice/.local/share/SpectrumPilot/metadata/3gpp/catalog")
        );
        assert_eq!(
            layout.default_workspace_root,
            PathBuf::from("/home/alice/SpectrumPilotWorkspace")
        );
    }

    #[test]
    fn runtime_paths_expose_workspace_and_internal_storage() {
        let layout = build_runtime_layout(
            &PathBuf::from("/home/alice"),
            &PathBuf::from("/home/alice/.local/share/com.hzh.spectrumpilot"),
            &PathBuf::from("/home/alice/.cache/com.hzh.spectrumpilot"),
            &PathBuf::from("/home/alice/.local/share/com.hzh.spectrumpilot/logs"),
        );
        let settings = AppSettings {
            record_type: "spectrumpilot-settings".to_string(),
            workspace_root: "/data/SpectrumPilotWorkspace".to_string(),
        };
        let paths = build_runtime_paths(&layout, &settings);

        assert_eq!(
            paths.app_storage_dir,
            "/home/alice/.local/share/SpectrumPilot"
        );
        assert_eq!(
            paths.metadata_dir,
            "/home/alice/.local/share/SpectrumPilot/metadata"
        );
        assert_eq!(
            paths.internal_cache_dir,
            "/home/alice/.local/share/SpectrumPilot/cache"
        );
        assert_eq!(
            paths.logs_dir,
            "/home/alice/.local/share/SpectrumPilot/logs"
        );
        assert_eq!(paths.workspace_root, "/data/SpectrumPilotWorkspace");
        assert_eq!(
            paths.three_gpp_workspace_dir,
            "/data/SpectrumPilotWorkspace/3gpp"
        );
        assert_eq!(
            paths.three_gpp_catalog_dir,
            "/home/alice/.local/share/SpectrumPilot/metadata/3gpp/catalog"
        );
    }

    #[test]
    fn app_settings_default_workspace_root_is_home_workspace() {
        let temp = tempfile::tempdir().expect("tempdir");
        let layout = build_runtime_layout(
            &temp.path().join("home"),
            &temp.path().join("legacy-data"),
            &temp.path().join("legacy-cache"),
            &temp.path().join("legacy-logs"),
        );

        let settings = read_app_settings(&layout).expect("settings");

        assert_eq!(
            settings.workspace_root,
            temp.path()
                .join("home")
                .join("SpectrumPilotWorkspace")
                .display()
                .to_string()
        );
    }

    #[test]
    fn app_settings_roundtrip_workspace_root() {
        let temp = tempfile::tempdir().expect("tempdir");
        let layout = build_runtime_layout(
            &temp.path().join("home"),
            &temp.path().join("legacy-data"),
            &temp.path().join("legacy-cache"),
            &temp.path().join("legacy-logs"),
        );
        let settings = AppSettings {
            record_type: "spectrumpilot-settings".to_string(),
            workspace_root: temp.path().join("workspace").display().to_string(),
        };

        write_app_settings(&layout, &settings).expect("write settings");
        let read = read_app_settings(&layout).expect("read settings");

        assert_eq!(read, settings);
    }

    #[test]
    fn app_settings_are_created_with_default_workspace_on_first_run() {
        let temp = tempfile::tempdir().expect("tempdir");
        let layout = build_runtime_layout(
            &temp.path().join("home"),
            &temp.path().join("legacy-data"),
            &temp.path().join("legacy-cache"),
            &temp.path().join("legacy-logs"),
        );

        let settings = read_or_create_app_settings(&layout).expect("settings");

        assert_eq!(
            settings.workspace_root,
            temp.path()
                .join("home")
                .join("SpectrumPilotWorkspace")
                .display()
                .to_string()
        );
        assert!(app_settings_path(&layout).exists());
    }

    #[test]
    fn runtime_layout_migrates_legacy_catalog_settings_and_logs() {
        let temp = tempfile::tempdir().expect("tempdir");
        let home = temp.path().join("home");
        let legacy_data = temp.path().join("legacy-data");
        let legacy_cache = temp.path().join("legacy-cache");
        let legacy_logs = legacy_data.join("logs");
        let layout = build_runtime_layout(&home, &legacy_data, &legacy_cache, &legacy_logs);
        let old_catalog = legacy_cache.join("3gpp").join("catalog");
        std::fs::create_dir_all(old_catalog.join("manifests")).expect("catalog dir");
        std::fs::write(old_catalog.join("seed.json"), "{}").expect("seed");
        std::fs::write(old_catalog.join("manifests").join("root.json"), "{}").expect("manifest");
        std::fs::create_dir_all(legacy_data.join("config")).expect("config dir");
        std::fs::write(
            legacy_data.join("config").join("3gpp-settings.json"),
            r#"{"recordType":"3gpp-background-refresh-settings","enabled":false,"intervalMinutes":60}"#,
        )
        .expect("refresh settings");
        std::fs::create_dir_all(&legacy_logs).expect("logs dir");
        std::fs::write(legacy_logs.join("3gpp-refresh.log"), "old log").expect("log");

        migrate_runtime_layout(&layout).expect("migrate");

        assert!(layout.three_gpp_catalog_dir.join("seed.json").exists());
        assert!(layout
            .three_gpp_catalog_dir
            .join("manifests")
            .join("root.json")
            .exists());
        assert!(layout.config_dir.join("3gpp-settings.json").exists());
        assert!(layout.logs_dir.join("3gpp-refresh.log").exists());
    }

    #[test]
    fn runtime_layout_migration_preserves_legacy_files_and_existing_targets() {
        let temp = tempfile::tempdir().expect("tempdir");
        let home = temp.path().join("home");
        let legacy_data = temp.path().join("legacy-data");
        let legacy_cache = temp.path().join("legacy-cache");
        let legacy_logs = legacy_data.join("logs");
        let layout = build_runtime_layout(&home, &legacy_data, &legacy_cache, &legacy_logs);
        let legacy_catalog = legacy_cache.join("3gpp").join("catalog");
        let legacy_seed = legacy_catalog.join("seed.json");
        let target_seed = layout.three_gpp_catalog_dir.join("seed.json");
        std::fs::create_dir_all(&legacy_catalog).expect("legacy catalog");
        std::fs::write(&legacy_seed, "legacy seed").expect("legacy seed");
        std::fs::create_dir_all(&layout.three_gpp_catalog_dir).expect("target catalog");
        std::fs::write(&target_seed, "existing seed").expect("target seed");

        migrate_runtime_layout(&layout).expect("migrate");

        assert_eq!(
            std::fs::read_to_string(&legacy_seed).expect("legacy seed still exists"),
            "legacy seed"
        );
        assert_eq!(
            std::fs::read_to_string(&target_seed).expect("target seed still exists"),
            "existing seed"
        );
    }

    #[test]
    fn bootstrap_targets_are_limited_to_ftp_and_tsg_roots() {
        let targets = bootstrap_targets();

        assert_eq!(targets.len(), 7);
        assert_eq!(targets[0], "https://www.3gpp.org/ftp/");
        assert_eq!(
            targets[1..],
            [
                "https://www.3gpp.org/ftp/tsg_cn/",
                "https://www.3gpp.org/ftp/tsg_ct/",
                "https://www.3gpp.org/ftp/tsg_geran/",
                "https://www.3gpp.org/ftp/tsg_ran/",
                "https://www.3gpp.org/ftp/tsg_sa/",
                "https://www.3gpp.org/ftp/tsg_t/"
            ]
        );
    }

    #[test]
    fn background_refresh_targets_only_supported_tsg_roots() {
        let targets = background_refresh_targets();

        assert_eq!(targets.len(), 6);
        assert!(targets
            .iter()
            .all(|target| target.starts_with("https://www.3gpp.org/ftp/tsg_")));
        assert!(!targets.contains(&"https://www.3gpp.org/ftp/"));
        assert!(targets.contains(&"https://www.3gpp.org/ftp/tsg_ran/"));
        assert!(targets.contains(&"https://www.3gpp.org/ftp/tsg_sa/"));
        assert!(targets.contains(&"https://www.3gpp.org/ftp/tsg_ct/"));
    }

    #[test]
    fn background_refresh_settings_default_to_enabled() {
        let temp = tempfile::tempdir().expect("tempdir");

        let settings = read_background_refresh_settings(temp.path()).expect("settings");

        assert!(settings.enabled);
        assert_eq!(
            settings.interval_minutes,
            GPP_BACKGROUND_REFRESH_INTERVAL_MINUTES
        );
    }

    #[test]
    fn background_refresh_settings_roundtrip_disabled() {
        let temp = tempfile::tempdir().expect("tempdir");
        let settings = BackgroundRefreshSettings {
            record_type: "3gpp-background-refresh-settings".to_string(),
            enabled: false,
            interval_minutes: GPP_BACKGROUND_REFRESH_INTERVAL_MINUTES,
        };

        write_background_refresh_settings(temp.path(), &settings).expect("write settings");
        let read = read_background_refresh_settings(temp.path()).expect("read settings");

        assert_eq!(read, settings);
    }

    #[test]
    fn background_refresh_log_appends_lines_to_log_file() {
        let temp = tempfile::tempdir().expect("tempdir");

        append_background_refresh_log(temp.path(), "started root refresh").expect("write log");
        let body =
            std::fs::read_to_string(background_refresh_log_path(temp.path())).expect("read log");

        assert!(body.contains("started root refresh"));
    }

    #[test]
    fn background_refresh_descends_only_when_manifest_fingerprint_changes() {
        let old_manifest = test_manifest(
            "https://www.3gpp.org/ftp/tsg_ran/",
            DirectoryRole::Root,
            "sha256:old",
            vec![],
        );
        let unchanged_manifest = test_manifest(
            "https://www.3gpp.org/ftp/tsg_ran/",
            DirectoryRole::Root,
            "sha256:old",
            vec![],
        );
        let changed_manifest = test_manifest(
            "https://www.3gpp.org/ftp/tsg_ran/",
            DirectoryRole::Root,
            "sha256:new",
            vec![],
        );

        assert!(!should_refresh_manifest_children(
            Some(&old_manifest),
            &unchanged_manifest
        ));
        assert!(should_refresh_manifest_children(
            Some(&old_manifest),
            &changed_manifest
        ));
        assert!(should_refresh_manifest_children(None, &unchanged_manifest));
    }

    #[test]
    fn background_refresh_selects_recent_meeting_children_with_limit() {
        let manifest = test_manifest(
            "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/",
            DirectoryRole::WorkGroup,
            "sha256:wg",
            vec![
                test_dir_child("TSGR2_120"),
                test_dir_child("TSGR2_133bis"),
                test_dir_child("TSGR2_132"),
                test_dir_child("Archive"),
                test_file_child("README.txt"),
                test_dir_child("TSGR2_131"),
            ],
        );

        let selected = recent_meeting_children(&manifest, 2);

        assert_eq!(
            selected
                .iter()
                .map(|child| child.name.as_str())
                .collect::<Vec<_>>(),
            vec!["TSGR2_133bis", "TSGR2_132"]
        );
    }

    #[test]
    fn background_refresh_writes_docs_records_to_shards_and_lookup_index() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = CatalogPaths::new(temp.path().join("3gpp"));
        let docs_manifest = test_docs_manifest("R2-2601401");

        write_docs_tdoc_records(&paths, &docs_manifest, "2026-07-02T00:00:00Z")
            .expect("write docs records");

        let meeting_shard = read_tdoc_meeting_shard(&paths, "RAN2", "TSGR2_133bis")
            .expect("read meeting shard")
            .expect("meeting shard");
        let index_shard = read_tdoc_index_shard(&paths, "R2", 2026)
            .expect("read index shard")
            .expect("index shard");

        assert_eq!(meeting_shard.files.len(), 1);
        assert!(index_shard.items.contains_key("R2-2601401"));
    }

    #[test]
    fn likely_meeting_filter_targets_same_year_and_recent_meetings() {
        let filter = likely_meeting_filter("R2-2601401").expect("filter");

        assert!(filter("TSGR2_133bis"));
        assert!(filter("TSGR2_132"));
        assert!(!filter("TSGR2_95"));
        assert!(!filter("TSGR1_133bis"));
    }

    #[test]
    fn download_target_keeps_3gpp_workspace_layout() {
        let workspace = PathBuf::from("/tmp/SpectrumPilotWorkspace");
        let url = "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip";

        assert_eq!(
            download_file_name_for_url(url).as_deref(),
            Some("R2-2601401.zip")
        );
        assert_eq!(
            download_target_path(&workspace, url),
            workspace
                .join("3gpp")
                .join("downloads")
                .join("R2-2601401.zip")
        );
    }

    #[test]
    fn installs_bundled_seed_catalog_into_empty_catalog() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = CatalogPaths::new(temp.path());

        let installed = install_bundled_catalog_seed_if_empty(&paths).expect("install seed");
        let summary = spectrumpilot_3gpp_core::catalog::summarize_catalog(&paths).expect("summary");
        let second_install = install_bundled_catalog_seed_if_empty(&paths).expect("second install");

        assert!(installed >= 7);
        assert!(summary.manifest_count >= 7);
        assert_eq!(second_install, 0);
    }

    #[test]
    fn installs_missing_seed_records_and_indexes_when_manifests_exist() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = CatalogPaths::new(temp.path());
        let existing_manifest = DirectoryManifest {
            schema_version: 1,
            record_type: "directory-manifest".to_string(),
            url: "https://www.3gpp.org/ftp/".to_string(),
            path_segments: vec![],
            directory_role: DirectoryRole::Root,
            checked_at: "2026-07-02T08:00:00Z".to_string(),
            child_fingerprint: "sha256:existing".to_string(),
            children: vec![],
        };
        write_manifest(&paths, &existing_manifest).expect("existing manifest");

        let installed = install_bundled_catalog_seed_if_empty(&paths).expect("install seed");
        let summary = spectrumpilot_3gpp_core::catalog::summarize_catalog(&paths).expect("summary");

        assert!(installed > 0);
        assert!(summary.record_count > 0);
        assert!(summary.index_count > 0);
    }

    #[test]
    fn catalog_status_includes_bundled_seed_metadata() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = CatalogPaths::new(temp.path());
        install_bundled_catalog_seed_if_empty(&paths).expect("install seed");
        let summary = spectrumpilot_3gpp_core::catalog::summarize_catalog(&paths).expect("summary");

        let settings = BackgroundRefreshSettings {
            record_type: "3gpp-background-refresh-settings".to_string(),
            enabled: true,
            interval_minutes: GPP_BACKGROUND_REFRESH_INTERVAL_MINUTES,
        };
        let status = to_catalog_status(
            &paths,
            summary,
            &settings,
            &background_refresh_log_path(temp.path()),
        )
        .expect("status");

        assert_eq!(status.seed_version, "stage-seed-2026-07-02");
        assert_eq!(
            status.seed_generated_at,
            Some("2026-07-02T00:00:00Z".to_string())
        );
        assert_eq!(
            status.seed_scope,
            "RAN2 meetings TSGR2_132 and TSGR2_133bis"
        );
    }

    #[test]
    fn background_refresh_status_roundtrips_to_catalog_root() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = CatalogPaths::new(temp.path());
        let status = BackgroundRefreshStatus {
            record_type: "3gpp-background-refresh-status".to_string(),
            state: "succeeded".to_string(),
            last_started_at: Some("2026-07-03T08:00:00Z".to_string()),
            last_completed_at: Some("2026-07-03T08:01:00Z".to_string()),
            last_error: None,
            last_refreshed_manifest_count: 12,
        };

        write_background_refresh_status(&paths, &status).expect("write status");
        let read = read_background_refresh_status(&paths)
            .expect("read status")
            .expect("status exists");

        assert_eq!(read, status);
    }

    #[test]
    fn catalog_status_includes_persisted_background_refresh_status() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = CatalogPaths::new(temp.path());
        install_bundled_catalog_seed_if_empty(&paths).expect("install seed");
        write_background_refresh_status(
            &paths,
            &BackgroundRefreshStatus {
                record_type: "3gpp-background-refresh-status".to_string(),
                state: "failed".to_string(),
                last_started_at: Some("2026-07-03T08:00:00Z".to_string()),
                last_completed_at: None,
                last_error: Some("HTTP 429".to_string()),
                last_refreshed_manifest_count: 0,
            },
        )
        .expect("write refresh status");
        let summary = spectrumpilot_3gpp_core::catalog::summarize_catalog(&paths).expect("summary");

        let settings = BackgroundRefreshSettings {
            record_type: "3gpp-background-refresh-settings".to_string(),
            enabled: true,
            interval_minutes: GPP_BACKGROUND_REFRESH_INTERVAL_MINUTES,
        };
        let status = to_catalog_status(
            &paths,
            summary,
            &settings,
            &background_refresh_log_path(temp.path()),
        )
        .expect("status");

        assert_eq!(status.background_refresh_state, "failed");
        assert_eq!(
            status.background_refresh_last_started_at,
            Some("2026-07-03T08:00:00Z".to_string())
        );
        assert_eq!(status.background_refresh_last_completed_at, None);
        assert_eq!(
            status.background_refresh_last_error,
            Some("HTTP 429".to_string())
        );
        assert_eq!(status.background_refresh_last_refreshed_manifest_count, 0);
    }

    #[test]
    fn writes_discovered_records_as_meeting_shard_and_index_shard() {
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

        assert!(read_tdoc_meeting_shard(&paths, "RAN2", "TSGR2_133bis")
            .expect("read meeting")
            .is_some());
        assert!(read_tdoc_index_shard(&paths, "R2", 2026)
            .expect("read index")
            .expect("index")
            .items
            .contains_key("R2-2601401"));
    }

    fn test_ran2_record(tdoc: &str, meeting_slug: &str) -> FileRecord {
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

    fn test_manifest(
        url: &str,
        role: DirectoryRole,
        child_fingerprint: &str,
        children: Vec<DirectoryChild>,
    ) -> DirectoryManifest {
        DirectoryManifest {
            schema_version: 1,
            record_type: "directory-manifest".to_string(),
            url: url.to_string(),
            path_segments: vec![],
            directory_role: role,
            checked_at: "2026-07-02T00:00:00Z".to_string(),
            child_fingerprint: child_fingerprint.to_string(),
            children,
        }
    }

    fn test_dir_child(name: &str) -> DirectoryChild {
        DirectoryChild {
            name: name.to_string(),
            kind: EntryKind::Directory,
            url: format!("https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/{name}/"),
            role: DirectoryRole::Unknown,
            remote_modified_raw: Some("2026/07/02 12:00".to_string()),
            size_raw: None,
            size_bytes: None,
        }
    }

    fn test_file_child(name: &str) -> DirectoryChild {
        DirectoryChild {
            name: name.to_string(),
            kind: EntryKind::File,
            url: format!("https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/{name}"),
            role: DirectoryRole::Auxiliary,
            remote_modified_raw: Some("2026/07/02 12:00".to_string()),
            size_raw: Some("1 KB".to_string()),
            size_bytes: Some(1024),
        }
    }

    fn test_docs_manifest(tdoc: &str) -> DirectoryManifest {
        DirectoryManifest {
            schema_version: 1,
            record_type: "directory-manifest".to_string(),
            url: "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/".to_string(),
            path_segments: vec![
                "tsg_ran".to_string(),
                "WG2_RL2".to_string(),
                "TSGR2_133bis".to_string(),
                "Docs".to_string(),
            ],
            directory_role: DirectoryRole::Docs,
            checked_at: "2026-07-02T00:00:00Z".to_string(),
            child_fingerprint: "sha256:docs".to_string(),
            children: vec![DirectoryChild {
                name: format!("{tdoc}.zip"),
                kind: EntryKind::File,
                url: format!(
                    "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/{tdoc}.zip"
                ),
                role: DirectoryRole::Auxiliary,
                remote_modified_raw: Some("2026/07/02 12:00".to_string()),
                size_raw: Some("10 KB".to_string()),
                size_bytes: Some(10_240),
            }],
        }
    }

    #[tokio::test]
    #[ignore]
    async fn live_online_search_finds_known_ran2_tdoc() {
        let temp_root = std::env::temp_dir().join(format!(
            "spectrumpilot-live-search-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let paths = CatalogPaths::new(temp_root);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .user_agent("SpectrumPilot/0.1 live smoke test")
            .build()
            .expect("client");

        let (records, searched_urls) =
            search_online_tdoc(&paths, "R2-2601401", &Utc::now().to_rfc3339(), &client)
                .await
                .expect("live search");

        assert!(searched_urls > 0);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].file_name, "R2-2601401.zip");
    }
}
