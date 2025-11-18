use super::events::EngineEvent;
use super::state::{ScenarioRuntime, StepStatus};
use super::steps::{StepRunResult, run_single_step};
use crate::executor::SharedExecutor;
use crate::scenario::{Scenario, Step};
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

/// Scenario 전체를 실행하고 이벤트를 송신한다.
pub async fn run_scenario(
    scenario: Scenario,
    executor: SharedExecutor,
    sender: UnboundedSender<EngineEvent>,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    let mut runtime = ScenarioRuntime::new(scenario.clone());
    let mut started: HashSet<String> = HashSet::new();
    let mut succeeded: HashSet<String> = HashSet::new();
    let mut failed: HashSet<String> = HashSet::new();
    type RunningHandle = tokio::task::JoinHandle<(String, StepRunResult)>;
    let mut running_tasks: FuturesUnordered<RunningHandle> = FuturesUnordered::new();
    loop {
        if cancel.is_cancelled() {
            break;
        }
        mark_blocked_steps(&scenario, &mut runtime, &mut started, &mut failed, &sender);
        let ready_steps = collect_ready_steps(&scenario, &started, &succeeded, &failed);
        let mut sequential: Vec<Step> = Vec::new();
        let mut parallel: Vec<Step> = Vec::new();
        for step in ready_steps {
            if step.allow_parallel {
                parallel.push(step);
            } else {
                sequential.push(step);
            }
        }
        for step in sequential {
            let step_id = step.id.clone();
            started.insert(step_id.clone());
            mark_step_started(&mut runtime, &step_id, &sender);
            let result =
                run_single_step(step, executor.clone(), sender.clone(), cancel.clone()).await;
            apply_result(
                result,
                &mut runtime,
                &step_id,
                &mut succeeded,
                &mut failed,
                &sender,
            );
        }
        for step in parallel {
            let step_id = step.id.clone();
            started.insert(step_id.clone());
            mark_step_started(&mut runtime, &step_id, &sender);
            let exec = executor.clone();
            let tx = sender.clone();
            let token = cancel.clone();
            running_tasks.push(tokio::spawn(async move {
                let outcome = run_single_step(step, exec, tx, token).await;
                (step_id, outcome)
            }));
        }
        if let Some(join_result) = running_tasks.next().await {
            let (step_id, run_result) = match join_result {
                Ok(value) => value,
                Err(err) => (
                    "unknown".to_string(),
                    StepRunResult::Failed(err.to_string()),
                ),
            };
            apply_result(
                run_result,
                &mut runtime,
                &step_id,
                &mut succeeded,
                &mut failed,
                &sender,
            );
            continue;
        }
        if runtime.steps_state.len() == succeeded.len() + failed.len() {
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }
    let _ = sender.send(EngineEvent::ScenarioFinished);
    Ok(())
}

/// Step이 시작될 때 상태와 이벤트를 갱신한다.
fn mark_step_started(
    runtime: &mut ScenarioRuntime,
    step_id: &str,
    sender: &UnboundedSender<EngineEvent>,
) {
    if let Some(state) = runtime.steps_state.get_mut(step_id) {
        state.status = StepStatus::Running;
        state.started_at = Some(std::time::Instant::now());
    }
    let _ = sender.send(EngineEvent::StepStarted {
        step_id: step_id.to_string(),
    });
}

/// 선행 Step 실패로 인해 더 이상 실행할 수 없는 Step을 실패 처리한다.
fn mark_blocked_steps(
    scenario: &Scenario,
    runtime: &mut ScenarioRuntime,
    started: &mut HashSet<String>,
    failed: &mut HashSet<String>,
    sender: &UnboundedSender<EngineEvent>,
) {
    for step in &scenario.steps {
        if started.contains(&step.id) {
            continue;
        }
        if step.depends_on.iter().any(|dep| failed.contains(dep)) {
            started.insert(step.id.clone());
            failed.insert(step.id.clone());
            if let Some(state) = runtime.steps_state.get_mut(&step.id) {
                state.status = StepStatus::Failed("선행 Step 실패로 건너뜀".into());
                state.finished_at = Some(std::time::Instant::now());
            }
            let _ = sender.send(EngineEvent::StepLog {
                step_id: step.id.clone(),
                line: "선행 Step 실패로 인해 실행하지 않습니다.".into(),
            });
            let _ = sender.send(EngineEvent::StepFinished {
                step_id: step.id.clone(),
                success: false,
            });
        }
    }
}

/// Step 실행 결과를 반영하고 이벤트를 송신한다.
fn apply_result(
    result: StepRunResult,
    runtime: &mut ScenarioRuntime,
    step_id: &str,
    succeeded: &mut HashSet<String>,
    failed: &mut HashSet<String>,
    sender: &UnboundedSender<EngineEvent>,
) {
    match result {
        StepRunResult::Success => {
            succeeded.insert(step_id.to_string());
            if let Some(state) = runtime.steps_state.get_mut(step_id) {
                state.status = StepStatus::Success;
                state.finished_at = Some(std::time::Instant::now());
            }
            let _ = sender.send(EngineEvent::StepFinished {
                step_id: step_id.to_string(),
                success: true,
            });
        }
        StepRunResult::Failed(msg) => {
            failed.insert(step_id.to_string());
            if let Some(state) = runtime.steps_state.get_mut(step_id) {
                state.status = StepStatus::Failed(msg.clone());
                state.finished_at = Some(std::time::Instant::now());
            }
            let _ = sender.send(EngineEvent::StepLog {
                step_id: step_id.to_string(),
                line: msg,
            });
            let _ = sender.send(EngineEvent::StepFinished {
                step_id: step_id.to_string(),
                success: false,
            });
        }
    }
}

/// 의존성이 모두 충족된 Step을 추출한다.
fn collect_ready_steps(
    scenario: &Scenario,
    started: &HashSet<String>,
    succeeded: &HashSet<String>,
    failed: &HashSet<String>,
) -> Vec<Step> {
    scenario
        .steps
        .iter()
        .filter(|step| {
            !started.contains(&step.id)
                && step.depends_on.iter().all(|dep| succeeded.contains(dep))
                && step.depends_on.iter().all(|dep| !failed.contains(dep))
        })
        .cloned()
        .collect()
}
