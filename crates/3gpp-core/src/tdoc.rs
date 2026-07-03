use crate::model::TDocKey;
use crate::normalize::TDocSource;

pub fn source_for_tdoc_prefix(prefix: &str) -> Option<TDocSource> {
    let prefix = prefix.to_ascii_uppercase();
    let (root, work_group_path, work_group_code, meeting_series_prefix) = match prefix.as_str() {
        "RP" => ("tsg_ran", "TSG_RAN", "RAN", "TSGR"),
        "R1" => ("tsg_ran", "WG1_RL1", "RAN1", "TSGR1"),
        "R2" => ("tsg_ran", "WG2_RL2", "RAN2", "TSGR2"),
        "R3" => ("tsg_ran", "WG3_Iu", "RAN3", "TSGR3"),
        "R4" => ("tsg_ran", "WG4_Radio", "RAN4", "TSGR4"),
        "R5" => ("tsg_ran", "WG5_Test_ex-T1", "RAN5", "TSGR5"),
        "SP" => ("tsg_sa", "TSG_SA", "SA", "TSGS"),
        "S1" => ("tsg_sa", "WG1_Serv", "SA1", "TSGS1"),
        "S2" => ("tsg_sa", "WG2_Arch", "SA2", "TSGS2"),
        "S3" => ("tsg_sa", "WG3_Security", "SA3", "TSGS3"),
        "S4" => ("tsg_sa", "WG4_CODEC", "SA4", "TSGS4"),
        "S5" => ("tsg_sa", "WG5_TM", "SA5", "TSGS5"),
        "S6" => ("tsg_sa", "WG6_MissionCritical", "SA6", "TSGS6"),
        "CP" => ("tsg_ct", "TSG_CT", "CT", "TSGC"),
        "C1" => ("tsg_ct", "WG1_mm-cc-sm_ex-CN1", "CT1", "TSGC1"),
        "C2" => ("tsg_ct", "WG2_capability_ex-T2", "CT2", "TSGC2"),
        "C3" => ("tsg_ct", "WG3_interworking_ex-CN3", "CT3", "TSGC3"),
        "C4" => ("tsg_ct", "WG4_protocollars_ex-CN4", "CT4", "TSGC4"),
        "C5" => ("tsg_ct", "WG5_osa_ex-CN5", "CT5", "TSGC5"),
        "C6" => ("tsg_ct", "WG6_Smartcard_Ex-T3", "CT6", "TSGC6"),
        _ => return None,
    };

    Some(TDocSource {
        root: root.to_string(),
        work_group_path: work_group_path.to_string(),
        work_group_code: work_group_code.to_string(),
        work_group_url: format!("https://www.3gpp.org/ftp/{root}/{work_group_path}/"),
        meeting_series_prefix: meeting_series_prefix.to_string(),
    })
}

pub fn direct_probe_url(source: &TDocSource, meeting_slug: &str, tdoc: &TDocKey) -> String {
    format!(
        "https://www.3gpp.org/ftp/{}/{}/{}/Docs/{}.zip",
        source.root, source.work_group_path, meeting_slug, tdoc.key
    )
}
