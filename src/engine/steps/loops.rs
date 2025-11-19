use super::super::context::SharedExecutionContext;
use super::super::events::EngineEvent;
use super::super::resources::EngineHandles;
use super::{StepRunResult, run_single_step};
use crate::engine::ConfirmBridge;
use crate::scenario::{LoopIterationFailure, LoopStepConfig, Step};
use anyhow::{Context, Result};
use glob::glob;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use super::utils::log_step;

/// Loop Step을 실행하고 각 반복에서 하위 Step 전체를 처리한다.
pub(super) async fn execute_loop_step(
    config: &LoopStepConfig,
    log_step_id: &str,
    handles: Arc<EngineHandles>,
    ctx: SharedExecutionContext,
    sender: UnboundedSender<EngineEvent>,
    cancel: CancellationToken,
    confirm_bridge: Option<ConfirmBridge>,
) -> Result<()> {
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
    for (idx, entry) in entries.iter().enumerate() {
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
            &format!("[반복 {}] {} = {}", idx + 1, config.as_var, value),
        );
        let iteration_result = run_iteration_steps(
            &config.steps,
            handles.clone(),
            ctx.clone(),
            sender.clone(),
            cancel.clone(),
            confirm_bridge.clone(),
        )
        .await;
        if let Err(err) = iteration_result {
            match config.on_iteration_failure {
                LoopIterationFailure::StopAll => return Err(err),
                LoopIterationFailure::Continue => {
                    log_step(&sender, log_step_id, &format!("Loop 반복 실패 무시: {err}"));
                }
            }
        }
    }
    Ok(())
}

/// 단일 Loop 반복에서 하위 Step 전체를 의존성 순으로 실행한다.
async fn run_iteration_steps(
    steps: &[Step],
    handles: Arc<EngineHandles>,
    ctx: SharedExecutionContext,
    sender: UnboundedSender<EngineEvent>,
    cancel: CancellationToken,
    confirm_bridge: Option<ConfirmBridge>,
) -> Result<()> {
    if steps.is_empty() {
        return Ok(());
    }
    let mut completed: HashSet<String> = HashSet::new();
    let total = steps.len();
    loop {
        if completed.len() == total {
            break;
        }
        if cancel.is_cancelled() {
            anyhow::bail!("사용자에 의해 Loop 반복이 중단되었습니다.");
        }
        let mut progressed = false;
        for step in steps {
            if completed.contains(&step.id) {
                continue;
            }
            ensure_dependencies(step, &completed, steps)?;
            if !step.depends_on.iter().all(|dep| completed.contains(dep)) {
                continue;
            }
            progressed = true;
            match run_single_step(
                step.clone(),
                handles.clone(),
                ctx.clone(),
                sender.clone(),
                cancel.clone(),
                confirm_bridge.clone(),
            )
            .await
            {
                StepRunResult::Success => {
                    completed.insert(step.id.clone());
                }
                StepRunResult::Failed(msg) => anyhow::bail!(msg),
            }
        }
        if !progressed {
            anyhow::bail!("Loop 하위 Step 실행 순서를 결정할 수 없습니다. 의존성을 확인하세요.");
        }
    }
    Ok(())
}

/// 의존성이 Loop 내부 Step에만 국한되는지 검증한다.
fn ensure_dependencies(step: &Step, completed: &HashSet<String>, steps: &[Step]) -> Result<()> {
    for dep in &step.depends_on {
        if completed.contains(dep) {
            continue;
        }
        if !steps.iter().any(|s| &s.id == dep) {
            anyhow::bail!(
                "Loop 하위 Step '{}'에서 알 수 없는 의존성 '{}'을 참조했습니다.",
                step.id,
                dep
            );
        }
    }
    Ok(())
}
