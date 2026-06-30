use spectrum_3gpp_core::model::{FileRecord, MeetingRecord};
use spectrum_3gpp_core::normalize::{
    infer_work_group, parse_meeting_slug, parse_size_bytes, parse_tdoc_key,
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
