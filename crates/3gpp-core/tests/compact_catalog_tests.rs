use spectrum_3gpp_core::catalog::{summarize_catalog, CatalogPaths};
use spectrum_3gpp_core::compact::resolve_tdoc_from_compact_catalog;

fn write_json(path: &std::path::Path, body: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    std::fs::write(path, body).expect("write json");
}

#[test]
fn resolves_tdoc_from_compact_catalog_index_and_records() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = CatalogPaths::new(temp.path().join("3gpp"));
    write_json(
        &paths.root().join("compact/records/RAN2.json"),
        r#"{
          "schemaVersion":1,
          "recordType":"tdoc-compact-records",
          "workGroupCode":"RAN2",
          "baseUrl":"https://www.3gpp.org/ftp/",
          "meetings":[{
            "id":0,
            "meetingSlug":"TSGR2_133bis",
            "docsPath":"tsg_ran/WG2_RL2/TSGR2_133bis/Docs",
            "checkedAt":"2026-07-04T00:00:00Z",
            "files":[["R2-2601401.zip",80437,"04-03-26 09:50AM","R2-2601401"]]
          }]
        }"#,
    );
    write_json(
        &paths.root().join("compact/index/R2_26.json"),
        r#"{
          "schemaVersion":1,
          "recordType":"tdoc-compact-index",
          "prefix":"R2",
          "year":2026,
          "items":{"R2-2601401":["RAN2",0,0]}
        }"#,
    );
    write_json(
        &paths.root().join("compact/summary.json"),
        r#"{
          "schemaVersion":1,
          "recordType":"tdoc-compact-summary",
          "catalogFormat":"compact-v1",
          "recordCount":1,
          "meetingCount":1,
          "recordShardCount":1,
          "indexShardCount":1,
          "indexItemCount":1,
          "latestCheckedAt":"2026-07-04T00:00:00Z"
        }"#,
    );

    let resolved = resolve_tdoc_from_compact_catalog(&paths, "R2", 2026, "R2-2601401")
        .expect("resolve")
        .expect("resolved entry");

    assert_eq!(resolved.tdoc, "R2-2601401");
    assert_eq!(resolved.file_name, "R2-2601401.zip");
    assert_eq!(
        resolved.url,
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip"
    );
    assert_eq!(resolved.work_group_code, "RAN2");
    assert_eq!(resolved.meeting_slug, "TSGR2_133bis");
    assert_eq!(
        resolved.remote_modified_raw.as_deref(),
        Some("04-03-26 09:50AM")
    );
    assert_eq!(resolved.size_bytes, Some(80437));

    let summary = summarize_catalog(&paths).expect("summary");
    assert_eq!(summary.record_count, 1);
    assert_eq!(summary.index_count, 1);
    assert_eq!(
        summary.last_checked_at.as_deref(),
        Some("2026-07-04T00:00:00Z")
    );
}

#[test]
fn returns_none_when_compact_index_shard_is_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = CatalogPaths::new(temp.path().join("3gpp"));

    let resolved =
        resolve_tdoc_from_compact_catalog(&paths, "R2", 2026, "R2-2601401").expect("resolve");

    assert!(resolved.is_none());
}
