use crate::model::TDocKey;
use crate::normalize::TDocSource;

pub fn source_for_tdoc_prefix(prefix: &str) -> Option<TDocSource> {
    let prefix = prefix.to_ascii_uppercase();
    let (root, work_group_path, work_group_code, meeting_series_prefix, default_start_meeting) =
        match prefix.as_str() {
            "RP" => ("tsg_ran", "TSG_RAN", "RAN", "TSGR", Some("TSGR_100")),
            "R1" => ("tsg_ran", "WG1_RL1", "RAN1", "TSGR1", Some("TSGR1_105")),
            "R2" => ("tsg_ran", "WG2_RL2", "RAN2", "TSGR2", Some("TSGR2_120")),
            "R3" => ("tsg_ran", "WG3_Iu", "RAN3", "TSGR3", Some("TSGR3_120")),
            "R4" => ("tsg_ran", "WG4_Radio", "RAN4", "TSGR4", Some("TSGR4_100")),
            "R5" => (
                "tsg_ran",
                "WG5_Test_ex-T1",
                "RAN5",
                "TSGR5",
                Some("TSGR5_100"),
            ),
            "SP" => ("tsg_sa", "TSG_SA", "SA", "TSGS", Some("TSGS_100")),
            "S1" => ("tsg_sa", "WG1_Serv", "SA1", "TSGS1", Some("TSGS1_100")),
            "S2" => ("tsg_sa", "WG2_Arch", "SA2", "TSGS2", Some("TSGS2_170")),
            "S3" => ("tsg_sa", "WG3_Security", "SA3", "TSGS3", Some("TSGS3_110")),
            "S4" => ("tsg_sa", "WG4_CODEC", "SA4", "TSGS4", Some("TSGS4_125")),
            "S5" => ("tsg_sa", "WG5_TM", "SA5", "TSGS5", Some("TSGS5_145")),
            "S6" => (
                "tsg_sa",
                "WG6_MissionCritical",
                "SA6",
                "TSGS6",
                Some("TSGS6_70"),
            ),
            "CP" => ("tsg_ct", "TSG_CT", "CT", "TSGC", Some("TSGC_100")),
            "C1" => (
                "tsg_ct",
                "WG1_mm-cc-sm_ex-CN1",
                "CT1",
                "TSGC1",
                Some("TSGC1_145"),
            ),
            "C2" => (
                "tsg_ct",
                "WG2_capability_ex-T2",
                "CT2",
                "TSGC2",
                Some("TSGC2_125"),
            ),
            "C3" => (
                "tsg_ct",
                "WG3_interworking_ex-CN3",
                "CT3",
                "TSGC3",
                Some("TSGC3_125"),
            ),
            "C4" => (
                "tsg_ct",
                "WG4_protocollars_ex-CN4",
                "CT4",
                "TSGC4",
                Some("TSGC4_120"),
            ),
            "C5" => ("tsg_ct", "WG5_osa_ex-CN5", "CT5", "TSGC5", Some("TSGC5_80")),
            "C6" => (
                "tsg_ct",
                "WG6_Smartcard_Ex-T3",
                "CT6",
                "TSGC6",
                Some("TSGC6_110"),
            ),
            _ => return None,
        };

    Some(TDocSource {
        root: root.to_string(),
        work_group_path: work_group_path.to_string(),
        work_group_code: work_group_code.to_string(),
        work_group_url: format!("https://www.3gpp.org/ftp/{root}/{work_group_path}/"),
        meeting_series_prefix: meeting_series_prefix.to_string(),
        default_start_meeting: default_start_meeting.map(str::to_string),
    })
}

pub fn direct_probe_url(source: &TDocSource, meeting_slug: &str, tdoc: &TDocKey) -> String {
    format!(
        "https://www.3gpp.org/ftp/{}/{}/{}/Docs/{}.zip",
        source.root, source.work_group_path, meeting_slug, tdoc.key
    )
}
