use spectrum_3gpp_core::model::{FileRecord, MeetingRecord};

#[test]
fn meeting_id_uses_root_workgroup_and_slug() {
    let id = MeetingRecord::stable_id("tsg_ran", "WG2_RL2", "TSGR2_133bis");
    assert_eq!(id, "meeting:tsg_ran/WG2_RL2/TSGR2_133bis");
}

#[test]
fn file_id_uses_canonical_url_hash() {
    let id = FileRecord::stable_id(
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip",
    );

    assert_eq!(
        id,
        "file-url-sha256:2254601a1bb1bff3fe98fe104ea06c92a9b96da6ee6d6c3012d33312c944e8e3"
    );
}
