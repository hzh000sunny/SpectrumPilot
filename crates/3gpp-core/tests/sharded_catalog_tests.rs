use spectrum_3gpp_core::catalog::{
    merge_tdoc_index_shard, read_tdoc_index_shard, read_tdoc_meeting_shard, summarize_catalog,
    tdoc_index_shard_path, tdoc_meeting_shard_path, write_tdoc_index_shard,
    write_tdoc_meeting_shard, CatalogPaths,
};
use spectrum_3gpp_core::index::{build_tdoc_index_shards, resolve_tdoc_from_index_shard};
use spectrum_3gpp_core::model::{TDocIndexEntry, TDocIndexShard, TDocMeetingRecordShard};

mod support {
    include!("./support/file_records.rs");
}

#[test]
fn tdoc_meeting_shard_serializes_records_for_one_meeting() {
    let record = support::ran2_record("R2-2601401", "TSGR2_133bis");
    let shard = TDocMeetingRecordShard::from_records(
        "RAN2",
        "TSGR2_133bis",
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/",
        "2026-07-02T08:00:00Z",
        vec![record.clone()],
    );

    let body = serde_json::to_string(&shard).expect("serialize shard");

    assert!(body.contains("\"recordType\":\"tdoc-meeting-records\""));
    assert_eq!(shard.work_group_code, "RAN2");
    assert_eq!(shard.meeting_slug, "TSGR2_133bis");
    assert_eq!(shard.files, vec![record]);
}

#[test]
fn tdoc_index_shard_serializes_lookup_entries_by_key() {
    let entry = TDocIndexEntry {
        tdoc: "R2-2601401".to_string(),
        file_name: "R2-2601401.zip".to_string(),
        url: "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip"
            .to_string(),
        work_group_code: "RAN2".to_string(),
        meeting_slug: "TSGR2_133bis".to_string(),
        record_shard: "records/tdoc/RAN2/TSGR2_133bis.json".to_string(),
        remote_modified_raw: None,
        size_bytes: None,
    };
    let shard = TDocIndexShard::new("R2", 2026, vec![entry.clone()]);

    assert_eq!(shard.record_type, "tdoc-lookup-index");
    assert_eq!(shard.prefix, "R2");
    assert_eq!(shard.year, 2026);
    assert_eq!(shard.items.get("R2-2601401"), Some(&entry));
}

#[test]
fn catalog_writes_meeting_shard_and_index_shard() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = CatalogPaths::new(temp.path().join("3gpp"));
    let record = support::ran2_record("R2-2601401", "TSGR2_133bis");
    let meeting_shard = TDocMeetingRecordShard::from_records(
        "RAN2",
        "TSGR2_133bis",
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/",
        "2026-07-02T08:00:00Z",
        vec![record],
    );

    let meeting_path = write_tdoc_meeting_shard(&paths, &meeting_shard).expect("write meeting");
    assert_eq!(
        meeting_path,
        paths.root().join("records/tdoc/RAN2/TSGR2_133bis.json")
    );
    assert_eq!(
        tdoc_meeting_shard_path(&paths, "RAN2", "TSGR2_133bis"),
        meeting_path
    );

    let index_entry = TDocIndexEntry {
        tdoc: "R2-2601401".to_string(),
        file_name: "R2-2601401.zip".to_string(),
        url: "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip"
            .to_string(),
        work_group_code: "RAN2".to_string(),
        meeting_slug: "TSGR2_133bis".to_string(),
        record_shard: "records/tdoc/RAN2/TSGR2_133bis.json".to_string(),
        remote_modified_raw: Some("2026/05/22 10:14".to_string()),
        size_bytes: Some(10_240),
    };
    let index_shard = TDocIndexShard::new("R2", 2026, vec![index_entry]);
    let index_path = write_tdoc_index_shard(&paths, &index_shard).expect("write index");
    assert_eq!(index_path, paths.root().join("indexes/tdoc/R2/26.json"));
    assert_eq!(tdoc_index_shard_path(&paths, "R2", 2026), index_path);

    let read_meeting = read_tdoc_meeting_shard(&paths, "RAN2", "TSGR2_133bis")
        .expect("read meeting")
        .expect("meeting exists");
    let read_index = read_tdoc_index_shard(&paths, "R2", 2026)
        .expect("read index")
        .expect("index exists");
    assert_eq!(read_meeting.files.len(), 1);
    assert!(read_index.items.contains_key("R2-2601401"));

    let summary = summarize_catalog(&paths).expect("summary");
    assert_eq!(summary.record_count, 1);
    assert_eq!(summary.index_count, 1);
}

#[test]
fn builds_index_shards_by_prefix_and_year() {
    let first = support::ran2_record("R2-2601401", "TSGR2_133bis");
    let second = support::ran2_record("R2-2508956", "TSGR2_132");
    let shards = build_tdoc_index_shards(&[first, second]);

    assert_eq!(shards.len(), 2);
    assert!(shards
        .iter()
        .any(|shard| shard.prefix == "R2" && shard.year == 2026));
    assert!(shards
        .iter()
        .any(|shard| shard.prefix == "R2" && shard.year == 2025));
}

#[test]
fn resolves_tdoc_from_single_index_shard() {
    let record = support::ran2_record("R2-2601401", "TSGR2_133bis");
    let shard = build_tdoc_index_shards(&[record])
        .into_iter()
        .next()
        .expect("index shard");

    let resolved = resolve_tdoc_from_index_shard("R2-2601401", &shard).expect("resolved");
    assert_eq!(
        resolved.url,
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip"
    );
    assert_eq!(resolved.record_shard, "records/tdoc/RAN2/TSGR2_133bis.json");
}

#[test]
fn merges_index_shards_without_dropping_existing_entries() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = CatalogPaths::new(temp.path().join("3gpp"));
    let first = support::ran2_record("R2-2601401", "TSGR2_133bis");
    let second = support::ran2_record("R2-2601402", "TSGR2_133bis");
    let first_shard = build_tdoc_index_shards(&[first])
        .into_iter()
        .next()
        .expect("first shard");
    let second_shard = build_tdoc_index_shards(&[second])
        .into_iter()
        .next()
        .expect("second shard");

    merge_tdoc_index_shard(&paths, &first_shard).expect("merge first");
    merge_tdoc_index_shard(&paths, &second_shard).expect("merge second");

    let merged = read_tdoc_index_shard(&paths, "R2", 2026)
        .expect("read merged")
        .expect("merged shard");
    assert!(merged.items.contains_key("R2-2601401"));
    assert!(merged.items.contains_key("R2-2601402"));
}
