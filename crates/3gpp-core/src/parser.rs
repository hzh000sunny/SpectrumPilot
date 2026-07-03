use scraper::{Html, Selector};

use crate::error::{GppError, Result};
use crate::model::{DirectoryRole, EntryKind};
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
    let checkbox_selector =
        Selector::parse("input[type='checkbox']").map_err(|e| GppError::Parse(e.to_string()))?;

    let mut rows = Vec::new();

    for row in document.select(&row_selector) {
        let cells: Vec<_> = row.select(&cell_selector).collect();
        let Some(anchor) = row.select(&anchor_selector).find(|a| {
            a.value().attr("href").is_some_and(|href| {
                href.to_ascii_lowercase()
                    .starts_with("https://www.3gpp.org/ftp/")
            })
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

        let kind = if row.select(&checkbox_selector).next().is_some() {
            EntryKind::File
        } else {
            EntryKind::Directory
        };

        let remote_modified_raw = cells
            .iter()
            .map(|cell| cell.text().collect::<String>().trim().to_string())
            .find(|text| text.contains('/') && text.contains(':'));

        let size_raw = cells.last().and_then(|cell| {
            let text = cell.text().collect::<String>().trim().to_string();
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        });

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
