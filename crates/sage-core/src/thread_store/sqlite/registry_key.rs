use std::path::Path;

pub(super) fn sqlite_registry_key(path: &Path) -> String {
    if path == Path::new(":memory:") {
        return format!("sqlite-memory:{}", uuid::Uuid::new_v4());
    }
    let db_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    format!("sqlite:{}", db_path.display())
}
