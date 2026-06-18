use tauri::State;

use crate::audit::{list_audit_events as load_audit_events, AuditEvent};

use super::{AppError, AppState};

#[tauri::command]
pub async fn list_audit_events(
    limit: Option<i64>,
    offset: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<AuditEvent>, AppError> {
    Ok(load_audit_events(&state.pool, limit.unwrap_or(100), offset.unwrap_or(0)).await?)
}
