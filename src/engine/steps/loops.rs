use super::super::context::SharedExecutionContext;
use super::super::events::EngineEvent;
use super::super::resources::EngineHandles;
use crate::scenario::{LoopStepConfig, Step};
use anyhow::Context;
use futures::future::BoxFuture;
use glob::glob;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use super::confirm::{ConfirmPhase, evaluate_confirm};
use super::utils::log_step;

/// Loop Step을 실행한다.
pub(super) async fn execute_loop_step(
    config: &LoopStepConfig,
    log_step_id: &str,
    handles: Arc<EngineHandles>,
    ctx: SharedExecutionContext,
    sender: UnboundedSender<EngineEvent>,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    let pattern = {
        let guard = ctx.read().await;
        guard.expand_required(&config.for_each_glob, "loop.pattern")?
    };
    let mut entries: Vec<PathBuf> = Vec::new();
    for entry in glob(&pattern).context("glob 패턴 파싱 실패")? {
        entries.push(entry?);
    }
    if entries.is_empty() {
        log_step(
            &sender,
            log_step_id,
            &format!("Loop 패턴에 해당하는 파일이 없습니다: {pattern}"),
        );
        return Ok(());
    }
    for entry in entries {
        if cancel.is_cancelled() {
            anyhow::bail!("사용자에 의해 Loop Step이 중단되었습니다.");
        }
        let value = entry.to_string_lossy().to_string();
        {
            let mut guard = ctx.write().await;
            guard.set_var(&config.as_var, &value);
        }
        log_step(
            &sender,
            log_step_id,
            &format!("Loop 변수 {} = {}", config.as_var, value),
        );
        for child in &config.steps {
            log_step(
                &sender,
                log_step_id,
                &format!("Loop 하위 Step '{}' 실행", child.name),
            );
            run_embedded_step(
                child,
                log_step_id,
                handles.clone(),
                ctx.clone(),
                sender.clone(),
                cancel.clone(),
            )
            .await
            .with_context(|| format!("Loop 하위 Step '{}' 실패", child.name))?;
        }
    }
    Ok(())
}

/// Loop 내 하위 Step을 순차 실행한다.
fn run_embedded_step<'a>(
    step: &'a Step,
    log_step_id: &'a str,
    handles: Arc<EngineHandles>,
    ctx: SharedExecutionContext,
    sender: UnboundedSender<EngineEvent>,
    cancel: CancellationToken,
) -> BoxFuture<'a, anyhow::Result<()>> {
    Box::pin(async move {
        if let Some(confirm) = &step.confirm {
            if !evaluate_confirm(step, log_step_id, confirm, ConfirmPhase::Before, &sender) {
                anyhow::bail!("Loop 하위 Step '{}' 사전 컨펌 거부", step.name);
            }
        }

        let timeout_duration = Duration::from_secs(step.timeout_sec.max(1));
        let mut attempt: u8 = 0;

        loop {
            if cancel.is_cancelled() {
                anyhow::bail!("사용자에 의해 Loop 하위 Step이 중단되었습니다.");
            }

            let backoff = Duration::from_secs(2_u64.pow(attempt as u32));

            let exec_future = super::execute_step_kind(
                step,
                log_step_id,
                handles.clone(),
                ctx.clone(),
                sender.clone(),
                cancel.clone(),
            );

            let result = tokio::time::timeout(timeout_duration, exec_future).await;

            match result {
                Ok(Ok(())) => {
                    if let Some(confirm) = &step.confirm {
                        if !evaluate_confirm(
                            step,
                            log_step_id,
                            confirm,
                            ConfirmPhase::After,
                            &sender,
                        ) {
                            anyhow::bail!("Loop 하위 Step '{}' 사후 컨펌 거부", step.name);
                        }
                    }
                    return Ok(());
                }
                Ok(Err(err)) => {
                    attempt += 1;
                    if attempt > step.retry {
                        return Err(err);
                    }
                    sleep(backoff).await;
                }
                Err(_) => {
                    attempt += 1;
                    if attempt > step.retry {
                        anyhow::bail!("Loop 하위 Step '{}' 시간 초과", step.name);
                    }
                    sleep(backoff).await;
                }
            }
        }
    })
}
