use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

pub fn choose_open_target(stem: &str, relative_files: &[String]) -> Option<String> {
    for extension in ["docx", "doc", "pdf"] {
        let exact = format!("{stem}.{extension}");
        if let Some(file) = relative_files
            .iter()
            .find(|file| file.eq_ignore_ascii_case(&exact))
        {
            return Some(file.clone());
        }
    }

    for extension in ["docx", "doc", "pdf"] {
        let matches = relative_files
            .iter()
            .filter(|file| {
                file.to_ascii_lowercase()
                    .ends_with(&format!(".{extension}"))
            })
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            return Some(matches[0].clone());
        }
    }

    None
}

pub fn tdoc_extract_dir(
    workspace_root: &Path,
    work_group: &str,
    meeting: &str,
    tdoc: &str,
) -> PathBuf {
    workspace_root
        .join("3gpp")
        .join("tdocs")
        .join(work_group)
        .join(meeting)
        .join(tdoc)
}

pub fn spec_extract_dir(workspace_root: &Path, spec_number: &str, version: &str) -> PathBuf {
    workspace_root
        .join("3gpp")
        .join("specs")
        .join(spec_number)
        .join(version)
}

pub fn zip_path_for_extract_dir(extract_dir: &Path, file_name: &str) -> PathBuf {
    extract_dir.join(file_name)
}

pub async fn download_zip(
    client: &reqwest::Client,
    url: &str,
    zip_path: &Path,
) -> Result<u64, String> {
    if let Some(parent) = zip_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|source| format!("failed to create {}: {source}", parent.display()))?;
    }

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|source| format!("failed to download {url}: {source}"))?
        .error_for_status()
        .map_err(|source| format!("failed to download {url}: {source}"))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|source| format!("failed to read download {url}: {source}"))?;
    fs::write(zip_path, &bytes)
        .map_err(|source| format!("failed to write {}: {source}", zip_path.display()))?;
    Ok(bytes.len() as u64)
}

pub fn extract_zip(zip_path: &Path, extract_dir: &Path) -> Result<Vec<String>, String> {
    fs::create_dir_all(extract_dir)
        .map_err(|source| format!("failed to create {}: {source}", extract_dir.display()))?;

    let zip_file = File::open(zip_path)
        .map_err(|source| format!("failed to open {}: {source}", zip_path.display()))?;
    let mut archive = zip::ZipArchive::new(zip_file)
        .map_err(|source| format!("failed to read zip {}: {source}", zip_path.display()))?;
    let mut relative_files = Vec::new();

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|source| format!("failed to read zip entry {index}: {source}"))?;
        let Some(relative_path) = entry.enclosed_name().map(PathBuf::from) else {
            continue;
        };
        let output_path = extract_dir.join(&relative_path);

        if entry.is_dir() {
            fs::create_dir_all(&output_path).map_err(|source| {
                format!("failed to create {}: {source}", output_path.display())
            })?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|source| format!("failed to create {}: {source}", parent.display()))?;
        }

        let mut output = File::create(&output_path)
            .map_err(|source| format!("failed to create {}: {source}", output_path.display()))?;
        io::copy(&mut entry, &mut output)
            .map_err(|source| format!("failed to extract {}: {source}", output_path.display()))?;
        relative_files.push(relative_path.to_string_lossy().replace('\\', "/"));
    }

    Ok(relative_files)
}

pub fn resolve_open_path(extract_dir: &Path, stem: &str, relative_files: &[String]) -> PathBuf {
    choose_open_target(stem, relative_files)
        .map(|relative| extract_dir.join(relative))
        .unwrap_or_else(|| extract_dir.to_path_buf())
}

pub fn cached_open_path(
    extract_dir: &Path,
    stem: &str,
    zip_file_name: &str,
) -> Result<Option<PathBuf>, String> {
    let relative_files = list_extracted_relative_files(extract_dir, zip_file_name)?;
    if relative_files.is_empty() {
        return Ok(None);
    }

    let open_path = resolve_open_path(extract_dir, stem, &relative_files);
    if open_path.exists() {
        Ok(Some(open_path))
    } else {
        Ok(None)
    }
}

pub fn has_cached_zip(zip_path: &Path) -> bool {
    zip_path
        .metadata()
        .map(|metadata| metadata.is_file() && metadata.len() > 0)
        .unwrap_or(false)
}

fn list_extracted_relative_files(
    extract_dir: &Path,
    zip_file_name: &str,
) -> Result<Vec<String>, String> {
    if !extract_dir.exists() {
        return Ok(Vec::new());
    }

    let mut relative_files = Vec::new();
    collect_relative_files(extract_dir, extract_dir, zip_file_name, &mut relative_files)?;
    relative_files.sort();
    Ok(relative_files)
}

fn collect_relative_files(
    root: &Path,
    current: &Path,
    zip_file_name: &str,
    relative_files: &mut Vec<String>,
) -> Result<(), String> {
    for entry in fs::read_dir(current)
        .map_err(|source| format!("failed to read {}: {source}", current.display()))?
    {
        let entry =
            entry.map_err(|source| format!("failed to read {}: {source}", current.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_relative_files(root, &path, zip_file_name, relative_files)?;
            continue;
        }
        if path.file_name().and_then(|value| value.to_str()) == Some(zip_file_name) {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|source| format!("failed to read relative path {}: {source}", path.display()))?
            .to_string_lossy()
            .replace('\\', "/");
        relative_files.push(relative);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        cached_open_path, choose_open_target, has_cached_zip, spec_extract_dir, tdoc_extract_dir,
    };

    #[test]
    fn chooses_exact_docx_before_other_documents() {
        let files = vec![
            "R2-2601401.pdf".to_string(),
            "R2-2601401.docx".to_string(),
            "other.docx".to_string(),
        ];

        assert_eq!(
            choose_open_target("R2-2601401", &files).as_deref(),
            Some("R2-2601401.docx")
        );
    }

    #[test]
    fn chooses_single_docx_when_no_exact_file_exists() {
        let files = vec!["cover.txt".to_string(), "document.docx".to_string()];

        assert_eq!(
            choose_open_target("R2-2601401", &files).as_deref(),
            Some("document.docx")
        );
    }

    #[test]
    fn returns_none_when_multiple_documents_are_ambiguous() {
        let files = vec!["a.docx".to_string(), "b.docx".to_string()];

        assert_eq!(choose_open_target("R2-2601401", &files), None);
    }

    #[test]
    fn builds_stable_extract_directories() {
        let workspace = PathBuf::from("/tmp/SpectrumPilotWorkspace");

        assert_eq!(
            tdoc_extract_dir(&workspace, "RAN2", "TSGR2_133bis", "R2-2601401"),
            workspace
                .join("3gpp")
                .join("tdocs")
                .join("RAN2")
                .join("TSGR2_133bis")
                .join("R2-2601401")
        );
        assert_eq!(
            spec_extract_dir(&workspace, "38.321", "j30"),
            workspace
                .join("3gpp")
                .join("specs")
                .join("38.321")
                .join("j30")
        );
    }

    #[test]
    fn cached_open_path_prefers_existing_exact_document_and_ignores_zip() {
        let temp = tempfile::tempdir().expect("tempdir");
        let extract_dir = temp.path().join("R2-2601401");
        std::fs::create_dir_all(&extract_dir).expect("extract dir");
        std::fs::write(extract_dir.join("R2-2601401.zip"), b"zip").expect("zip");
        std::fs::write(extract_dir.join("R2-2601401.docx"), b"docx").expect("docx");
        std::fs::write(extract_dir.join("other.docx"), b"other").expect("other");

        let open_path = cached_open_path(&extract_dir, "R2-2601401", "R2-2601401.zip")
            .expect("cached path")
            .expect("open path");

        assert_eq!(open_path, extract_dir.join("R2-2601401.docx"));
    }

    #[test]
    fn cached_open_path_returns_none_when_only_zip_exists() {
        let temp = tempfile::tempdir().expect("tempdir");
        let extract_dir = temp.path().join("R2-2601401");
        std::fs::create_dir_all(&extract_dir).expect("extract dir");
        std::fs::write(extract_dir.join("R2-2601401.zip"), b"zip").expect("zip");

        assert_eq!(
            cached_open_path(&extract_dir, "R2-2601401", "R2-2601401.zip").expect("cached path"),
            None
        );
    }

    #[test]
    fn has_cached_zip_requires_existing_non_empty_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let zip_path = temp.path().join("R2-2601401.zip");

        assert!(!has_cached_zip(&zip_path));
        std::fs::write(&zip_path, b"").expect("empty zip");
        assert!(!has_cached_zip(&zip_path));
        std::fs::write(&zip_path, b"zip").expect("zip");
        assert!(has_cached_zip(&zip_path));
    }
}
