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

const UPPERCASE_FTP_HTML: &str = r#"
<table>
  <tbody>
    <tr>
      <td><input type="checkbox" value="R3-2600001.zip" /></td>
      <td><img src="/ftp/geticon.axd?file=.zip" /></td>
      <td><a class="file" href="https://www.3gpp.org/FTP/tsg_ran/WG3_Iu/TSGR3_34/Docs/R3-2600001.zip">R3-2600001.zip</a></td>
      <td>2026/01/01 9:50</td>
      <td>1 KB</td>
    </tr>
  </tbody>
</table>
"#;

const DOTTED_DIRECTORY_HTML: &str = r#"
<table>
  <tbody>
    <tr>
      <td></td>
      <td><img src="/ftp/geticon.axd?file=" /></td>
      <td><a href="https://www.3gpp.org/ftp/Specs/archive/26_series/26.347/">26.347</a></td>
      <td>2026/01/01 9:50</td>
      <td></td>
    </tr>
  </tbody>
</table>
"#;

const GBR_DIRECTORY_HTML: &str = r#"
<table>
  <tbody>
    <tr>
      <td></td>
      <td><img src="/ftp/geticon.axd?file=" /></td>
      <td><a href="https://www.3gpp.org/ftp/tsg_sa/WG2_Arch/TSGS2_82_Jacksonville/Joint_Sessions/JM3_MBR-gt-GBR_Wednesday0700/?sortby=namerev">JM3_MBR-gt-GBR_Wednesday0700</a></td>
      <td>2026/01/01 9:50</td>
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
    assert_eq!(
        rows[0].role,
        spectrum_3gpp_core::model::DirectoryRole::Auxiliary
    );
    assert_eq!(
        rows[0].remote_modified_raw.as_deref(),
        Some("2026/04/03 9:50")
    );
    assert_eq!(rows[0].size_raw.as_deref(), Some("78,5 KB"));
    assert_eq!(rows[0].size_bytes, Some(80_384));

    assert_eq!(rows[1].name, "__MACOSX");
    assert_eq!(rows[1].kind, EntryKind::Directory);
    assert_eq!(
        rows[1].role,
        spectrum_3gpp_core::model::DirectoryRole::Unknown
    );
    assert_eq!(rows[1].size_raw, None);
}

#[test]
fn parses_uppercase_ftp_links() {
    let rows = parse_directory_listing(UPPERCASE_FTP_HTML).expect("parse");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "R3-2600001.zip");
    assert_eq!(rows[0].kind, EntryKind::File);
    assert_eq!(rows[0].size_raw.as_deref(), Some("1 KB"));
    assert_eq!(rows[0].size_bytes, Some(1024));
}

#[test]
fn treats_dotted_archive_names_as_directories() {
    let rows = parse_directory_listing(DOTTED_DIRECTORY_HTML).expect("parse");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "26.347");
    assert_eq!(rows[0].kind, EntryKind::Directory);
    assert_eq!(rows[0].size_raw, None);
}

#[test]
fn does_not_confuse_gbr_in_directory_names_for_size() {
    let rows = parse_directory_listing(GBR_DIRECTORY_HTML).expect("parse");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "JM3_MBR-gt-GBR_Wednesday0700");
    assert_eq!(rows[0].kind, EntryKind::Directory);
    assert_eq!(rows[0].size_raw, None);
}
