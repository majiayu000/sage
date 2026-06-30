use crate::thread_store::error::{ThreadStoreError, ThreadStoreResult};

pub(super) fn u64_to_i64(field: &'static str, value: u64) -> ThreadStoreResult<i64> {
    i64::try_from(value).map_err(|_| {
        ThreadStoreError::InvalidInput(format!("{field} exceeds sqlite integer range: {value}"))
    })
}

pub(super) fn stored_i64_to_u64(
    table: &'static str,
    field: &'static str,
    id: &str,
    value: i64,
) -> ThreadStoreResult<u64> {
    u64::try_from(value).map_err(|_| ThreadStoreError::InvalidStoredData {
        table,
        field,
        id: id.to_string(),
        message: format!("negative integer: {value}"),
    })
}

pub(super) fn stored_optional_i64_to_u64(
    table: &'static str,
    field: &'static str,
    id: &str,
    value: Option<i64>,
) -> ThreadStoreResult<Option<u64>> {
    value
        .map(|value| stored_i64_to_u64(table, field, id, value))
        .transpose()
}
