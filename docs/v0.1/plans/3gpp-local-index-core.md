# 3GPP Local Index Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a tested Rust core that parses current 3GPP directory listings, normalizes meeting and TDoc records, writes sharded JSON catalog files, and resolves proposal numbers from local indexes.

**Architecture:** Start with the Rust core only, without Tauri or React UI. The core is split into parsing, normalization, manifest generation, catalog storage, indexing, and resolution layers. Network access is isolated behind a fetcher interface so most behavior is tested with fixtures.

**Tech Stack:** Rust workspace, Cargo, `serde`, `serde_json`, `scraper`, `url`, `sha2`, `chrono`, `thiserror`, `tempfile`, optional `reqwest` + `tokio` for live fetch smoke tests.

---

## File Structure

Create this implementation shape:

```text
Cargo.toml
crates/
  3gpp-core/
    Cargo.toml
    src/
      lib.rs
      error.rs
      model.rs
      normalize.rs
      parser.rs
      manifest.rs
      catalog.rs
      index.rs
      resolver.rs
      fetch.rs
    tests/
      parser_tests.rs
      normalize_tests.rs
      manifest_tests.rs
      catalog_tests.rs
      resolver_tests.rs
      fixtures/
        meeting-root-ran2.html
        docs-ran2.html
        docs-sa2.html
        docs-ct1-with-artifact.html
```

The folder name remains `3gpp-core` to match the product wording, but the Rust package should be `spectrum-3gpp-core` and the library crate should be `spectrum_3gpp_core` because Rust crate identifiers cannot start with a digit.

## Task 1: Rust Workspace And Core Crate

**Files:**
- Create: `Cargo.toml`
- Create: `crates/3gpp-core/Cargo.toml`
- Create: `crates/3gpp-core/src/lib.rs`
- Create: `crates/3gpp-core/src/error.rs`

- [ ] **Step 1: Create the workspace manifest**

Create `Cargo.toml`:

```toml
[workspace]
members = [
  "crates/3gpp-core"
]
resolver = "2"
```

- [ ] **Step 2: Create the core crate manifest**

Create `crates/3gpp-core/Cargo.toml`:

```toml
[package]
name = "spectrum-3gpp-core"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"

[lib]
name = "spectrum_3gpp_core"
path = "src/lib.rs"

[dependencies]
chrono = { version = "0.4", features = ["serde"] }
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"], optional = true }
scraper = "0.20"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
thiserror = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"], optional = true }
url = "2"

[dev-dependencies]
tempfile = "3"

[features]
default = []
live-fetch = ["dep:reqwest", "dep:tokio"]
```

- [ ] **Step 3: Add the crate module boundary**

Create `crates/3gpp-core/src/lib.rs`:

```rust
pub mod error;

pub use error::{GppError, Result};
```

Task 1 only exposes the `error` module so the crate compiles before the later
parser, model, and storage files exist. Add the remaining module declarations in
the later tasks when their source files are created.

- [ ] **Step 4: Add the shared error type**

Create `crates/3gpp-core/src/error.rs`:

```rust
use thiserror::Error;

pub type Result<T> = std::result::Result<T, GppError>;

#[derive(Debug, Error)]
pub enum GppError {
    #[error("invalid 3GPP URL: {0}")]
    InvalidUrl(String),

    #[error("failed to parse directory listing: {0}")]
    Parse(String),

    #[error("failed to read or write catalog file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to serialize catalog JSON: {0}")]
    Json(#[from] serde_json::Error),
}
```

- [ ] **Step 5: Verify the workspace compiles**

Run:

```bash
cargo test --workspace
```

Expected: PASS with zero tests.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/3gpp-core
git commit -m "feat: add 3GPP core crate"
```

## Task 2: Domain Model And Stable IDs

**Files:**
- Create: `crates/3gpp-core/src/model.rs`
- Modify: `crates/3gpp-core/src/lib.rs`
- Test: `crates/3gpp-core/tests/normalize_tests.rs`

- [ ] **Step 1: Write failing tests for stable IDs**

Create `crates/3gpp-core/tests/normalize_tests.rs`:

```rust
use spectrum_3gpp_core::model::{FileRecord, MeetingRecord};

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

    assert!(id.starts_with("file-url-sha256:"));
    assert_eq!(id.len(), "file-url-sha256:".len() + 64);
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test -p spectrum-3gpp-core --test normalize_tests
```

Expected: FAIL because `model` types do not exist yet.

- [ ] **Step 3: Implement the domain model**

Modify `crates/3gpp-core/src/lib.rs`:

```rust
pub mod error;
pub mod model;

pub use error::{GppError, Result};
```

Create `crates/3gpp-core/src/model.rs`:

```rust
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectoryRole {
    Root,
    WorkGroup,
    MeetingRoot,
    Docs,
    Inbox,
    Report,
    Agenda,
    Auxiliary,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntryKind {
    Directory,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocsState {
    Unknown,
    Available,
    Empty,
    Missing,
    Forbidden,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryChild {
    pub name: String,
    pub kind: EntryKind,
    pub url: String,
    pub role: DirectoryRole,
    pub remote_modified_raw: Option<String>,
    pub size_raw: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryManifest {
    pub schema_version: u32,
    pub record_type: String,
    pub url: String,
    pub path_segments: Vec<String>,
    pub directory_role: DirectoryRole,
    pub checked_at: String,
    pub child_fingerprint: String,
    pub children: Vec<DirectoryChild>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeetingRecord {
    pub schema_version: u32,
    pub record_type: String,
    pub id: String,
    pub root: String,
    pub work_group_path: String,
    pub work_group_code: Option<String>,
    pub work_group_label: Option<String>,
    pub meeting_slug: String,
    pub meeting_series: Option<String>,
    pub meeting_number: Option<u32>,
    pub meeting_variant: Option<String>,
    pub location: Option<String>,
    pub scheduled_month: Option<String>,
    pub url: String,
    pub docs_url: Option<String>,
    pub docs_state: DocsState,
    pub last_seen_remote_modified_raw: Option<String>,
}

impl MeetingRecord {
    pub fn stable_id(root: &str, work_group_path: &str, meeting_slug: &str) -> String {
        format!("meeting:{root}/{work_group_path}/{meeting_slug}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TDocKey {
    pub key: String,
    pub prefix: String,
    pub number_text: String,
    pub year_hint: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileClassification {
    pub is_primary_tdoc: bool,
    pub is_zip: bool,
    pub is_ignored_artifact: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRecord {
    pub schema_version: u32,
    pub record_type: String,
    pub id: String,
    pub canonical_url: String,
    pub parent_directory_url: String,
    pub root: String,
    pub work_group_path: String,
    pub work_group_code: Option<String>,
    pub meeting_id: Option<String>,
    pub meeting_slug: Option<String>,
    pub container_role: DirectoryRole,
    pub file_name: String,
    pub extension: Option<String>,
    pub remote_modified_raw: Option<String>,
    pub size_raw: Option<String>,
    pub size_bytes: Option<u64>,
    pub tdoc: Option<TDocKey>,
    pub classification: FileClassification,
}

impl FileRecord {
    pub fn stable_id(canonical_url: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(canonical_url.as_bytes());
        format!("file-url-sha256:{:x}", hasher.finalize())
    }
}
```

- [ ] **Step 4: Run tests**

Run:

```bash
cargo test -p spectrum-3gpp-core --test normalize_tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/3gpp-core/src/model.rs crates/3gpp-core/tests/normalize_tests.rs
git commit -m "feat: add 3GPP domain model"
```

## Task 3: Normalization Rules

**Files:**
- Create: `crates/3gpp-core/src/normalize.rs`
- Modify: `crates/3gpp-core/src/lib.rs`
- Modify: `crates/3gpp-core/tests/normalize_tests.rs`

- [ ] **Step 1: Add failing normalization tests**

Append to `crates/3gpp-core/tests/normalize_tests.rs`:

```rust
use spectrum_3gpp_core::normalize::{
    infer_work_group, parse_meeting_slug, parse_size_bytes, parse_tdoc_key,
};

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
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test -p spectrum-3gpp-core --test normalize_tests
```

Expected: FAIL because normalization functions do not exist.

- [ ] **Step 3: Implement normalization**

Modify `crates/3gpp-core/src/lib.rs`:

```rust
pub mod error;
pub mod model;
pub mod normalize;

pub use error::{GppError, Result};
```

Create `crates/3gpp-core/src/normalize.rs`:

```rust
use crate::model::TDocKey;

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
    let unit = parts.next().unwrap_or("B").to_ascii_uppercase();
    let number: f64 = value.parse().ok()?;
    let multiplier = match unit.as_str() {
        "KB" => 1024.0,
        "MB" => 1024.0 * 1024.0,
        "GB" => 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    };
    Some((number * multiplier).round() as u64)
}

pub fn parse_tdoc_key(file_name: &str) -> Option<TDocKey> {
    let stem = file_name.strip_suffix(".zip").unwrap_or(file_name);
    let (prefix, number_text) = stem.split_once('-')?;
    if prefix.is_empty() || number_text.is_empty() {
        return None;
    }
    if !prefix.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    if !number_text.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let year_hint = number_text
        .get(0..2)
        .and_then(|yy| yy.parse::<u32>().ok())
        .map(|yy| if yy >= 90 { 1900 + yy } else { 2000 + yy });

    Some(TDocKey {
        key: format!("{prefix}-{number_text}"),
        prefix: prefix.to_string(),
        number_text: number_text.to_string(),
        year_hint,
    })
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

    let digits: String = number_part.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() {
        result.number = digits.parse().ok();
    }

    let variant: String = number_part.chars().skip_while(|c| c.is_ascii_digit()).collect();
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
```

- [ ] **Step 4: Run normalization tests**

Run:

```bash
cargo test -p spectrum-3gpp-core --test normalize_tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/3gpp-core/src/normalize.rs crates/3gpp-core/tests/normalize_tests.rs
git commit -m "feat: normalize 3GPP meeting and TDoc fields"
```

## Task 4: Directory Listing Parser

**Files:**
- Create: `crates/3gpp-core/src/parser.rs`
- Modify: `crates/3gpp-core/src/lib.rs`
- Create: `crates/3gpp-core/tests/parser_tests.rs`

- [ ] **Step 1: Write parser tests**

Create `crates/3gpp-core/tests/parser_tests.rs`:

```rust
use spectrum_3gpp_core::model::EntryKind;
use spectrum_3gpp_core::parser::parse_directory_listing;

const DOCS_HTML: &str = r#"
<table>
  <tbody>
    <tr>
      <td><input type="checkbox" value="R2-2601401.zip" /></td>
      <td><img src="/ftp/geticon.axd?file=.zip" /></td>
      <td><a class="file" href="https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip">R2-2601401.zip</a></td>
      <td>2026/04/03 9:50</td>
      <td>78,5 KB</td>
    </tr>
    <tr>
      <td></td>
      <td><img src="/ftp/geticon.axd?file=" /></td>
      <td><a href="https://www.3gpp.org/ftp/tsg_ct/WG1_mm-cc-sm_ex-CN1/TSGC1_161_Dalian/Docs/__MACOSX">__MACOSX</a></td>
      <td>2026/05/22 7:29</td>
      <td></td>
    </tr>
  </tbody>
</table>
"#;

#[test]
fn parses_file_and_directory_rows() {
    let rows = parse_directory_listing(DOCS_HTML).expect("parse");
    assert_eq!(rows.len(), 2);

    assert_eq!(rows[0].name, "R2-2601401.zip");
    assert_eq!(rows[0].kind, EntryKind::File);
    assert_eq!(rows[0].remote_modified_raw.as_deref(), Some("2026/04/03 9:50"));
    assert_eq!(rows[0].size_raw.as_deref(), Some("78,5 KB"));

    assert_eq!(rows[1].name, "__MACOSX");
    assert_eq!(rows[1].kind, EntryKind::Directory);
    assert_eq!(rows[1].size_raw, None);
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test -p spectrum-3gpp-core --test parser_tests
```

Expected: FAIL because `parse_directory_listing` does not exist.

- [ ] **Step 3: Implement parser**

Modify `crates/3gpp-core/src/lib.rs`:

```rust
pub mod error;
pub mod model;
pub mod normalize;
pub mod parser;

pub use error::{GppError, Result};
```

Create `crates/3gpp-core/src/parser.rs`:

```rust
use scraper::{Html, Selector};

use crate::error::{GppError, Result};
use crate::model::{EntryKind, DirectoryRole};
use crate::normalize::parse_size_bytes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDirectoryRow {
    pub name: String,
    pub url: String,
    pub kind: EntryKind,
    pub role: DirectoryRole,
    pub remote_modified_raw: Option<String>,
    pub size_raw: Option<String>,
    pub size_bytes: Option<u64>,
}

pub fn parse_directory_listing(html: &str) -> Result<Vec<ParsedDirectoryRow>> {
    let document = Html::parse_document(html);
    let row_selector = Selector::parse("tr").map_err(|e| GppError::Parse(e.to_string()))?;
    let cell_selector = Selector::parse("td").map_err(|e| GppError::Parse(e.to_string()))?;
    let anchor_selector = Selector::parse("a").map_err(|e| GppError::Parse(e.to_string()))?;

    let mut rows = Vec::new();

    for row in document.select(&row_selector) {
        let cells: Vec<_> = row.select(&cell_selector).collect();
        let Some(anchor) = row.select(&anchor_selector).find(|a| {
            a.value()
                .attr("href")
                .is_some_and(|href| href.starts_with("https://www.3gpp.org/ftp/"))
        }) else {
            continue;
        };

        let Some(url) = anchor.value().attr("href") else {
            continue;
        };

        let name = anchor.text().collect::<String>().trim().to_string();
        if name.is_empty() {
            continue;
        }

        let remote_modified_raw = cells
            .iter()
            .map(|cell| cell.text().collect::<String>().trim().to_string())
            .find(|text| text.contains('/') && text.contains(':'));

        let size_raw = cells
            .iter()
            .map(|cell| cell.text().collect::<String>().trim().to_string())
            .find(|text| text.contains("KB") || text.contains("MB") || text.contains("GB"));

        let kind = if size_raw.is_some() || name.contains('.') {
            EntryKind::File
        } else {
            EntryKind::Directory
        };

        rows.push(ParsedDirectoryRow {
            role: infer_directory_role(&name, &kind),
            name,
            url: url.to_string(),
            kind,
            remote_modified_raw,
            size_bytes: size_raw.as_deref().and_then(parse_size_bytes),
            size_raw,
        });
    }

    Ok(rows)
}

fn infer_directory_role(name: &str, kind: &EntryKind) -> DirectoryRole {
    if *kind == EntryKind::File {
        return DirectoryRole::Auxiliary;
    }

    match name.to_ascii_lowercase().as_str() {
        "docs" => DirectoryRole::Docs,
        "inbox" => DirectoryRole::Inbox,
        "report" => DirectoryRole::Report,
        "agenda" => DirectoryRole::Agenda,
        _ => DirectoryRole::Unknown,
    }
}
```

- [ ] **Step 4: Run parser tests**

Run:

```bash
cargo test -p spectrum-3gpp-core --test parser_tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/3gpp-core/src/parser.rs crates/3gpp-core/tests/parser_tests.rs
git commit -m "feat: parse 3GPP directory listings"
```

## Task 5: Manifest Builder And Fingerprint

**Files:**
- Create: `crates/3gpp-core/src/manifest.rs`
- Modify: `crates/3gpp-core/src/lib.rs`
- Create: `crates/3gpp-core/tests/manifest_tests.rs`

- [ ] **Step 1: Write manifest tests**

Create `crates/3gpp-core/tests/manifest_tests.rs`:

```rust
use spectrum_3gpp_core::manifest::build_manifest_from_html;
use spectrum_3gpp_core::model::{DirectoryRole, EntryKind};

const MEETING_HTML: &str = r#"
<table>
  <tr><td></td><td></td><td><a href="https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs">Docs</a></td><td>2026/06/25 9:59</td><td></td></tr>
  <tr><td></td><td></td><td><a href="https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Report">Report</a></td><td>2026/05/20 15:23</td><td></td></tr>
</table>
"#;

#[test]
fn builds_manifest_with_stable_fingerprint() {
    let manifest = build_manifest_from_html(
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/",
        DirectoryRole::MeetingRoot,
        "2026-07-01T00:00:00Z",
        MEETING_HTML,
    )
    .expect("manifest");

    assert_eq!(manifest.path_segments, vec!["tsg_ran", "WG2_RL2", "TSGR2_133bis"]);
    assert_eq!(manifest.children.len(), 2);
    assert_eq!(manifest.children[0].name, "Docs");
    assert_eq!(manifest.children[0].kind, EntryKind::Directory);
    assert_eq!(manifest.children[0].role, DirectoryRole::Docs);
    assert!(manifest.child_fingerprint.starts_with("sha256:"));
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test -p spectrum-3gpp-core --test manifest_tests
```

Expected: FAIL because `build_manifest_from_html` does not exist.

- [ ] **Step 3: Implement manifest builder**

Modify `crates/3gpp-core/src/lib.rs`:

```rust
pub mod error;
pub mod manifest;
pub mod model;
pub mod normalize;
pub mod parser;

pub use error::{GppError, Result};
```

Create `crates/3gpp-core/src/manifest.rs`:

```rust
use sha2::{Digest, Sha256};
use url::Url;

use crate::error::{GppError, Result};
use crate::model::{DirectoryChild, DirectoryManifest, DirectoryRole};
use crate::parser::parse_directory_listing;

pub fn build_manifest_from_html(
    directory_url: &str,
    directory_role: DirectoryRole,
    checked_at: &str,
    html: &str,
) -> Result<DirectoryManifest> {
    let parsed_url = Url::parse(directory_url)
        .map_err(|_| GppError::InvalidUrl(directory_url.to_string()))?;
    let path_segments = parsed_url
        .path_segments()
        .map(|segments| {
            segments
                .filter(|segment| !segment.is_empty() && *segment != "ftp")
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let children = parse_directory_listing(html)?
        .into_iter()
        .map(|row| DirectoryChild {
            name: row.name,
            kind: row.kind,
            url: row.url,
            role: row.role,
            remote_modified_raw: row.remote_modified_raw,
            size_raw: row.size_raw,
            size_bytes: row.size_bytes,
        })
        .collect::<Vec<_>>();

    let child_fingerprint = child_fingerprint(&children);

    Ok(DirectoryManifest {
        schema_version: 1,
        record_type: "directory-manifest".to_string(),
        url: directory_url.to_string(),
        path_segments,
        directory_role,
        checked_at: checked_at.to_string(),
        child_fingerprint,
        children,
    })
}

pub fn child_fingerprint(children: &[DirectoryChild]) -> String {
    let mut normalized = children
        .iter()
        .map(|child| {
            format!(
                "{}\t{:?}\t{}\t{}\t{}",
                child.name,
                child.kind,
                child.url,
                child.remote_modified_raw.as_deref().unwrap_or(""),
                child.size_raw.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>();
    normalized.sort();

    let mut hasher = Sha256::new();
    for item in normalized {
        hasher.update(item.as_bytes());
        hasher.update(b"\n");
    }

    format!("sha256:{:x}", hasher.finalize())
}
```

- [ ] **Step 4: Run manifest tests**

Run:

```bash
cargo test -p spectrum-3gpp-core --test manifest_tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/3gpp-core/src/manifest.rs crates/3gpp-core/tests/manifest_tests.rs
git commit -m "feat: build 3GPP directory manifests"
```

## Task 6: Domain Mapping

**Files:**
- Modify: `crates/3gpp-core/src/manifest.rs`
- Modify: `crates/3gpp-core/src/lib.rs`
- Create: `crates/3gpp-core/tests/domain_mapping_tests.rs`

- [ ] **Step 1: Write domain mapping tests**

Create `crates/3gpp-core/tests/domain_mapping_tests.rs`:

```rust
use spectrum_3gpp_core::manifest::{meeting_from_manifest, file_records_from_docs_manifest};
use spectrum_3gpp_core::model::{DirectoryRole, DocsState};

#[test]
fn maps_meeting_manifest_to_meeting_record() {
    let html = r#"
    <table>
      <tr><td></td><td></td><td><a href="https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs">Docs</a></td><td>2026/06/25 9:59</td><td></td></tr>
    </table>
    "#;
    let manifest = spectrum_3gpp_core::manifest::build_manifest_from_html(
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/",
        DirectoryRole::MeetingRoot,
        "2026-07-01T00:00:00Z",
        html,
    )
    .expect("manifest");

    let meeting = meeting_from_manifest(&manifest).expect("meeting");
    assert_eq!(meeting.id, "meeting:tsg_ran/WG2_RL2/TSGR2_133bis");
    assert_eq!(meeting.work_group_code.as_deref(), Some("RAN2"));
    assert_eq!(meeting.docs_state, DocsState::Available);
    assert_eq!(meeting.meeting_number, Some(133));
}

#[test]
fn maps_docs_manifest_to_primary_tdoc_files() {
    let html = r#"
    <table>
      <tr><td></td><td></td><td><a href="https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip">R2-2601401.zip</a></td><td>2026/04/03 9:50</td><td>78,5 KB</td></tr>
      <tr><td></td><td></td><td><a href="https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/__MACOSX">__MACOSX</a></td><td>2026/05/22 7:29</td><td></td></tr>
    </table>
    "#;
    let manifest = spectrum_3gpp_core::manifest::build_manifest_from_html(
        "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/",
        DirectoryRole::Docs,
        "2026-07-01T00:00:00Z",
        html,
    )
    .expect("manifest");

    let files = file_records_from_docs_manifest(&manifest).expect("files");
    assert_eq!(files.len(), 2);
    assert!(files[0].classification.is_primary_tdoc);
    assert_eq!(files[0].tdoc.as_ref().unwrap().key, "R2-2601401");
    assert!(files[1].classification.is_ignored_artifact);
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test -p spectrum-3gpp-core --test domain_mapping_tests
```

Expected: FAIL because mapping functions do not exist.

- [ ] **Step 3: Implement mapping functions**

Modify `crates/3gpp-core/src/lib.rs`:

```rust
pub mod error;
pub mod manifest;
pub mod model;
pub mod normalize;
pub mod parser;

pub use error::{GppError, Result};
```

Modify `crates/3gpp-core/src/manifest.rs` by adding functions that:

```rust
use crate::model::{
    DocsState, FileClassification, FileRecord, MeetingRecord, TDocKey,
};
use crate::normalize::{infer_work_group, parse_meeting_slug, parse_tdoc_key};
```

Add `meeting_from_manifest()`:

```rust
pub fn meeting_from_manifest(manifest: &DirectoryManifest) -> Result<MeetingRecord> {
    if manifest.path_segments.len() < 3 {
        return Err(GppError::Parse(format!(
            "meeting path needs at least 3 segments: {}",
            manifest.url
        )));
    }

    let root = manifest.path_segments[0].clone();
    let work_group_path = manifest.path_segments[1].clone();
    let meeting_slug = manifest.path_segments[2].clone();
    let parsed = parse_meeting_slug(&meeting_slug);
    let wg = infer_work_group(&root, &work_group_path);
    let docs_child = manifest
        .children
        .iter()
        .find(|child| child.role == DirectoryRole::Docs);
    let docs_state = if docs_child.is_some() {
        DocsState::Available
    } else {
        DocsState::Missing
    };

    Ok(MeetingRecord {
        schema_version: 1,
        record_type: "meeting".to_string(),
        id: MeetingRecord::stable_id(&root, &work_group_path, &meeting_slug),
        root,
        work_group_path,
        work_group_code: wg.code,
        work_group_label: wg.label,
        meeting_slug,
        meeting_series: parsed.series,
        meeting_number: parsed.number,
        meeting_variant: parsed.variant,
        location: parsed.location,
        scheduled_month: parsed.scheduled_month,
        url: manifest.url.clone(),
        docs_url: docs_child.map(|child| child.url.clone()),
        docs_state,
        last_seen_remote_modified_raw: None,
    })
}
```

Add `file_records_from_docs_manifest()`:

```rust
pub fn file_records_from_docs_manifest(manifest: &DirectoryManifest) -> Result<Vec<FileRecord>> {
    if manifest.path_segments.len() < 4 {
        return Err(GppError::Parse(format!(
            "docs path needs at least 4 segments: {}",
            manifest.url
        )));
    }

    let root = manifest.path_segments[0].clone();
    let work_group_path = manifest.path_segments[1].clone();
    let meeting_slug = manifest.path_segments[2].clone();
    let wg = infer_work_group(&root, &work_group_path);
    let meeting_id = MeetingRecord::stable_id(&root, &work_group_path, &meeting_slug);

    Ok(manifest
        .children
        .iter()
        .map(|child| {
            let extension = child
                .name
                .rsplit_once('.')
                .map(|(_, ext)| ext.to_ascii_lowercase());
            let is_zip = extension.as_deref() == Some("zip");
            let tdoc = if is_zip {
                parse_tdoc_key(&child.name)
            } else {
                None
            };
            let is_ignored_artifact = child.name == "__MACOSX";
            let is_primary_tdoc = is_zip && tdoc.is_some() && !is_ignored_artifact;

            FileRecord {
                schema_version: 1,
                record_type: if is_primary_tdoc {
                    "tdoc-file".to_string()
                } else {
                    "auxiliary-file".to_string()
                },
                id: FileRecord::stable_id(&child.url),
                canonical_url: child.url.clone(),
                parent_directory_url: manifest.url.clone(),
                root: root.clone(),
                work_group_path: work_group_path.clone(),
                work_group_code: wg.code.clone(),
                meeting_id: Some(meeting_id.clone()),
                meeting_slug: Some(meeting_slug.clone()),
                container_role: DirectoryRole::Docs,
                file_name: child.name.clone(),
                extension,
                remote_modified_raw: child.remote_modified_raw.clone(),
                size_raw: child.size_raw.clone(),
                size_bytes: child.size_bytes,
                tdoc,
                classification: FileClassification {
                    is_primary_tdoc,
                    is_zip,
                    is_ignored_artifact,
                },
            }
        })
        .collect())
}
```

- [ ] **Step 4: Run mapping tests**

Run:

```bash
cargo test -p spectrum-3gpp-core --test domain_mapping_tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/3gpp-core/src/manifest.rs crates/3gpp-core/tests/domain_mapping_tests.rs
git commit -m "feat: map 3GPP manifests to domain records"
```

## Task 7: Catalog Storage

**Files:**
- Create: `crates/3gpp-core/src/catalog.rs`
- Modify: `crates/3gpp-core/src/lib.rs`
- Create: `crates/3gpp-core/tests/catalog_tests.rs`

- [ ] **Step 1: Write catalog tests**

Create `crates/3gpp-core/tests/catalog_tests.rs`:

```rust
use spectrum_3gpp_core::catalog::CatalogPaths;

#[test]
fn builds_expected_catalog_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = CatalogPaths::new(temp.path());

    assert!(paths.root().ends_with("catalog"));
    assert!(paths.manifests_dir().ends_with("manifests"));
    assert!(paths.records_dir().ends_with("records"));
    assert!(paths.indexes_dir().ends_with("indexes"));
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test -p spectrum-3gpp-core --test catalog_tests
```

Expected: FAIL because `CatalogPaths` does not exist.

- [ ] **Step 3: Implement catalog paths and atomic JSON write**

Modify `crates/3gpp-core/src/lib.rs`:

```rust
pub mod catalog;
pub mod error;

pub use error::{GppError, Result};
```

Create `crates/3gpp-core/src/catalog.rs`:

```rust
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{GppError, Result};

#[derive(Debug, Clone)]
pub struct CatalogPaths {
    root: PathBuf,
}

impl CatalogPaths {
    pub fn new(app_cache_3gpp_dir: impl AsRef<Path>) -> Self {
        Self {
            root: app_cache_3gpp_dir.as_ref().join("catalog"),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifests_dir(&self) -> PathBuf {
        self.root.join("manifests")
    }

    pub fn records_dir(&self) -> PathBuf {
        self.root.join("records")
    }

    pub fn indexes_dir(&self) -> PathBuf {
        self.root.join("indexes")
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        for dir in [self.root(), &self.manifests_dir(), &self.records_dir(), &self.indexes_dir()] {
            fs::create_dir_all(dir).map_err(|source| GppError::Io {
                path: dir.display().to_string(),
                source,
            })?;
        }
        Ok(())
    }
}

pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| GppError::Io {
            path: parent.display().to_string(),
            source,
        })?;
    }

    let tmp_path = path.with_extension("tmp");
    let body = serde_json::to_vec_pretty(value)?;
    fs::write(&tmp_path, body).map_err(|source| GppError::Io {
        path: tmp_path.display().to_string(),
        source,
    })?;
    fs::rename(&tmp_path, path).map_err(|source| GppError::Io {
        path: path.display().to_string(),
        source,
    })?;
    Ok(())
}
```

- [ ] **Step 4: Run catalog tests**

Run:

```bash
cargo test -p spectrum-3gpp-core --test catalog_tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/3gpp-core/src/catalog.rs crates/3gpp-core/tests/catalog_tests.rs
git commit -m "feat: add 3GPP catalog storage paths"
```

## Task 8: Lookup Index And Resolver

**Files:**
- Create: `crates/3gpp-core/src/index.rs`
- Create: `crates/3gpp-core/src/resolver.rs`
- Modify: `crates/3gpp-core/src/lib.rs`
- Create: `crates/3gpp-core/tests/resolver_tests.rs`

- [ ] **Step 1: Write resolver tests**

Create `crates/3gpp-core/tests/resolver_tests.rs`:

```rust
use spectrum_3gpp_core::index::TDocLookupIndex;
use spectrum_3gpp_core::model::{
    DirectoryRole, FileClassification, FileRecord, TDocKey,
};
use spectrum_3gpp_core::resolver::resolve_tdoc;

fn sample_file() -> FileRecord {
    let url = "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/R2-2601401.zip";
    FileRecord {
        schema_version: 1,
        record_type: "tdoc-file".to_string(),
        id: FileRecord::stable_id(url),
        canonical_url: url.to_string(),
        parent_directory_url: "https://www.3gpp.org/ftp/tsg_ran/WG2_RL2/TSGR2_133bis/Docs/".to_string(),
        root: "tsg_ran".to_string(),
        work_group_path: "WG2_RL2".to_string(),
        work_group_code: Some("RAN2".to_string()),
        meeting_id: Some("meeting:tsg_ran/WG2_RL2/TSGR2_133bis".to_string()),
        meeting_slug: Some("TSGR2_133bis".to_string()),
        container_role: DirectoryRole::Docs,
        file_name: "R2-2601401.zip".to_string(),
        extension: Some("zip".to_string()),
        remote_modified_raw: Some("2026/04/03 9:50".to_string()),
        size_raw: Some("78,5 KB".to_string()),
        size_bytes: Some(80384),
        tdoc: Some(TDocKey {
            key: "R2-2601401".to_string(),
            prefix: "R2".to_string(),
            number_text: "2601401".to_string(),
            year_hint: Some(2026),
        }),
        classification: FileClassification {
            is_primary_tdoc: true,
            is_zip: true,
            is_ignored_artifact: false,
        },
    }
}

#[test]
fn resolves_tdoc_from_local_index() {
    let file = sample_file();
    let index = TDocLookupIndex::from_files(&[file.clone()]);
    let resolved = resolve_tdoc("r2-2601401", &index, &[file]).expect("match");

    assert_eq!(resolved.file_name, "R2-2601401.zip");
    assert_eq!(resolved.work_group_code.as_deref(), Some("RAN2"));
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test -p spectrum-3gpp-core --test resolver_tests
```

Expected: FAIL because index and resolver modules do not exist.

- [ ] **Step 3: Implement lookup index**

Modify `crates/3gpp-core/src/lib.rs`:

```rust
pub mod error;
pub mod index;
pub mod model;
pub mod resolver;

pub use error::{GppError, Result};
```

Create `crates/3gpp-core/src/index.rs`:

```rust
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::FileRecord;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TDocLookupIndex {
    pub schema_version: u32,
    pub index_type: String,
    pub items: BTreeMap<String, Vec<String>>,
}

impl TDocLookupIndex {
    pub fn from_files(files: &[FileRecord]) -> Self {
        let mut items: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for file in files {
            if !file.classification.is_primary_tdoc {
                continue;
            }
            let Some(tdoc) = &file.tdoc else {
                continue;
            };
            items.entry(tdoc.key.to_ascii_uppercase())
                .or_default()
                .push(file.id.clone());
        }

        Self {
            schema_version: 1,
            index_type: "by-tdoc".to_string(),
            items,
        }
    }
}
```

- [ ] **Step 4: Implement resolver**

Create `crates/3gpp-core/src/resolver.rs`:

```rust
use crate::index::TDocLookupIndex;
use crate::model::FileRecord;

pub fn resolve_tdoc<'a>(
    query: &str,
    index: &TDocLookupIndex,
    files: &'a [FileRecord],
) -> Option<&'a FileRecord> {
    let key = query.trim().to_ascii_uppercase();
    let ids = index.items.get(&key)?;

    files.iter().find(|file| ids.iter().any(|id| id == &file.id))
}
```

- [ ] **Step 5: Run resolver tests**

Run:

```bash
cargo test -p spectrum-3gpp-core --test resolver_tests
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/3gpp-core/src/index.rs crates/3gpp-core/src/resolver.rs crates/3gpp-core/tests/resolver_tests.rs
git commit -m "feat: resolve 3GPP TDocs from local index"
```

## Task 9: Optional Live Fetch Boundary

**Files:**
- Create: `crates/3gpp-core/src/fetch.rs`
- Modify: `crates/3gpp-core/src/lib.rs`

- [ ] **Step 1: Add fetch trait and no-network default implementation**

Modify `crates/3gpp-core/src/lib.rs`:

```rust
pub mod error;
pub mod fetch;

pub use error::{GppError, Result};
```

Create `crates/3gpp-core/src/fetch.rs`:

```rust
use crate::error::Result;

pub trait DirectoryFetcher {
    fn fetch_directory_html(&self, url: &str) -> impl std::future::Future<Output = Result<String>> + Send;
}

#[cfg(feature = "live-fetch")]
pub struct ReqwestDirectoryFetcher {
    client: reqwest::Client,
}

#[cfg(feature = "live-fetch")]
impl ReqwestDirectoryFetcher {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[cfg(feature = "live-fetch")]
impl Default for ReqwestDirectoryFetcher {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: Run compilation without live fetch**

Run:

```bash
cargo test -p spectrum-3gpp-core
```

Expected: PASS.

- [ ] **Step 3: Run compilation with live fetch feature**

Run:

```bash
cargo test -p spectrum-3gpp-core --features live-fetch
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/3gpp-core/src/fetch.rs crates/3gpp-core/Cargo.toml
git commit -m "feat: add 3GPP directory fetch boundary"
```

## Task 10: Documentation And Verification

**Files:**
- Modify: `README.md`
- Modify: `docs/v0.1/plans/README.md`

- [ ] **Step 1: Update project status**

Modify `README.md` to mention that the first code milestone is the Rust `3gpp-core` local index parser and resolver.

- [ ] **Step 2: Update plan index**

Modify `docs/v0.1/plans/README.md` to include:

```markdown
- [3GPP Local Index Core](./3gpp-local-index-core.md)
```

- [ ] **Step 3: Run full verification**

Run:

```bash
cargo test --workspace
```

Expected: all tests PASS.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/v0.1/plans/README.md
git commit -m "docs: document 3GPP core implementation milestone"
```

## Acceptance Criteria

- `cargo test --workspace` passes.
- The core crate can parse current 3GPP-style directory listing HTML from fixtures.
- The core crate can build `DirectoryManifest` records.
- The core crate can build `MeetingRecord` and `FileRecord` records.
- Primary TDoc zip files under `Docs` are classified as `tdoc-file`.
- Root-level or non-proposal files are classified as auxiliary files.
- `__MACOSX` and similar artifacts do not become primary TDoc records.
- The lookup index resolves a normalized TDoc query such as `r2-2601401` to a file record.
- No Tauri or React UI work is required for this milestone.

## Implementation Notes

- Do not download zip contents in this milestone.
- Preserve raw remote timestamp and size strings.
- Do not use TDoc number as a primary file ID.
- Use URL hash for file identity.
- Keep network access optional and isolated behind `live-fetch`.
- Fixture-based parser tests are required before any live-site smoke testing.

## Self-Review

Spec coverage:

- Structured storage schema: covered by Tasks 2, 5, 6, 7, and 8.
- Manifest and parent-directory diff foundation: covered by Task 5.
- Current `.../<meeting>/Docs/*.zip` shape: covered by Tasks 4 and 6.
- Local lookup by proposal number: covered by Task 8.
- Avoiding UI-first implementation: enforced by the plan scope and acceptance criteria.

Placeholder scan:

- The plan contains no `TBD` or `TODO` markers.
- Code steps include concrete file paths and code blocks.

Type consistency:

- `DirectoryManifest`, `MeetingRecord`, `FileRecord`, and `TDocLookupIndex` are defined before downstream tasks use them.
- Stable ID formats match the design document.
