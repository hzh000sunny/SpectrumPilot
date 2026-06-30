use spectrum_3gpp_core::model::EntryKind;
use spectrum_3gpp_core::parser::parse_directory_listing;

const DOCS_HTML: &str = r#"
<table>
  <tbody>
    <tr>
      <td><input type="checkbox" value="R2-2601401.zip" /></td>
      <td><img src="/ftp/geticon.axd?file=.zip" /></td>
      <td><a class="file" href="https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip">R2-2601401.zip</a></td>
      <td>2026/04/03 9:50</td>
      <td>78,5 KB</td>
    </tr>
    <tr>
      <td></td>
      <td><img src="/ftp/geticon.axd?file=" /></td>
      <td><a href="https://www.3gpp.org/ftp/tsg_ct/WG1_mm-cc-sm_ex-CN1/TSGC1_161_Dalian/Docs/__MACOSX">__MACOSX</a></td>
      <td>2026/05/22 7:29</td>
      <td></td>
    </tr>
  </tbody>
</table>
"#;

#[test]
fn parses_file_and_directory_rows() {
    let rows = parse_directory_listing(DOCS_HTML).expect("parse");
    assert_eq!(rows.len(), 2);

    assert_eq!(rows[0].name, "R2-2601401.zip");
    assert_eq!(rows[0].kind, EntryKind::File);
    assert_eq!(rows[0].remote_modified_raw.as_deref(), Some("2026/04/03 9:50"));
    assert_eq!(rows[0].size_raw.as_deref(), Some("78,5 KB"));

    assert_eq!(rows[1].name, "__MACOSX");
    assert_eq!(rows[1].kind, EntryKind::Directory);
    assert_eq!(rows[1].size_raw, None);
}
