use spectrum_3gpp_core::normalize::normalize_tdoc_query;
use spectrum_3gpp_core::tdoc::{direct_probe_url, source_for_tdoc_prefix};

#[test]
fn maps_plenary_and_workgroup_prefixes() {
    let cases = [
        ("RP", "tsg_ran", "TSG_RAN", "TSGR"),
        ("R1", "tsg_ran", "WG1_RL1", "TSGR1"),
        ("R2", "tsg_ran", "WG2_RL2", "TSGR2"),
        ("R3", "tsg_ran", "WG3_Iu", "TSGR3"),
        ("R4", "tsg_ran", "WG4_Radio", "TSGR4"),
        ("R5", "tsg_ran", "WG5_Test_ex-T1", "TSGR5"),
        ("SP", "tsg_sa", "TSG_SA", "TSGS"),
        ("S1", "tsg_sa", "WG1_Serv", "TSGS1"),
        ("S2", "tsg_sa", "WG2_Arch", "TSGS2"),
        ("S3", "tsg_sa", "WG3_Security", "TSGS3"),
        ("S4", "tsg_sa", "WG4_CODEC", "TSGS4"),
        ("S5", "tsg_sa", "WG5_TM", "TSGS5"),
        ("S6", "tsg_sa", "WG6_MissionCritical", "TSGS6"),
        ("CP", "tsg_ct", "TSG_CT", "TSGC"),
        ("C1", "tsg_ct", "WG1_mm-cc-sm_ex-CN1", "TSGC1"),
        ("C2", "tsg_ct", "WG2_capability_ex-T2", "TSGC2"),
        ("C3", "tsg_ct", "WG3_interworking_ex-CN3", "TSGC3"),
        ("C4", "tsg_ct", "WG4_protocollars_ex-CN4", "TSGC4"),
        ("C5", "tsg_ct", "WG5_osa_ex-CN5", "TSGC5"),
        ("C6", "tsg_ct", "WG6_Smartcard_Ex-T3", "TSGC6"),
    ];

    for (prefix, root, path, series) in cases {
        let source = source_for_tdoc_prefix(prefix).expect(prefix);
        assert_eq!(source.root, root, "{prefix}");
        assert_eq!(source.work_group_path, path, "{prefix}");
        assert_eq!(source.meeting_series_prefix, series, "{prefix}");
    }
}

#[test]
fn maps_default_start_meetings_for_ran_sa_and_ct_sources() {
    let cases = [
        ("RP", "RAN", Some("TSGR_100")),
        ("R2", "RAN2", Some("TSGR2_120")),
        ("SP", "SA", Some("TSGS_100")),
        ("S2", "SA2", Some("TSGS2_170")),
        ("CP", "CT", Some("TSGC_100")),
        ("C1", "CT1", Some("TSGC1_145")),
    ];

    for (prefix, work_group, start) in cases {
        let source = source_for_tdoc_prefix(prefix).expect(prefix);
        assert_eq!(source.work_group_code, work_group);
        assert_eq!(source.default_start_meeting.as_deref(), start, "{prefix}");
    }
}

#[test]
fn builds_exact_direct_probe_url() {
    let tdoc = normalize_tdoc_query("R2-2601401").expect("tdoc");
    let source = source_for_tdoc_prefix(&tdoc.prefix).expect("source");
    let url = direct_probe_url(&source, "TSGR2_133bis", &tdoc);

    assert_eq!(
        url,
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip"
    );
}
