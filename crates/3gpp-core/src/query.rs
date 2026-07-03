use crate::model::TDocKey;
use crate::normalize::normalize_tdoc_query;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GppQuery {
    Specification(SpecificationQuery),
    Contribution(ContributionQuery),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecificationQuery {
    pub spec_number: String,
    pub archive_stem: String,
    pub series: String,
    pub version_prefix: Option<String>,
    pub exact_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContributionQuery {
    pub tdoc: TDocKey,
    pub meeting_hint: Option<String>,
    pub start_meeting: Option<String>,
}

pub fn parse_gpp_query(input: &str) -> Option<GppQuery> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    parse_contribution_query(trimmed)
        .map(GppQuery::Contribution)
        .or_else(|| parse_specification_query(trimmed).map(GppQuery::Specification))
}

fn parse_contribution_query(input: &str) -> Option<ContributionQuery> {
    let parts = input.split_whitespace().collect::<Vec<_>>();
    let first = parts.first()?;
    let tdoc = normalize_tdoc_query(first)?;
    let mut meeting_hint = None;
    let mut start_meeting = None;

    if parts.len() >= 2 {
        if parts[1].eq_ignore_ascii_case("from") && parts.len() >= 3 {
            start_meeting = Some(parts[2].to_string());
        } else {
            meeting_hint = Some(parts[1].to_string());
        }
    }

    Some(ContributionQuery {
        tdoc,
        meeting_hint,
        start_meeting,
    })
}

fn parse_specification_query(input: &str) -> Option<SpecificationQuery> {
    let parts = input.split_whitespace().collect::<Vec<_>>();
    let first = parts.first()?.trim();
    let (spec_number, inline_version) = split_spec_and_inline_version(first)?;
    let version = parts
        .get(1)
        .copied()
        .or(inline_version.as_deref())
        .map(str::to_ascii_lowercase);
    let archive_stem = spec_number.replace('.', "");
    let series = spec_number.get(0..2)?.to_string();
    let (version_prefix, exact_version) = match version {
        Some(value) if value.len() == 1 => (Some(value), None),
        Some(value) => (Some(value[0..1].to_string()), Some(value)),
        None => (None, None),
    };

    Some(SpecificationQuery {
        spec_number,
        archive_stem,
        series,
        version_prefix,
        exact_version,
    })
}

fn split_spec_and_inline_version(value: &str) -> Option<(String, Option<String>)> {
    let normalized = value.trim().to_ascii_lowercase();
    let (raw_spec, inline_version) = match normalized.rsplit_once('-') {
        Some((left, right)) if is_version_code(right) => (left, Some(right.to_string())),
        _ => (normalized.as_str(), None),
    };
    let spec_number = normalize_spec_number(raw_spec)?;
    Some((spec_number, inline_version))
}

fn normalize_spec_number(value: &str) -> Option<String> {
    if value.contains('.') {
        return valid_spec_number(value).then(|| value.to_string());
    }

    let mut chars = value.chars();
    let first = chars.next()?;
    let second = chars.next()?;
    if !first.is_ascii_digit() || !second.is_ascii_digit() {
        return None;
    }
    let rest = chars.collect::<String>();
    if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit() || c == '-') {
        return None;
    }
    Some(format!("{first}{second}.{rest}"))
}

fn valid_spec_number(value: &str) -> bool {
    let Some((series, rest)) = value.split_once('.') else {
        return false;
    };
    series.len() == 2
        && series.chars().all(|c| c.is_ascii_digit())
        && !rest.is_empty()
        && rest.chars().all(|c| c.is_ascii_digit() || c == '-')
}

fn is_version_code(value: &str) -> bool {
    value.len() >= 2
        && value
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
        && value.chars().skip(1).all(|c| c.is_ascii_alphanumeric())
}
