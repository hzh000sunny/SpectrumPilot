use spectrum_3gpp_core::catalog::CatalogPaths;
use spectrum_3gpp_core::catalog::{
    file_record_path, manifest_path_for_url, read_file_records, read_spec_archive_record,
    spec_archive_record_path, summarize_catalog, write_file_records, write_manifest,
    write_spec_archive_record,
};
use spectrum_3gpp_core::model::{
    DirectoryManifest, DirectoryRole, FileClassification, FileRecord, SpecArchiveRecord, TDocKey,
};

#[test]
fn builds_expected_catalog_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = CatalogPaths::new(temp.path());

    assert!(paths.root().ends_with("catalog"));
    assert!(paths.manifests_dir().ends_with("manifests"));
    assert!(paths.records_dir().ends_with("records"));
    assert!(paths.indexes_dir().ends_with("indexes"));
}

#[test]
fn manifest_path_uses_windows_safe_url_hash() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = CatalogPaths::new(temp.path());
    let path = manifest_path_for_url(&paths, "https://www.3gpp.org/ftp/tsg_ran/");

    assert!(path.starts_with(paths.manifests_dir()));
    assert_eq!(
        path.extension().and_then(|value| value.to_str()),
        Some("json")
    );
    assert_eq!(
        path.file_stem()
            .and_then(|value| value.to_str())
            .expect("file stem")
            .len(),
        64
    );
    assert!(path
        .file_stem()
        .and_then(|value| value.to_str())
        .expect("file stem")
        .chars()
        .all(|value| value.is_ascii_hexdigit()));
}

#[test]
fn writes_manifest_and_summarizes_catalog() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = CatalogPaths::new(temp.path());
    let manifest = DirectoryManifest {
        schema_version: 1,
        record_type: "directory-manifest".to_string(),
        url: "https://www.3gpp.org/ftp/".to_string(),
        path_segments: vec![],
        directory_role: DirectoryRole::Root,
        checked_at: "2026-07-01T00:00:00Z".to_string(),
        child_fingerprint: "sha256:test".to_string(),
        children: vec![],
    };

    write_manifest(&paths, &manifest).expect("write manifest");
    let summary = summarize_catalog(&paths).expect("catalog summary");
    let written = std::fs::read_to_string(manifest_path_for_url(&paths, &manifest.url))
        .expect("manifest JSON");

    assert_eq!(summary.manifest_count, 1);
    assert_eq!(summary.record_count, 0);
    assert_eq!(summary.index_count, 0);
    assert_eq!(
        summary.last_checked_at.as_deref(),
        Some("2026-07-01T00:00:00Z")
    );
    assert!(written.contains("\"schemaVersion\""));
    assert!(written.contains("\"checkedAt\""));
    assert!(!written.contains("\"schema_version\""));
    assert!(!written.contains("\"checked_at\""));
}

#[test]
fn writes_and_reads_file_records() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = CatalogPaths::new(temp.path());
    let url = "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip";
    let record = FileRecord {
        schema_version: 1,
        record_type: "tdoc-file".to_string(),
        id: FileRecord::stable_id(url),
        canonical_url: url.to_string(),
        parent_directory_url: "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/"
            .to_string(),
        root: "tsg_ran".to_string(),
        work_group_path: "WG2_RL2".to_string(),
        work_group_code: Some("RAN2".to_string()),
        meeting_id: Some("meeting:tsg_ran/WG2_RL2/TSGR2_133bis".to_string()),
        meeting_slug: Some("TSGR2_133bis".to_string()),
        container_role: DirectoryRole::Docs,
        file_name: "R2-2601401.zip".to_string(),
        extension: Some("zip".to_string()),
        remote_modified_raw: Some("2026/04/03 9:50".to_string()),
        size_raw: Some("78,5 KB".to_string()),
        size_bytes: Some(80_384),
        tdoc: Some(TDocKey {
            key: "R2-2601401".to_string(),
            prefix: "R2".to_string(),
            number_text: "2601401".to_string(),
            year_hint: Some(2026),
        }),
        classification: FileClassification {
            is_primary_tdoc: true,
            is_zip: true,
            is_ignored_artifact: false,
        },
    };

    write_file_records(&paths, &[record.clone()]).expect("write records");

    let records = read_file_records(&paths).expect("read records");
    let summary = summarize_catalog(&paths).expect("summary");

    assert_eq!(records, vec![record]);
    assert_eq!(summary.record_count, 1);
}

#[test]
fn file_record_path_uses_windows_safe_file_name() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = CatalogPaths::new(temp.path());
    let url = "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip";
    let record = FileRecord {
        schema_version: 1,
        record_type: "tdoc-file".to_string(),
        id: FileRecord::stable_id(url),
        canonical_url: url.to_string(),
        parent_directory_url: "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/"
            .to_string(),
        root: "tsg_ran".to_string(),
        work_group_path: "WG2_RL2".to_string(),
        work_group_code: Some("RAN2".to_string()),
        meeting_id: Some("meeting:tsg_ran/WG2_RL2/TSGR2_133bis".to_string()),
        meeting_slug: Some("TSGR2_133bis".to_string()),
        container_role: DirectoryRole::Docs,
        file_name: "R2-2601401.zip".to_string(),
        extension: Some("zip".to_string()),
        remote_modified_raw: Some("2026/04/03 9:50".to_string()),
        size_raw: Some("78,5 KB".to_string()),
        size_bytes: Some(80_384),
        tdoc: Some(TDocKey {
            key: "R2-2601401".to_string(),
            prefix: "R2".to_string(),
            number_text: "2601401".to_string(),
            year_hint: Some(2026),
        }),
        classification: FileClassification {
            is_primary_tdoc: true,
            is_zip: true,
            is_ignored_artifact: false,
        },
    };
    let path = file_record_path(&paths, &record);

    assert!(path.starts_with(paths.records_dir()));
    assert_eq!(
        path.extension().and_then(|value| value.to_str()),
        Some("json")
    );
    assert!(!path
        .file_name()
        .and_then(|value| value.to_str())
        .expect("file name")
        .contains(':'));
}

#[test]
fn writes_and_reads_spec_archive_records_by_series_and_spec_number() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = CatalogPaths::new(temp.path());
    let record = SpecArchiveRecord {
        schema_version: 1,
        record_type: "spec-archive-record".to_string(),
        spec_number: "38.321".to_string(),
        archive_url: "https://www.3gpp.org/ftp/Specs/archive/38_series/38.321/".to_string(),
        checked_at: "2026-07-04T00:00:00Z".to_string(),
        files: vec!["38321-f10.zip".to_string(), "38321-j30.zip".to_string()],
    };

    let path = spec_archive_record_path(&paths, "38.321");
    assert!(path.starts_with(paths.records_dir().join("specs").join("38_series")));
    assert_eq!(
        path.file_name().and_then(|value| value.to_str()),
        Some("38.321.json")
    );

    write_spec_archive_record(&paths, &record).expect("write spec record");
    assert_eq!(
        read_spec_archive_record(&paths, "38.321").expect("read spec record"),
        Some(record)
    );
    assert_eq!(
        read_spec_archive_record(&paths, "38.322").expect("missing spec record"),
        None
    );
}
