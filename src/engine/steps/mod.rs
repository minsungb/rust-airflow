use super::context::SharedExecutionContext;
use super::events::EngineEvent;
use super::resources::EngineHandles;
use crate::scenario::{Step, StepKind};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

mod confirm;
mod extract;
mod loops;
mod shell;
mod sql;
mod sqlldr;
mod utils;

use confirm::{ConfirmPhase, evaluate_confirm};
use extract::execute_extract_step;
use loops::execute_loop_step;
use shell::run_shell_command;
use sql::{execute_sql, load_sql_file};
use sqlldr::run_sqlldr;
use utils::{display_path, log_step};

/// Step 실행의 결과를 표현한다.
#[derive(Debug)]
pub(super) enum StepRunResult {
    /// 실행 성공.
    Success,
    /// 오류 메시지와 함께 실패.
    Failed(String),
}

/// 단일 Step을 실행하고 결과를 반환한다.
pub(super) async fn run_single_step(
    step: Step,
    handles: Arc<EngineHandles>,
    ctx: SharedExecutionContext,
    sender: UnboundedSender<EngineEvent>,
    cancel: CancellationToken,
) -> StepRunResult {
    if let Some(confirm) = &step.confirm {
        if !evaluate_confirm(&step, &step.id, confirm, ConfirmPhase::Before, &sender) {
            return StepRunResult::Failed(format!(
                "사전 컨펌에서 Step '{}' 실행이 거부되었습니다.",
                step.name
            ));
        }
    }
    let timeout_duration = Duration::from_secs(step.timeout_sec.max(1));
    let mut attempt: u8 = 0;
    loop {
        if cancel.is_cancelled() {
            return StepRunResult::Failed("사용자에 의해 실행이 중단되었습니다.".to_string());
        }
        let backoff = Duration::from_secs(2_u64.pow(attempt as u32));
        let log_step_id = step.id.clone();
        let exec_future = execute_step_kind(
            &step,
            &log_step_id,
            handles.clone(),
            ctx.clone(),
            sender.clone(),
            cancel.clone(),
        );
        let result = tokio::time::timeout(timeout_duration, exec_future).await;
        match result {
            Ok(Ok(())) => {
                if let Some(confirm) = &step.confirm {
                    if !evaluate_confirm(&step, &step.id, confirm, ConfirmPhase::After, &sender) {
                        return StepRunResult::Failed(format!(
                            "사후 컨펌에서 Step '{}' 실행이 거부되었습니다.",
                            step.name
                        ));
                    }
                }
                return StepRunResult::Success;
            }
            Ok(Err(err)) => {
                attempt += 1;
                if attempt > step.retry {
                    return StepRunResult::Failed(format!("실패: {err}"));
                }
                let _ = sender.send(EngineEvent::StepLog {
                    step_id: step.id.clone(),
                    line: format!("오류 발생, {}초 후 재시도", backoff.as_secs()),
                });
                sleep(backoff).await;
            }
            Err(_) => {
                attempt += 1;
                if attempt > step.retry {
                    return StepRunResult::Failed("시간 초과".into());
                }
                let _ = sender.send(EngineEvent::StepLog {
                    step_id: step.id.clone(),
                    line: "시간 초과 발생, 재시도 준비".into(),
                });
                sleep(backoff).await;
            }
        }
    }
}

/// StepKind별 실제 수행 로직을 실행한다.
async fn execute_step_kind(
    step: &Step,
    log_step_id: &str,
    handles: Arc<EngineHandles>,
    ctx: SharedExecutionContext,
    sender: UnboundedSender<EngineEvent>,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    match &step.kind {
        StepKind::Sql { sql, target_db } => {
            log_step(&sender, log_step_id, "SQL 실행 시작");
            execute_sql(sql, target_db.as_deref(), handles, ctx).await?;
        }
        StepKind::SqlFile { path, target_db } => {
            let file_sql = load_sql_file(path, ctx.clone()).await?;
            log_step(
                &sender,
                log_step_id,
                &format!("SQL 파일 실행: {}", display_path(path)),
            );
            execute_sql(&file_sql, target_db.as_deref(), handles, ctx).await?;
        }
        StepKind::SqlLoaderPar { config } => {
            run_sqlldr(
                config,
                ctx,
                &sender,
                log_step_id,
                Duration::from_secs(step.timeout_sec.max(1)),
            )
            .await?;
        }
        StepKind::Shell { config } => {
            run_shell_command(
                config,
                ctx,
                &sender,
                log_step_id,
                Duration::from_secs(step.timeout_sec.max(1)),
            )
            .await?;
        }
        StepKind::ExtractVarFromFile { config } => {
            execute_extract_step(config, ctx, log_step_id, &sender).await?;
        }
        StepKind::Loop { config } => {
            execute_loop_step(config, log_step_id, handles, ctx, sender, cancel).await?;
        }
    }
    Ok(())
}
