use spectrum_3gpp_core::manifest::build_manifest_from_html;
use spectrum_3gpp_core::model::{DirectoryRole, EntryKind};

const MEETING_HTML: &str = r#"
<table>
  <tbody>
    <tr>
      <td></td>
      <td></td>
      <td><a href="https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs">Docs</a></td>
      <td>2026/06/25 9:59</td>
      <td></td>
    </tr>
    <tr>
      <td></td>
      <td></td>
      <td><a href="https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Report">Report</a></td>
      <td>2026/05/20 15:23</td>
      <td></td>
    </tr>
  </tbody>
</table>
"#;

#[test]
fn builds_manifest_with_stable_fingerprint() {
    let manifest = build_manifest_from_html(
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/",
        DirectoryRole::MeetingRoot,
        "2026-07-01T00:00:00Z",
        MEETING_HTML,
    )
    .expect("manifest");

    assert_eq!(
        manifest.path_segments,
        vec!["tsg_ran", "WG2_RL2", "TSGR2_133bis"]
    );
    assert_eq!(manifest.children.len(), 2);
    assert_eq!(manifest.children[0].name, "Docs");
    assert_eq!(manifest.children[0].kind, EntryKind::Directory);
    assert_eq!(manifest.children[0].role, DirectoryRole::Docs);
    assert_eq!(manifest.children[1].name, "Report");
    assert!(manifest.child_fingerprint.starts_with("sha256:"));
}
