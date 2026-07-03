use crate::index::TDocLookupIndex;
use crate::model::FileRecord;

pub fn resolve_tdoc<'a>(
    query: &str,
    index: &TDocLookupIndex,
    files: &'a [FileRecord],
) -> Option<&'a FileRecord> {
    let key = query.trim().to_ascii_uppercase();
    let ids = index.items.get(&key)?;

    files
        .iter()
        .find(|file| ids.iter().any(|id| id == &file.id))
}
