use spectrum_3gpp_core::model::{
    DirectoryRole, FileClassification, FileRecord, MeetingRecord, TDocKey,
};

pub fn ran2_record(tdoc: &str, meeting_slug: &str) -> FileRecord {
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
        meeting_id: Some(MeetingRecord::stable_id(
            "tsg_ran",
            "WG2_RL2",
            meeting_slug,
        )),
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
            year_hint: Some(year_hint_from_tdoc(tdoc)),
        }),
        classification: FileClassification {
            is_primary_tdoc: true,
            is_zip: true,
            is_ignored_artifact: false,
        },
    }
}

fn year_hint_from_tdoc(tdoc: &str) -> u32 {
    let year_digits = tdoc
        .split_once('-')
        .map(|(_, number)| number.chars().take(2).collect::<String>())
        .unwrap_or_default();
    let year = year_digits.parse::<u32>().unwrap_or(0);
    if year >= 80 {
        1900 + year
    } else {
        2000 + year
    }
}
