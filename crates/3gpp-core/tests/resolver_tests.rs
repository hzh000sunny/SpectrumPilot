use spectrum_3gpp_core::index::TDocLookupIndex;
use spectrum_3gpp_core::model::{DirectoryRole, FileClassification, FileRecord, TDocKey};
use spectrum_3gpp_core::resolver::resolve_tdoc;

fn sample_file() -> FileRecord {
    let url = "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip";
    FileRecord {
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
    }
}

#[test]
fn resolves_tdoc_from_local_index() {
    let file = sample_file();
    let index = TDocLookupIndex::from_files(&[file.clone()]);
    let files = [file];
    let resolved = resolve_tdoc("r2-2601401", &index, &files).expect("match");

    assert_eq!(resolved.file_name, "R2-2601401.zip");
    assert_eq!(resolved.work_group_code.as_deref(), Some("RAN2"));
}
