use super::super::events::EngineEvent;
use crate::scenario::{ConfirmDefault, Step, StepConfirmConfig};
use tokio::sync::mpsc::UnboundedSender;

/// Confirm 단계를 구분하기 위한 열거형이다.
pub(super) enum ConfirmPhase {
    /// 실행 전 확인 단계.
    Before,
    /// 실행 후 확인 단계.
    After,
}

/// 컨펌 설정을 기반으로 기본 응답을 평가한다.
pub(super) fn evaluate_confirm(
    step: &Step,
    log_step_id: &str,
    confirm: &StepConfirmConfig,
    phase: ConfirmPhase,
    sender: &UnboundedSender<EngineEvent>,
) -> bool {
    let (enabled, message) = match phase {
        ConfirmPhase::Before => (confirm.before, confirm.message_before.as_deref()),
        ConfirmPhase::After => (confirm.after, confirm.message_after.as_deref()),
    };
    if !enabled {
        return true;
    }
    let default_text = match confirm.default_answer {
        ConfirmDefault::Yes => "YES",
        ConfirmDefault::No => "NO",
    };
    let phase_text = match phase {
        ConfirmPhase::Before => "실행 전",
        ConfirmPhase::After => "실행 후",
    };
    let msg = message
        .map(|m| m.to_string())
        .unwrap_or_else(|| format!("{phase_text} 확인이 필요합니다."));
    let _ = sender.send(EngineEvent::StepLog {
        step_id: log_step_id.to_string(),
        line: format!("[Confirm:{}] {msg} (기본응답: {default_text})", step.id),
    });
    matches!(confirm.default_answer, ConfirmDefault::Yes)
}
