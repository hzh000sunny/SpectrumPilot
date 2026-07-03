use crate::query::SpecificationQuery;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpecVersion {
    pub release_letter: char,
    pub major: u32,
    pub minor: u32,
}

impl SpecVersion {
    pub fn parse(value: &str) -> Option<Self> {
        let mut chars = value.chars();
        let release_letter = chars.next()?.to_ascii_lowercase();
        if !release_letter.is_ascii_alphabetic() {
            return None;
        }

        let rest = chars.collect::<String>();
        if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_alphanumeric()) {
            return None;
        }

        let major = rest
            .chars()
            .next()
            .and_then(|c| c.to_digit(36))
            .unwrap_or(0);
        let minor = rest
            .chars()
            .nth(1)
            .and_then(|c| c.to_digit(36))
            .unwrap_or(0);

        Some(Self {
            release_letter,
            major,
            minor,
        })
    }
}

pub fn archive_directory_url(query: &SpecificationQuery) -> String {
    format!(
        "https://www.3gpp.org/ftp/Specs/archive/{}_series/{}/",
        query.series, query.spec_number
    )
}

pub fn archive_file_name(query: &SpecificationQuery, version: &str) -> String {
    format!(
        "{}-{}.zip",
        query.archive_stem,
        version.to_ascii_lowercase()
    )
}

pub fn select_latest_spec_file(
    archive_stem: &str,
    version_prefix: Option<&str>,
    files: &[String],
) -> Option<String> {
    let prefix = version_prefix.map(str::to_ascii_lowercase);
    files
        .iter()
        .filter_map(|file| {
            let version = file
                .strip_prefix(&format!("{archive_stem}-"))?
                .strip_suffix(".zip")?;
            if let Some(prefix) = &prefix {
                if !version.starts_with(prefix) {
                    return None;
                }
            }
            Some((SpecVersion::parse(version)?, file.clone()))
        })
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, file)| file)
}
