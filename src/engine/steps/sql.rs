use super::super::context::SharedExecutionContext;
use super::super::resources::EngineHandles;
use anyhow::Context;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;

/// SQL 문자열을 실행한다.
pub(super) async fn execute_sql(
    sql: &str,
    target_db: Option<&str>,
    handles: Arc<EngineHandles>,
    ctx: SharedExecutionContext,
) -> anyhow::Result<()> {
    let expanded_sql = {
        let guard = ctx.read().await;
        guard.expand_required(sql, "sql")?
    };
    let target = target_db.unwrap_or("default");
    let executor = handles.get_db_executor(target)?;
    executor.execute_sql(&expanded_sql).await
}

/// SQL 파일을 읽어 문자열을 반환한다.
pub(super) async fn load_sql_file(
    path: &PathBuf,
    ctx: SharedExecutionContext,
) -> anyhow::Result<String> {
    let raw = path.to_string_lossy().to_string();
    let actual_path = {
        let guard = ctx.read().await;
        guard.expand_required(&raw, "sql_file")?
    };
    let content = fs::read_to_string(&actual_path)
        .await
        .with_context(|| format!("SQL 파일 읽기 실패: {actual_path}"))?;
    let guard = ctx.read().await;
    guard.expand_required(&content, "sql_file_content")
}
