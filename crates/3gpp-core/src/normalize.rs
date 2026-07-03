use crate::model::TDocKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TDocSource {
    pub root: String,
    pub work_group_path: String,
    pub work_group_code: String,
    pub work_group_url: String,
    pub meeting_series_prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMeetingSlug {
    pub series: Option<String>,
    pub number: Option<u32>,
    pub variant: Option<String>,
    pub location: Option<String>,
    pub scheduled_month: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkGroupInfo {
    pub code: Option<String>,
    pub label: Option<String>,
}

pub fn parse_size_bytes(raw: &str) -> Option<u64> {
    let raw = raw.trim();
    let mut parts = raw.split_whitespace();
    let value = parts.next()?.replace(',', ".");
    let unit = parts.next().map(|unit| unit.to_ascii_uppercase());
    if parts.next().is_some() {
        return None;
    }
    let number: f64 = value.parse().ok()?;
    let multiplier = match unit.as_deref().unwrap_or("B") {
        "B" => 1.0,
        "KB" => 1024.0,
        "MB" => 1024.0 * 1024.0,
        "GB" => 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((number * multiplier).round() as u64)
}

pub fn parse_tdoc_key(file_name: &str) -> Option<TDocKey> {
    let trimmed = file_name.trim();
    let stem = if trimmed.to_ascii_lowercase().ends_with(".zip") {
        trimmed.get(..trimmed.len().saturating_sub(4))?
    } else {
        trimmed
    };
    let (prefix, number_text) = stem.split_once('-')?;
    let prefix = prefix.to_ascii_uppercase();
    if prefix.is_empty() || number_text.is_empty() {
        return None;
    }
    if !is_likely_tdoc_prefix(&prefix) {
        return None;
    }
    if !matches!(number_text.len(), 5..=7) || !number_text.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let year_hint = number_text
        .get(0..2)
        .and_then(|yy| yy.parse::<u32>().ok())
        .map(|yy| if yy >= 90 { 1900 + yy } else { 2000 + yy });

    Some(TDocKey {
        key: format!("{prefix}-{number_text}"),
        prefix,
        number_text: number_text.to_string(),
        year_hint,
    })
}

pub fn normalize_tdoc_query(query: &str) -> Option<TDocKey> {
    parse_tdoc_key(query)
}

pub fn infer_tdoc_sources(tdoc: &TDocKey) -> Vec<TDocSource> {
    crate::tdoc::source_for_tdoc_prefix(&tdoc.prefix)
        .into_iter()
        .collect()
}

fn is_likely_tdoc_prefix(prefix: &str) -> bool {
    match prefix {
        "CP" | "GP" | "NP" | "RP" | "SP" | "TP" => true,
        _ => {
            let bytes = prefix.as_bytes();
            if bytes.len() != 2 {
                return false;
            }

            let family = bytes[0];
            let number = bytes[1];
            if !number.is_ascii_digit() {
                return false;
            }

            let digit = number - b'0';
            match family {
                b'R' | b'S' | b'C' => (1..=6).contains(&digit),
                b'T' => (1..=3).contains(&digit),
                b'N' => (1..=5).contains(&digit),
                _ => false,
            }
        }
    }
}

pub fn parse_meeting_slug(slug: &str) -> ParsedMeetingSlug {
    let mut result = ParsedMeetingSlug {
        series: None,
        number: None,
        variant: None,
        location: None,
        scheduled_month: None,
    };

    let mut parts = slug.split('_');
    let Some(series) = parts.next() else {
        return result;
    };
    let Some(number_part) = parts.next() else {
        return result;
    };

    result.series = Some(series.to_string());

    let digits: String = number_part
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if !digits.is_empty() {
        result.number = digits.parse().ok();
    }

    let variant: String = number_part
        .chars()
        .skip_while(|c| c.is_ascii_digit())
        .collect();
    if !variant.is_empty() {
        result.variant = Some(variant);
    }

    let rest: Vec<&str> = parts.collect();
    if let Some(last) = rest.last() {
        if last.len() == 7
            && last.as_bytes().get(4) == Some(&b'-')
            && last.chars().filter(|c| c.is_ascii_digit()).count() == 6
        {
            result.scheduled_month = Some((*last).to_string());
            if rest.len() > 1 {
                result.location = Some(rest[..rest.len() - 1].join("_"));
            }
            return result;
        }
    }

    if !rest.is_empty() {
        result.location = Some(rest.join("_"));
    }

    result
}

pub fn infer_work_group(root: &str, work_group_path: &str) -> WorkGroupInfo {
    let code = match (root, work_group_path) {
        ("tsg_ran", "WG1_RL1") => Some("RAN1"),
        ("tsg_ran", "WG2_RL2") => Some("RAN2"),
        ("tsg_ran", "WG3_Iu") => Some("RAN3"),
        ("tsg_ran", "WG4_Radio") => Some("RAN4"),
        ("tsg_ran", "WG5_Test_ex-T1") => Some("RAN5"),
        ("tsg_sa", "WG1_Serv") => Some("SA1"),
        ("tsg_sa", "WG2_Arch") => Some("SA2"),
        ("tsg_sa", "WG3_Security") => Some("SA3"),
        ("tsg_sa", "WG4_CODEC") => Some("SA4"),
        ("tsg_sa", "WG5_TM") => Some("SA5"),
        ("tsg_sa", "WG6_MissionCritical") => Some("SA6"),
        ("tsg_ct", "WG1_mm-cc-sm_ex-CN1") => Some("CT1"),
        ("tsg_ct", "WG3_interworking_ex-CN3") => Some("CT3"),
        ("tsg_ct", "WG4_protocollars_ex-CN4") => Some("CT4"),
        ("tsg_ct", "WG6_Smartcard_Ex-T3") => Some("CT6"),
        _ => None,
    }
    .map(str::to_string);

    let label = code.as_ref().map(|c| {
        let (family, number) = c.split_at(c.len() - 1);
        format!("{family} WG{number}")
    });

    WorkGroupInfo { code, label }
}
