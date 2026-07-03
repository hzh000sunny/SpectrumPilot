use spectrum_3gpp_core::query::{parse_gpp_query, GppQuery};
use spectrum_3gpp_core::specs::{
    archive_directory_url, archive_file_name, select_latest_spec_file, SpecVersion,
};

#[test]
fn builds_archive_directory_and_exact_file_name() {
    let query = match parse_gpp_query("38.321 f10").expect("query") {
        GppQuery::Specification(spec) => spec,
        _ => panic!("expected spec"),
    };

    assert_eq!(
        archive_directory_url(&query),
        "https://www.3gpp.org/ftp/Specs/archive/38_series/38.321/"
    );
    assert_eq!(archive_file_name(&query, "f10"), "38321-f10.zip");
}

#[test]
fn builds_multipart_archive_directory_and_file_name() {
    let query = match parse_gpp_query("38.101-1 j50").expect("query") {
        GppQuery::Specification(spec) => spec,
        _ => panic!("expected spec"),
    };

    assert_eq!(
        archive_directory_url(&query),
        "https://www.3gpp.org/ftp/Specs/archive/38_series/38.101-1/"
    );
    assert_eq!(archive_file_name(&query, "j50"), "38101-1-j50.zip");
}

#[test]
fn sorts_and_selects_latest_versions() {
    let files = vec![
        "38321-f10.zip".to_string(),
        "38321-f20.zip".to_string(),
        "38321-j30.zip".to_string(),
        "38321-i90.zip".to_string(),
    ];

    assert_eq!(
        select_latest_spec_file("38321", None, &files).as_deref(),
        Some("38321-j30.zip")
    );
    assert_eq!(
        select_latest_spec_file("38321", Some("f"), &files).as_deref(),
        Some("38321-f20.zip")
    );
}

#[test]
fn parses_version_codes() {
    assert_eq!(
        SpecVersion::parse("f10").expect("version").release_letter,
        'f'
    );
    assert!(SpecVersion::parse("j30").expect("j30") > SpecVersion::parse("i90").expect("i90"));
    assert!(SpecVersion::parse("f20").expect("f20") > SpecVersion::parse("f10").expect("f10"));
}
