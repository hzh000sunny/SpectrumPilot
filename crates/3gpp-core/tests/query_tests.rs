use spectrum_3gpp_core::query::{parse_gpp_query, GppQuery};

#[test]
fn parses_spec_queries() {
    assert!(matches!(
        parse_gpp_query("38.321").expect("query"),
        GppQuery::Specification(spec)
            if spec.spec_number == "38.321"
                && spec.archive_stem == "38321"
                && spec.version_prefix.is_none()
                && spec.exact_version.is_none()
    ));
    assert!(matches!(
        parse_gpp_query("38321").expect("query"),
        GppQuery::Specification(spec)
            if spec.spec_number == "38.321" && spec.archive_stem == "38321"
    ));
    assert!(matches!(
        parse_gpp_query("38.321 f").expect("query"),
        GppQuery::Specification(spec)
            if spec.spec_number == "38.321" && spec.version_prefix.as_deref() == Some("f")
    ));
    assert!(matches!(
        parse_gpp_query("38.321 f10").expect("query"),
        GppQuery::Specification(spec)
            if spec.spec_number == "38.321" && spec.exact_version.as_deref() == Some("f10")
    ));
    assert!(matches!(
        parse_gpp_query("38321-f10").expect("query"),
        GppQuery::Specification(spec)
            if spec.spec_number == "38.321" && spec.exact_version.as_deref() == Some("f10")
    ));
    assert!(matches!(
        parse_gpp_query("38.101-1 j50").expect("query"),
        GppQuery::Specification(spec)
            if spec.spec_number == "38.101-1"
                && spec.archive_stem == "38101-1"
                && spec.exact_version.as_deref() == Some("j50")
    ));
    assert!(matches!(
        parse_gpp_query("38101-1-j50").expect("query"),
        GppQuery::Specification(spec)
            if spec.spec_number == "38.101-1"
                && spec.archive_stem == "38101-1"
                && spec.exact_version.as_deref() == Some("j50")
    ));
}

#[test]
fn parses_contribution_queries() {
    assert!(matches!(
        parse_gpp_query("r2-2601401.zip").expect("query"),
        GppQuery::Contribution(tdoc)
            if tdoc.tdoc.key == "R2-2601401" && tdoc.meeting_hint.is_none()
    ));
    assert!(matches!(
        parse_gpp_query("R2-2601401 TSGR2_133bis").expect("query"),
        GppQuery::Contribution(tdoc)
            if tdoc.tdoc.key == "R2-2601401"
                && tdoc.meeting_hint.as_deref() == Some("TSGR2_133bis")
    ));
    assert!(matches!(
        parse_gpp_query("R2-2601401 133bis").expect("query"),
        GppQuery::Contribution(tdoc)
            if tdoc.tdoc.key == "R2-2601401" && tdoc.meeting_hint.as_deref() == Some("133bis")
    ));
    assert!(matches!(
        parse_gpp_query("R2-2601401 from TSGR2_120").expect("query"),
        GppQuery::Contribution(tdoc)
            if tdoc.tdoc.key == "R2-2601401"
                && tdoc.start_meeting.as_deref() == Some("TSGR2_120")
    ));
}
