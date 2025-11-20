use super::super::context::SharedExecutionContext;
use super::super::events::{ConfirmPhase, EngineEvent};
use crate::engine::ConfirmBridge;
use crate::scenario::{ConfirmDefault, Step, StepConfirmConfig, StepKind};
use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;

/// 컨펌 설정을 기반으로 실제 UI 상호작용을 수행하고 결과를 컨텍스트에 저장한다.
///
/// 수락 여부는 `CONFIRM_<STEP_ID>` 형태(대문자)로 `ExecutionContext`에 "Yes" 또는
/// "No" 값으로 기록되어 이후 Step에서 플레이스홀더로 활용할 수 있다.
pub(super) async fn evaluate_confirm(
    step: &Step,
    confirm: &StepConfirmConfig,
    phase: ConfirmPhase,
    ctx: SharedExecutionContext,
    sender: &UnboundedSender<EngineEvent>,
    bridge: Option<ConfirmBridge>,
) -> Result<bool> {
    let (enabled, message) = match phase {
        ConfirmPhase::Before => (confirm.before, confirm.message_before.clone()),
        ConfirmPhase::After => (confirm.after, confirm.message_after.clone()),
    };
    if !enabled {
        return Ok(true);
    }

    let mut accepted = matches!(confirm.default_answer, ConfirmDefault::Yes);
    let answered = if let Some(bridge) = bridge {
        let (request_id, rx) = bridge.register();
        let event = EngineEvent::RequestConfirm {
            request_id,
            step_id: step.id.clone(),
            step_name: step.name.clone(),
            step_kind: step_kind_label(&step.kind),
            summary: summarize_step(step),
            message,
            default_answer: confirm.default_answer.clone(),
            phase,
        };
        let _ = sender.send(event);
        match rx.await {
            Ok(answer) => {
                accepted = answer;
                let _ = sender.send(EngineEvent::ConfirmResponse {
                    request_id,
                    step_id: step.id.clone(),
                    accepted: answer,
                });
                Ok(answer)
            }
            Err(_) => {
                bridge.cancel(request_id);
                Ok(matches!(confirm.default_answer, ConfirmDefault::Yes))
            }
        }
    } else {
        Ok(matches!(confirm.default_answer, ConfirmDefault::Yes))
    }?;

    let mut guard = ctx.write().await;
    let key = format!("CONFIRM_{}", step.id.to_uppercase());
    guard.set_var(&key, if accepted { "Yes" } else { "No" });
    Ok(answered)
}

/// StepKind를 사용자 친화적인 문자열로 변환한다.
fn step_kind_label(kind: &StepKind) -> String {
    match kind {
        StepKind::Sql { .. } => "sql",
        StepKind::SqlFile { .. } => "sql_file",
        StepKind::SqlLoaderPar { .. } => "sql_loader_par",
        StepKind::Shell { .. } => "shell",
        StepKind::Extract { .. } => "extract",
        StepKind::Loop { .. } => "loop",
    }
    .into()
}

/// Step 내용을 간단히 요약한다.
fn summarize_step(step: &Step) -> Option<String> {
    match &step.kind {
        StepKind::Sql { sql, .. } => Some(trim_lines(sql, 4)),
        StepKind::SqlFile { path, .. } => Some(format!("파일: {}", path.display())),
        StepKind::SqlLoaderPar { config } => {
            Some(format!("control: {}", config.control_file.display()))
        }
        StepKind::Shell { config } => Some(trim_lines(&config.script, 4)),
        StepKind::Extract { config } => Some(format!(
            "파일: {} / 그룹: {} / 변수: {}",
            config.file_path, config.group, config.var_name
        )),
        StepKind::Loop { config } => Some(format!(
            "Loop {} → {} ({} steps)",
            config.for_each_glob,
            config.as_var,
            config.steps.len()
        )),
    }
}

/// 여러 줄 문자열을 앞부분만 남기고 정리한다.
fn trim_lines(text: &str, max_lines: usize) -> String {
    let mut lines: Vec<&str> = text.lines().take(max_lines).collect();
    if text.lines().count() > max_lines {
        lines.push("...");
    }
    lines.join("\n")
}
