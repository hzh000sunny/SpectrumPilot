use spectrum_3gpp_core::model::{FileRecord, MeetingRecord};
use spectrum_3gpp_core::normalize::{
    infer_tdoc_sources, infer_work_group, normalize_tdoc_query, parse_meeting_slug,
    parse_size_bytes, parse_tdoc_key,
};

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

#[test]
fn parses_decimal_comma_kilobytes() {
    assert_eq!(parse_size_bytes("78,5 KB"), Some(80_384));
    assert_eq!(parse_size_bytes("213 KB"), Some(218_112));
}

#[test]
fn parses_ran2_meeting_slug() {
    let parsed = parse_meeting_slug("TSGR2_133bis");
    assert_eq!(parsed.series.as_deref(), Some("TSGR2"));
    assert_eq!(parsed.number, Some(133));
    assert_eq!(parsed.variant.as_deref(), Some("bis"));
    assert_eq!(parsed.location, None);
    assert_eq!(parsed.scheduled_month, None);
}

#[test]
fn parses_sa2_meeting_slug_with_location_and_month() {
    let parsed = parse_meeting_slug("TSGS2_175_Dalian_2026-05");
    assert_eq!(parsed.series.as_deref(), Some("TSGS2"));
    assert_eq!(parsed.number, Some(175));
    assert_eq!(parsed.location.as_deref(), Some("Dalian"));
    assert_eq!(parsed.scheduled_month.as_deref(), Some("2026-05"));
}

#[test]
fn parses_tdoc_key_and_year_hint() {
    let key = parse_tdoc_key("R2-2601401.zip").expect("tdoc");
    assert_eq!(key.key, "R2-2601401");
    assert_eq!(key.prefix, "R2");
    assert_eq!(key.number_text, "2601401");
    assert_eq!(key.year_hint, Some(2026));
}

#[test]
fn infers_work_group_from_path() {
    let wg = infer_work_group("tsg_ran", "WG2_RL2");
    assert_eq!(wg.code.as_deref(), Some("RAN2"));
    assert_eq!(wg.label.as_deref(), Some("RAN WG2"));
}

#[test]
fn parses_other_real_tdoc_families() {
    let cases = [
        ("CP-201031.zip", "CP-201031", "CP", "201031", Some(2020)),
        ("GP-040012.zip", "GP-040012", "GP", "040012", Some(2004)),
        ("NP-040020.zip", "NP-040020", "NP", "040020", Some(2004)),
        ("RP-99616.zip", "RP-99616", "RP", "99616", Some(1999)),
        ("TP-030009.zip", "TP-030009", "TP", "030009", Some(2003)),
        ("SP-251469.zip", "SP-251469", "SP", "251469", Some(2025)),
        ("R1-99850.zip", "R1-99850", "R1", "99850", Some(1999)),
        ("S1-261026.zip", "S1-261026", "S1", "261026", Some(2026)),
        ("S4-260036.zip", "S4-260036", "S4", "260036", Some(2026)),
        ("C6-180102.zip", "C6-180102", "C6", "180102", Some(2018)),
        ("T2-000599.zip", "T2-000599", "T2", "000599", Some(2000)),
        ("N3-000553.zip", "N3-000553", "N3", "000553", Some(2000)),
    ];

    for (input, key, prefix, number_text, year_hint) in cases {
        let parsed = parse_tdoc_key(input).expect(input);
        assert_eq!(parsed.key, key);
        assert_eq!(parsed.prefix, prefix);
        assert_eq!(parsed.number_text, number_text);
        assert_eq!(parsed.year_hint, year_hint);
    }
}

#[test]
fn rejects_unknown_size_units() {
    assert_eq!(parse_size_bytes("78 XB"), None);
    assert_eq!(parse_size_bytes("78 KB/s"), None);
}

#[test]
fn rejects_non_tdoc_filenames() {
    assert_eq!(parse_tdoc_key("README-123.zip"), None);
    assert_eq!(parse_tdoc_key("WG1-12345.zip"), None);
    assert_eq!(parse_tdoc_key("RAN2-12345.zip"), None);
    assert_eq!(parse_tdoc_key("ABC-12345.zip"), None);
}

#[test]
fn normalizes_user_tdoc_queries() {
    let key = normalize_tdoc_query("  r2-2601401.zip  ").expect("tdoc query");

    assert_eq!(key.key, "R2-2601401");
    assert_eq!(key.prefix, "R2");
    assert_eq!(key.number_text, "2601401");
    assert_eq!(key.year_hint, Some(2026));
}

#[test]
fn infers_likely_source_branch_from_tdoc_prefix() {
    let key = normalize_tdoc_query("R2-2601401").expect("tdoc query");
    let sources = infer_tdoc_sources(&key);

    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].root, "tsg_ran");
    assert_eq!(sources[0].work_group_path, "WG2_RL2");
    assert_eq!(sources[0].work_group_code, "RAN2");
    assert_eq!(
        sources[0].work_group_url,
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/"
    );
    assert_eq!(sources[0].meeting_series_prefix, "TSGR2");
}
