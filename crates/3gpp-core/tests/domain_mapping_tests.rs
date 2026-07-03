use spectrum_3gpp_core::manifest::{
    build_manifest_from_html, file_records_from_docs_manifest, meeting_from_manifest,
};
use spectrum_3gpp_core::model::{DirectoryRole, DocsState};

#[test]
fn maps_meeting_manifest_to_meeting_record() {
    let html = r#"
    <table>
      <tr><td></td><td></td><td><a href="https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs">Docs</a></td><td>2026/06/25 9:59</td><td></td></tr>
    </table>
    "#;
    let manifest = build_manifest_from_html(
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/",
        DirectoryRole::MeetingRoot,
        "2026-07-01T00:00:00Z",
        html,
    )
    .expect("manifest");

    let meeting = meeting_from_manifest(&manifest).expect("meeting");
    assert_eq!(meeting.id, "meeting:tsg_ran/WG2_RL2/TSGR2_133bis");
    assert_eq!(meeting.work_group_code.as_deref(), Some("RAN2"));
    assert_eq!(meeting.docs_state, DocsState::Available);
    assert_eq!(meeting.meeting_number, Some(133));
}

#[test]
fn maps_docs_manifest_to_primary_tdoc_files() {
    let html = r#"
    <table>
      <tr><td></td><td></td><td><a href="https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip">R2-2601401.zip</a></td><td>2026/04/03 9:50</td><td>78,5 KB</td></tr>
      <tr><td></td><td></td><td><a href="https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/__MACOSX">__MACOSX</a></td><td>2026/05/22 7:29</td><td></td></tr>
    </table>
    "#;
    let manifest = build_manifest_from_html(
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/",
        DirectoryRole::Docs,
        "2026-07-01T00:00:00Z",
        html,
    )
    .expect("manifest");

    let files = file_records_from_docs_manifest(&manifest).expect("files");
    assert_eq!(files.len(), 2);
    assert!(files[0].classification.is_primary_tdoc);
    assert_eq!(files[0].tdoc.as_ref().unwrap().key, "R2-2601401");
    assert!(files[1].classification.is_ignored_artifact);
}
