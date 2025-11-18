use super::super::context::SharedExecutionContext;
use super::super::events::EngineEvent;
use crate::scenario::ExtractVarFromFileConfig;
use anyhow::Context;
use regex::Regex;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc::UnboundedSender;

use super::utils::log_step;

/// Extract Step을 실행한다.
pub(super) async fn execute_extract_step(
    config: &ExtractVarFromFileConfig,
    ctx: SharedExecutionContext,
    step_id: &str,
    sender: &UnboundedSender<EngineEvent>,
) -> anyhow::Result<()> {
    let file_path = {
        let guard = ctx.read().await;
        guard.expand_required(&config.file_path, "extract.file_path")?
    };
    let file = File::open(&file_path)
        .await
        .with_context(|| format!("파일을 열 수 없습니다: {file_path}"))?;
    let mut reader = BufReader::new(file).lines();
    let mut current_line = None;
    for i in 1..=config.line {
        if let Some(line) = reader.next_line().await? {
            if i == config.line {
                current_line = Some(line);
                break;
            }
        } else {
            anyhow::bail!("{file_path}에서 {}번째 줄을 찾을 수 없습니다.", config.line);
        }
    }
    let content = current_line.unwrap_or_default();
    let re = Regex::new(&config.pattern)
        .with_context(|| format!("정규식 컴파일 실패: {}", config.pattern))?;
    let captures = re
        .captures(&content)
        .ok_or_else(|| anyhow::anyhow!("패턴이 매칭되지 않았습니다: {content}"))?;
    let value = captures
        .get(config.group)
        .ok_or_else(|| anyhow::anyhow!("캡처 그룹 {}을 찾을 수 없습니다.", config.group))?
        .as_str()
        .to_string();
    {
        let mut guard = ctx.write().await;
        guard.set_var(&config.var_name, &value);
    }
    log_step(
        sender,
        step_id,
        &format!("변수 {} = {}", config.var_name, value),
    );
    Ok(())
}
