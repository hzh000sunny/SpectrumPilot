# 3GPP Fast Lookup, Download, Extract, and Open Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the next 3GPP workflow so a user can enter a specification or contribution query, see real progress, cancel the task, and have SpectrumPilot download, extract, and open the resolved document.

**Architecture:** Put query parsing and URL construction in `crates/3gpp-core`. Keep network, download, extraction, opening, progress events, and cancellation in the Tauri desktop bridge. Keep the React page focused on job control, progress display, advanced search scope, lookup rules, and candidate/result display.

**Tech Stack:** Rust, Tauri 2, React, TypeScript, Ant Design, `reqwest`, `zip`, `tokio-util`, `futures-util`, Vitest, Cargo tests.

---

## File Structure

| Path | Responsibility |
|---|---|
| `crates/3gpp-core/src/query.rs` | Parse user input into specification or contribution query models |
| `crates/3gpp-core/src/specs.rs` | Build spec archive paths, parse version codes, select latest versions |
| `crates/3gpp-core/src/tdoc.rs` | Complete contribution source mapping, meeting hints, and direct probe URL construction |
| `crates/3gpp-core/src/lib.rs` | Export new core modules |
| `crates/3gpp-core/tests/query_tests.rs` | Query parser tests |
| `crates/3gpp-core/tests/specs_tests.rs` | Specification version and URL tests |
| `crates/3gpp-core/tests/tdoc_tests.rs` | Contribution mapping and direct probe URL tests |
| `apps/desktop/src-tauri/Cargo.toml` | Add extraction, cancellation, and streaming dependencies |
| `apps/desktop/src-tauri/src/gpp/mod.rs` | Tauri command registration helpers and public command functions |
| `apps/desktop/src-tauri/src/gpp/jobs.rs` | Job registry, progress event payloads, and cancellation |
| `apps/desktop/src-tauri/src/gpp/workflow.rs` | Lookup workflow orchestration |
| `apps/desktop/src-tauri/src/gpp/download.rs` | Download, extract, document selection, and open target handling |
| `apps/desktop/src-tauri/src/lib.rs` | Wire the new GPP module into the Tauri invoke handler |
| `apps/desktop/src/api/gppCatalog.ts` | Typed frontend wrappers for lookup jobs and events |
| `apps/desktop/src/pages/GppPage.tsx` | Search mode, advanced scope, lookup rules, progress modal, candidate/result UI |
| `apps/desktop/src/pages/GppPage.test.tsx` | Frontend workflow and modal tests |
| `docs/v0.1/features/3gpp-search-download.md` | Update implemented behavior after the workflow lands |

## Task 1: Core Query Parser

**Files:**
- Create: `crates/3gpp-core/src/query.rs`
- Modify: `crates/3gpp-core/src/lib.rs`
- Test: `crates/3gpp-core/tests/query_tests.rs`

- [ ] **Step 1: Write failing tests for supported user inputs**

Add `crates/3gpp-core/tests/query_tests.rs`:

```rust
use spectrum_3gpp_core::query::{parse_gpp_query, GppQuery};

#[test]
fn parses_spec_queries() {
    assert!(matches!(
        parse_gpp_query("38.321").expect("query"),
        GppQuery::Specification(spec)
            if spec.spec_number == "38.321" && spec.version_prefix.is_none() && spec.exact_version.is_none()
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
        parse_gpp_query("38.101-1 j50").expect("query"),
        GppQuery::Specification(spec)
            if spec.spec_number == "38.101-1" && spec.archive_stem == "38101-1" && spec.exact_version.as_deref() == Some("j50")
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
            if tdoc.tdoc.key == "R2-2601401" && tdoc.meeting_hint.as_deref() == Some("TSGR2_133bis")
    ));
    assert!(matches!(
        parse_gpp_query("R2-2601401 133bis").expect("query"),
        GppQuery::Contribution(tdoc)
            if tdoc.tdoc.key == "R2-2601401" && tdoc.meeting_hint.as_deref() == Some("133bis")
    ));
    assert!(matches!(
        parse_gpp_query("R2-2601401 from TSGR2_120").expect("query"),
        GppQuery::Contribution(tdoc)
            if tdoc.tdoc.key == "R2-2601401" && tdoc.start_meeting.as_deref() == Some("TSGR2_120")
    ));
}
```

- [ ] **Step 2: Run tests and verify failure**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core --test query_tests
```

Expected: fail because `query` module and `parse_gpp_query` do not exist.

- [ ] **Step 3: Implement query models and parser**

Add `crates/3gpp-core/src/query.rs`:

```rust
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
    let version = parts.get(1).copied().or(inline_version).map(str::to_ascii_lowercase);
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

fn split_spec_and_inline_version(value: &str) -> Option<(String, Option<&str>)> {
    let normalized = value.trim().to_ascii_lowercase();
    let (raw_spec, inline_version) = match normalized.rsplit_once('-') {
        Some((left, right)) if is_version_code(right) => (left, Some(right)),
        _ => (normalized.as_str(), None),
    };
    let spec_number = normalize_spec_number(raw_spec)?;
    Some((spec_number, inline_version))
}

fn normalize_spec_number(value: &str) -> Option<String> {
    if value.contains('.') {
        return Some(value.to_string());
    }
    let mut chars = value.chars();
    let first = chars.next()?;
    let second = chars.next()?;
    if !first.is_ascii_digit() || !second.is_ascii_digit() {
        return None;
    }
    let rest = chars.collect::<String>();
    if rest.is_empty() {
        return None;
    }
    Some(format!("{first}{second}.{rest}"))
}

fn is_version_code(value: &str) -> bool {
    value.len() >= 2
        && value.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && value.chars().skip(1).all(|c| c.is_ascii_alphanumeric())
}
```

Update `crates/3gpp-core/src/lib.rs`:

```rust
pub mod query;
```

- [ ] **Step 4: Run tests and verify pass**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core --test query_tests
```

Expected: pass.

## Task 2: Contribution Source Mapping and Direct Probe URLs

**Files:**
- Create: `crates/3gpp-core/src/tdoc.rs`
- Modify: `crates/3gpp-core/src/lib.rs`
- Modify: `crates/3gpp-core/src/normalize.rs`
- Test: `crates/3gpp-core/tests/tdoc_tests.rs`

- [ ] **Step 1: Write failing tests for full source mapping**

Add `crates/3gpp-core/tests/tdoc_tests.rs`:

```rust
use spectrum_3gpp_core::normalize::normalize_tdoc_query;
use spectrum_3gpp_core::tdoc::{direct_probe_url, source_for_tdoc_prefix};

#[test]
fn maps_plenary_and_workgroup_prefixes() {
    let cases = [
        ("RP", "tsg_ran", "TSG_RAN", "TSGR"),
        ("R2", "tsg_ran", "WG2_RL2", "TSGR2"),
        ("SP", "tsg_sa", "TSG_SA", "TSGS"),
        ("S2", "tsg_sa", "WG2_Arch", "TSGS2"),
        ("CP", "tsg_ct", "TSG_CT", "TSGC"),
        ("C2", "tsg_ct", "WG2_capability_ex-T2", "TSGC2"),
        ("C5", "tsg_ct", "WG5_osa_ex-CN5", "TSGC5"),
    ];

    for (prefix, root, path, series) in cases {
        let source = source_for_tdoc_prefix(prefix).expect(prefix);
        assert_eq!(source.root, root);
        assert_eq!(source.work_group_path, path);
        assert_eq!(source.meeting_series_prefix, series);
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
```

- [ ] **Step 2: Run tests and verify failure**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core --test tdoc_tests
```

Expected: fail because `tdoc` module does not exist.

- [ ] **Step 3: Implement source mapping and direct URL builder**

Add `crates/3gpp-core/src/tdoc.rs`:

```rust
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
```

Update `crates/3gpp-core/src/lib.rs`:

```rust
pub mod tdoc;
```

Update `infer_tdoc_sources` in `crates/3gpp-core/src/normalize.rs` to delegate to `tdoc::source_for_tdoc_prefix`.

- [ ] **Step 4: Run tests and verify pass**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core --test tdoc_tests
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core --test normalize_tests
```

Expected: both pass.

## Task 3: Specification Archive Version Logic

**Files:**
- Create: `crates/3gpp-core/src/specs.rs`
- Modify: `crates/3gpp-core/src/lib.rs`
- Test: `crates/3gpp-core/tests/specs_tests.rs`

- [ ] **Step 1: Write failing tests for spec URLs and version selection**

Add `crates/3gpp-core/tests/specs_tests.rs`:

```rust
use spectrum_3gpp_core::query::parse_gpp_query;
use spectrum_3gpp_core::specs::{
    archive_directory_url, archive_file_name, select_latest_spec_file, SpecVersion,
};

#[test]
fn builds_archive_directory_and_exact_file_name() {
    let query = match parse_gpp_query("38.321 f10").expect("query") {
        spectrum_3gpp_core::query::GppQuery::Specification(spec) => spec,
        _ => panic!("expected spec"),
    };

    assert_eq!(
        archive_directory_url(&query),
        "https://www.3gpp.org/ftp/Specs/archive/38_series/38.321/"
    );
    assert_eq!(archive_file_name(&query, "f10"), "38321-f10.zip");
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
    assert_eq!(SpecVersion::parse("f10").expect("version").release_letter, 'f');
    assert!(SpecVersion::parse("j30").expect("j30") > SpecVersion::parse("i90").expect("i90"));
}
```

- [ ] **Step 2: Run tests and verify failure**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core --test specs_tests
```

Expected: fail because `specs` module does not exist.

- [ ] **Step 3: Implement spec archive helpers**

Add `crates/3gpp-core/src/specs.rs` with:

```rust
use crate::query::SpecificationQuery;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
    format!("{}-{}.zip", query.archive_stem, version.to_ascii_lowercase())
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
```

Update `crates/3gpp-core/src/lib.rs`:

```rust
pub mod specs;
```

- [ ] **Step 4: Run tests and verify pass**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core --test specs_tests
```

Expected: pass.

## Task 4: Desktop Job Types and Cancellation Skeleton

**Files:**
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Create: `apps/desktop/src-tauri/src/gpp/mod.rs`
- Create: `apps/desktop/src-tauri/src/gpp/jobs.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Test: `apps/desktop/src-tauri/src/gpp/jobs.rs`

- [ ] **Step 1: Write failing tests for job registry cancellation**

Create `apps/desktop/src-tauri/src/gpp/jobs.rs` with test module first:

```rust
#[cfg(test)]
mod tests {
    use super::JobRegistry;

    #[test]
    fn cancel_marks_existing_job_token() {
        let registry = JobRegistry::default();
        let job = registry.create_job();
        assert!(!job.token.is_cancelled());

        assert!(registry.cancel_job(job.id));
        assert!(job.token.is_cancelled());
    }

    #[test]
    fn cancel_unknown_job_returns_false() {
        let registry = JobRegistry::default();
        assert!(!registry.cancel_job("missing".to_string()));
    }
}
```

- [ ] **Step 2: Run tests and verify failure**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml cancel_marks_existing_job_token
```

Expected: fail because `JobRegistry` is not implemented.

- [ ] **Step 3: Add dependencies and implement registry**

Update `apps/desktop/src-tauri/Cargo.toml`:

```toml
tokio-util = "0.7"
uuid = { version = "1", features = ["v4", "serde"] }
```

Implement `apps/desktop/src-tauri/src/gpp/jobs.rs`:

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct LookupJob {
    pub id: String,
    pub token: CancellationToken,
}

#[derive(Debug, Default, Clone)]
pub struct JobRegistry {
    jobs: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl JobRegistry {
    pub fn create_job(&self) -> LookupJob {
        let id = Uuid::new_v4().to_string();
        let token = CancellationToken::new();
        self.jobs.lock().expect("jobs lock").insert(id.clone(), token.clone());
        LookupJob { id, token }
    }

    pub fn cancel_job(&self, id: String) -> bool {
        let Some(token) = self.jobs.lock().expect("jobs lock").remove(&id) else {
            return false;
        };
        token.cancel();
        true
    }

    pub fn finish_job(&self, id: &str) {
        self.jobs.lock().expect("jobs lock").remove(id);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GppLookupProgress {
    pub job_id: String,
    pub stage: String,
    pub message: String,
    pub progress: Option<u8>,
    pub searched_url_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GppLookupJobStarted {
    pub job_id: String,
}
```

Create `apps/desktop/src-tauri/src/gpp/mod.rs`:

```rust
pub mod jobs;
```

Update `apps/desktop/src-tauri/src/lib.rs`:

```rust
mod gpp;
```

- [ ] **Step 4: Run tests and verify pass**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml cancel_marks_existing_job_token
```

Expected: pass.

## Task 5: Download, Extract, and Open Target Selection

**Files:**
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Create: `apps/desktop/src-tauri/src/gpp/download.rs`
- Modify: `apps/desktop/src-tauri/src/gpp/mod.rs`
- Test: `apps/desktop/src-tauri/src/gpp/download.rs`

- [ ] **Step 1: Write failing tests for document selection**

Add tests in `download.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::choose_open_target;

    #[test]
    fn chooses_exact_docx_before_other_documents() {
        let files = vec![
            "R2-2601401.pdf".into(),
            "R2-2601401.docx".into(),
            "other.docx".into(),
        ];
        assert_eq!(
            choose_open_target("R2-2601401", &files).as_deref(),
            Some("R2-2601401.docx")
        );
    }

    #[test]
    fn returns_none_when_multiple_documents_are_ambiguous() {
        let files = vec!["a.docx".into(), "b.docx".into()];
        assert_eq!(choose_open_target("R2-2601401", &files), None);
    }
}
```

- [ ] **Step 2: Run tests and verify failure**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml chooses_exact_docx_before_other_documents
```

Expected: fail because `choose_open_target` does not exist.

- [ ] **Step 3: Add dependency and implement selection helper**

Update `apps/desktop/src-tauri/Cargo.toml`:

```toml
zip = "2"
futures-util = "0.3"
```

Add `apps/desktop/src-tauri/src/gpp/download.rs`:

```rust
pub fn choose_open_target(stem: &str, relative_files: &[String]) -> Option<String> {
    for extension in ["docx", "doc", "pdf"] {
        let exact = format!("{stem}.{extension}");
        if relative_files.iter().any(|file| file.eq_ignore_ascii_case(&exact)) {
            return Some(exact);
        }
    }

    for extension in ["docx", "doc", "pdf"] {
        let matches = relative_files
            .iter()
            .filter(|file| file.to_ascii_lowercase().ends_with(&format!(".{extension}")))
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            return Some(matches[0].to_string());
        }
    }

    None
}
```

Update `apps/desktop/src-tauri/src/gpp/mod.rs`:

```rust
pub mod download;
pub mod jobs;
```

- [ ] **Step 4: Run tests and verify pass**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml chooses_exact_docx_before_other_documents
```

Expected: pass.

## Task 6: Workflow Command Skeleton and Frontend API

**Files:**
- Create: `apps/desktop/src-tauri/src/gpp/workflow.rs`
- Modify: `apps/desktop/src-tauri/src/gpp/mod.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src/api/gppCatalog.ts`
- Test: `apps/desktop/src/pages/GppPage.test.tsx`

- [ ] **Step 1: Write failing frontend test for job start and progress modal**

In `apps/desktop/src/pages/GppPage.test.tsx`, add:

```tsx
it("starts a lookup job and shows a cancellable progress modal", async () => {
  const user = userEvent.setup();
  invokeMock.mockImplementation((command: string) => {
    if (command === "gpp_catalog_status") {
      return Promise.resolve({
        catalogRoot: "Preview only",
        manifestCount: 0,
        recordCount: 0,
        indexCount: 0,
        lastCheckedAt: null,
      });
    }
    if (command === "start_gpp_lookup_job") {
      return Promise.resolve({ jobId: "job-1" });
    }
    if (command === "cancel_gpp_lookup_job") {
      return Promise.resolve(true);
    }
    return Promise.reject(new Error(`unexpected command: ${command}`));
  });

  render(<GppPage />);
  await user.type(await screen.findByRole("textbox", { name: /query/i }), "R2-2601401");
  await user.click(await screen.findByRole("button", { name: /find, download & open/i }));

  expect(await screen.findByRole("dialog", { name: /3gpp lookup progress/i })).toBeInTheDocument();
  expect(screen.getByText(/starting lookup/i)).toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: /close/i }));
  expect(invokeMock).toHaveBeenCalledWith("cancel_gpp_lookup_job", { jobId: "job-1" });
});
```

- [ ] **Step 2: Run test and verify failure**

Run:

```bash
npm test -- --run src/pages/GppPage.test.tsx
```

Expected: fail because the page has no job action or modal.

- [ ] **Step 3: Add Tauri command request/response types**

Add `apps/desktop/src-tauri/src/gpp/workflow.rs`:

```rust
use serde::{Deserialize, Serialize};

use super::jobs::{GppLookupJobStarted, JobRegistry};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GppLookupRequest {
    pub query: String,
    pub mode: String,
    pub work_group: Option<String>,
    pub meeting_hint: Option<String>,
    pub search_window: String,
    pub open_after_download: bool,
}

#[tauri::command]
pub async fn start_gpp_lookup_job(
    app: tauri::AppHandle,
    registry: tauri::State<'_, JobRegistry>,
    request: GppLookupRequest,
) -> Result<GppLookupJobStarted, String> {
    let job = registry.create_job();
    let job_id = job.id.clone();
    let token = job.token.clone();
    tauri::async_runtime::spawn(async move {
        let _ = (app, request, token);
    });
    Ok(GppLookupJobStarted { job_id })
}

#[tauri::command]
pub fn cancel_gpp_lookup_job(
    registry: tauri::State<'_, JobRegistry>,
    job_id: String,
) -> Result<bool, String> {
    Ok(registry.cancel_job(job_id))
}
```

Update `apps/desktop/src-tauri/src/gpp/mod.rs`:

```rust
pub mod download;
pub mod jobs;
pub mod workflow;
```

Update `apps/desktop/src-tauri/src/lib.rs`:

```rust
.manage(gpp::jobs::JobRegistry::default())
...
gpp::workflow::start_gpp_lookup_job,
gpp::workflow::cancel_gpp_lookup_job,
```

- [ ] **Step 4: Add frontend API wrappers**

Update `apps/desktop/src/api/gppCatalog.ts`:

```ts
export type GppLookupMode = "auto" | "specification" | "proposal";

export type GppLookupJobRequest = {
  query: string;
  mode: GppLookupMode;
  workGroup: string | null;
  meetingHint: string | null;
  searchWindow: "fast-recent" | "from-meeting" | "deep-search";
  openAfterDownload: boolean;
};

export type GppLookupJobStarted = {
  jobId: string;
};

export async function startGppLookupJob(
  request: GppLookupJobRequest,
): Promise<GppLookupJobStarted> {
  if (!isTauri()) {
    return { jobId: "preview-job" };
  }
  return invoke<GppLookupJobStarted>("start_gpp_lookup_job", { request });
}

export async function cancelGppLookupJob(jobId: string): Promise<boolean> {
  if (!isTauri()) {
    return true;
  }
  return invoke<boolean>("cancel_gpp_lookup_job", { jobId });
}
```

- [ ] **Step 5: Run desktop and frontend tests**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml
npm test -- --run src/pages/GppPage.test.tsx
```

Expected: Rust tests pass. Frontend test still fails until the UI is added in Task 7.

## Task 7: Progress Modal, Advanced Scope, and Lookup Rules UI

**Files:**
- Modify: `apps/desktop/src/pages/GppPage.tsx`
- Modify: `apps/desktop/src/pages/GppPage.test.tsx`
- Modify: `apps/desktop/src/App.css`

- [ ] **Step 1: Implement the page UI to satisfy the failing test**

Update the page to include:

```tsx
<Segmented
  value={lookupMode}
  onChange={(value) => setLookupMode(value as GppLookupMode)}
  options={[
    { label: "Auto", value: "auto" },
    { label: "Specification", value: "specification" },
    { label: "Proposal", value: "proposal" },
  ]}
/>
```

Change the input label to `Query`, change the primary button to `Find, Download & Open`, and add an Ant Design modal:

```tsx
<Modal
  title="3GPP Lookup Progress"
  open={progressModalOpen}
  onCancel={handleCancelJob}
  footer={null}
  maskClosable={false}
>
  <Progress percent={progressPercent} status={progressStatus} />
  <Steps current={currentStep} items={progressSteps} />
  <p>{progressMessage}</p>
</Modal>
```

Add a compact `Lookup Rules` section with the examples from the design document.

- [ ] **Step 2: Run frontend test and verify pass**

Run:

```bash
npm test -- --run src/pages/GppPage.test.tsx
```

Expected: pass.

- [ ] **Step 3: Run full frontend build**

Run:

```bash
npm run build
```

Expected: pass. Vite chunk-size warnings are acceptable.

## Task 8: Real Workflow Progress Events and Direct Probe

**Files:**
- Modify: `apps/desktop/src-tauri/src/gpp/workflow.rs`
- Modify: `apps/desktop/src-tauri/src/gpp/jobs.rs`
- Modify: `apps/desktop/src/api/gppCatalog.ts`
- Modify: `apps/desktop/src/pages/GppPage.tsx`
- Test: `apps/desktop/src-tauri/src/gpp/workflow.rs`

- [ ] **Step 1: Write failing Rust test for direct probe URL hit classification**

Add a pure helper test in `workflow.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::is_successful_exact_probe;

    #[test]
    fn exact_probe_requires_success_and_exact_file_name() {
        assert!(is_successful_exact_probe(
            200,
            "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip",
            "R2-2601401.zip"
        ));
        assert!(!is_successful_exact_probe(
            403,
            "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_132/Docs/R2-2601401.zip",
            "R2-2601401.zip"
        ));
        assert!(!is_successful_exact_probe(
            200,
            "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601402.zip",
            "R2-2601401.zip"
        ));
    }
}
```

- [ ] **Step 2: Run test and verify failure**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml exact_probe_requires_success_and_exact_file_name
```

Expected: fail because helper does not exist.

- [ ] **Step 3: Implement exact probe helper and progress emission**

Add helper:

```rust
pub fn is_successful_exact_probe(status: u16, url: &str, expected_file_name: &str) -> bool {
    if status != 200 {
        return false;
    }
    url.rsplit('/')
        .next()
        .is_some_and(|file_name| file_name.eq_ignore_ascii_case(expected_file_name))
}
```

In the workflow job, emit progress with:

```rust
app.emit(
    "gpp-job-progress",
    GppLookupProgress {
        job_id: job_id.clone(),
        stage: "probing".to_string(),
        message: format!("Probing {meeting_slug}"),
        progress: Some(percent),
        searched_url_count,
    },
)?;
```

Use `CancellationToken::is_cancelled()` before each candidate batch and before downloading.

- [ ] **Step 4: Add frontend event listener**

Use Tauri events in `gppCatalog.ts`:

```ts
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type GppLookupProgress = {
  jobId: string;
  stage: string;
  message: string;
  progress: number | null;
  searchedUrlCount: number;
};

export async function listenGppLookupProgress(
  handler: (event: GppLookupProgress) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) {
    return () => undefined;
  }
  return listen<GppLookupProgress>("gpp-job-progress", (event) => handler(event.payload));
}
```

- [ ] **Step 5: Run tests**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml exact_probe_requires_success_and_exact_file_name
npm test -- --run src/pages/GppPage.test.tsx
```

Expected: pass.

## Task 9: Specification Lookup and Download/Extract/Open Completion

**Files:**
- Modify: `apps/desktop/src-tauri/src/gpp/workflow.rs`
- Modify: `apps/desktop/src-tauri/src/gpp/download.rs`
- Modify: `apps/desktop/src/pages/GppPage.tsx`
- Test: `apps/desktop/src-tauri/src/gpp/download.rs`

- [ ] **Step 1: Write failing test for workspace extraction path**

Add test:

```rust
#[cfg(test)]
mod path_tests {
    use std::path::PathBuf;
    use super::{spec_extract_dir, tdoc_extract_dir};

    #[test]
    fn builds_stable_extract_directories() {
        let workspace = PathBuf::from("/tmp/SpectrumPilotWorkspace");
        assert_eq!(
            tdoc_extract_dir(&workspace, "RAN2", "TSGR2_133bis", "R2-2601401"),
            workspace.join("3gpp").join("tdocs").join("RAN2").join("TSGR2_133bis").join("R2-2601401")
        );
        assert_eq!(
            spec_extract_dir(&workspace, "38.321", "j30"),
            workspace.join("3gpp").join("specs").join("38.321").join("j30")
        );
    }
}
```

- [ ] **Step 2: Run test and verify failure**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml builds_stable_extract_directories
```

Expected: fail because path helpers do not exist.

- [ ] **Step 3: Implement path helpers and extraction routine**

Add:

```rust
use std::path::{Path, PathBuf};

pub fn tdoc_extract_dir(
    workspace_root: &Path,
    work_group: &str,
    meeting: &str,
    tdoc: &str,
) -> PathBuf {
    workspace_root.join("3gpp").join("tdocs").join(work_group).join(meeting).join(tdoc)
}

pub fn spec_extract_dir(workspace_root: &Path, spec_number: &str, version: &str) -> PathBuf {
    workspace_root.join("3gpp").join("specs").join(spec_number).join(version)
}
```

Implement extraction with `zip::ZipArchive`, writing files only under the target directory by using `enclosed_name()` and rejecting unsafe paths.

- [ ] **Step 4: Wire completion events**

Emit:

```rust
app.emit("gpp-job-complete", complete_payload)?;
```

Payload should include:

```rust
pub struct GppLookupComplete {
    pub job_id: String,
    pub query: String,
    pub source_url: String,
    pub zip_path: String,
    pub extracted_path: String,
    pub opened_path: Option<String>,
    pub message: String,
}
```

- [ ] **Step 5: Run tests**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml builds_stable_extract_directories
npm test -- --run
```

Expected: pass.

## Task 10: Full Verification and Live Smoke Tests

**Files:**
- All changed files

- [ ] **Step 1: Run core Rust tests**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core
```

Expected: all tests pass.

- [ ] **Step 2: Run desktop Rust tests**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml
```

Expected: all non-ignored tests pass.

- [ ] **Step 3: Run frontend tests**

Run from `apps/desktop`:

```bash
npm test -- --run
```

Expected: all tests pass.

- [ ] **Step 4: Run frontend build**

Run from `apps/desktop`:

```bash
npm run build
```

Expected: build passes. Vite chunk-size warnings are acceptable.

- [ ] **Step 5: Run live smoke tests explicitly**

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml live_online_search_finds_known_ran2_tdoc -- --ignored --nocapture
```

Add ignored live tests named:

```text
live_lookup_download_finds_known_ran2_tdoc
live_spec_lookup_finds_exact_38321_f10
live_spec_lookup_finds_latest_38321
```

Run:

```bash
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml live_lookup_download_finds_known_ran2_tdoc -- --ignored --nocapture
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml live_spec_lookup_finds_exact_38321_f10 -- --ignored --nocapture
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml live_spec_lookup_finds_latest_38321 -- --ignored --nocapture
```

Expected: proposal and spec exact version resolve to real 3GPP zip URLs. Latest spec query resolves to the highest version visible in the archive listing at test time.

- [ ] **Step 6: Capture screenshots**

With the dev server on `0.0.0.0:1420`, capture:

```text
tmp/screenshots/3gpp-fast-lookup-idle-1440x900.png
tmp/screenshots/3gpp-fast-lookup-progress-1440x900.png
tmp/screenshots/3gpp-fast-lookup-complete-1440x900.png
```

Expected: the main page has lookup rules, no internal diagnostics section, and a cancellable progress modal during jobs.

- [ ] **Step 7: Update docs**

Update `docs/v0.1/features/3gpp-search-download.md` to describe:

- specification lookup
- contribution direct probe
- progress modal
- cancellation
- download/extract/open
- multiple-candidate fallback

Expected: docs match the implemented behavior.
