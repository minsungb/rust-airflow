use crate::executor::{SharedExecutor, run_sqlldr};
use crate::scenario::{Scenario, Step, StepKind};
use anyhow::Context;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

/// 엔진에서 UI로 전달되는 주요 이벤트 모델이다.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    /// Step 시작 알림.
    StepStarted { step_id: String },
    /// Step별 로그 라인.
    StepLog { step_id: String, line: String },
    /// Step 종료 알림.
    StepFinished { step_id: String, success: bool },
    /// 전체 시나리오 종료.
    ScenarioFinished,
}

/// Step의 런타임 상태를 표현한다.
#[derive(Debug, Clone)]
pub enum StepStatus {
    /// 아직 실행 대기 상태.
    Pending,
    /// 실행 중.
    Running,
    /// 정상 종료.
    Success,
    /// 실패와 함께 오류 메시지를 포함한다.
    Failed(String),
}

/// Step의 시간 및 로그 정보를 담는다.
#[derive(Debug, Clone)]
pub struct StepRuntimeState {
    /// 현재 상태 값.
    pub status: StepStatus,
    /// 시작 시각.
    pub started_at: Option<Instant>,
    /// 종료 시각.
    pub finished_at: Option<Instant>,
    /// 메모리에 적재된 로그 버퍼.
    pub logs: Vec<String>,
}

impl StepRuntimeState {
    /// 초기 상태를 생성한다.
    pub fn new() -> Self {
        Self {
            status: StepStatus::Pending,
            started_at: None,
            finished_at: None,
            logs: Vec::new(),
        }
    }
}

impl Default for StepRuntimeState {
    /// StepRuntimeState의 기본값을 정의한다.
    fn default() -> Self {
        Self::new()
    }
}

/// Scenario 실행 중 Step 상태 맵을 관리한다.
#[derive(Debug, Clone)]
pub struct ScenarioRuntime {
    /// 원본 시나리오 정의.
    pub scenario: Scenario,
    /// Step별 상태 맵.
    pub steps_state: HashMap<String, StepRuntimeState>,
}

impl ScenarioRuntime {
    /// Scenario를 받아 초기 상태를 생성한다.
    pub fn new(scenario: Scenario) -> Self {
        let steps_state = scenario
            .steps
            .iter()
            .map(|step| (step.id.clone(), StepRuntimeState::new()))
            .collect();
        Self {
            scenario,
            steps_state,
        }
    }
}

/// Scenario 전체를 실행하고 이벤트를 송신한다.
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
        state.started_at = Some(Instant::now());
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
                state.finished_at = Some(Instant::now());
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
                state.finished_at = Some(Instant::now());
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
                state.finished_at = Some(Instant::now());
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

#[derive(Debug)]
enum StepRunResult {
    Success,
    Failed(String),
}

/// 단일 Step을 실행하고 결과를 반환한다.
async fn run_single_step(
    step: Step,
    executor: SharedExecutor,
    sender: UnboundedSender<EngineEvent>,
    cancel: CancellationToken,
) -> StepRunResult {
    let timeout_duration = Duration::from_secs(step.timeout_sec.max(1));
    let mut attempt: u8 = 0;
    loop {
        if cancel.is_cancelled() {
            return StepRunResult::Failed("사용자에 의해 실행이 중단되었습니다.".to_string());
        }
        let backoff = Duration::from_secs(2_u64.pow(attempt as u32));
        let _ = sender.send(EngineEvent::StepLog {
            step_id: step.id.clone(),
            line: format!("[{}] {}차 시도", step.name, attempt + 1),
        });
        let exec_future = execute_step_kind(&step, executor.clone(), sender.clone());
        let result = tokio::time::timeout(timeout_duration, exec_future).await;
        match result {
            Ok(Ok(())) => return StepRunResult::Success,
            Ok(Err(err)) => {
                attempt += 1;
                if attempt > step.retry {
                    return StepRunResult::Failed(format!("실패: {err}"));
                }
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
    executor: SharedExecutor,
    sender: UnboundedSender<EngineEvent>,
) -> anyhow::Result<()> {
    match &step.kind {
        StepKind::Sql { sql } => {
            let _ = sender.send(EngineEvent::StepLog {
                step_id: step.id.clone(),
                line: "SQL 실행 시작".into(),
            });
            executor.execute_sql(sql).await?;
        }
        StepKind::SqlFile { path } => {
            let sql = fs::read_to_string(path)
                .await
                .with_context(|| format!("SQL 파일 읽기 실패: {}", path.display()))?;
            executor.execute_sql(&sql).await?;
        }
        StepKind::SqlLoaderPar { path } => {
            run_sqlldr(
                path,
                "DB_CONN",
                Duration::from_secs(step.timeout_sec.max(1)),
                &sender,
                &step.id,
            )
            .await?;
        }
        StepKind::Shell { shell } => {
            run_shell_command(
                shell,
                &sender,
                &step.id,
                Duration::from_secs(step.timeout_sec.max(1)),
            )
            .await?;
        }
    }
    Ok(())
}

/// 쉘 명령을 실행하고 실시간 로그를 전달한다.
async fn run_shell_command(
    command_str: &str,
    sender: &UnboundedSender<EngineEvent>,
    step_id: &str,
    timeout_duration: Duration,
) -> anyhow::Result<()> {
    let mut command = if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg(command_str);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command_str);
        cmd
    };
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    let mut child = command
        .spawn()
        .with_context(|| format!("쉘 명령 실행 실패: {command_str}"))?;
    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(pipe_forwarder(
            stdout,
            sender.clone(),
            step_id.to_string(),
            "STDOUT",
        ));
    }
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(pipe_forwarder(
            stderr,
            sender.clone(),
            step_id.to_string(),
            "STDERR",
        ));
    }
    let status = tokio::time::timeout(timeout_duration, child.wait()).await??;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("쉘 명령 종료 코드: {status}"))
    }
}

/// 프로세스 파이프를 읽어 로그 이벤트로 중계한다.
async fn pipe_forwarder<R>(
    reader: R,
    sender: UnboundedSender<EngineEvent>,
    step_id: String,
    tag: &'static str,
) where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let _ = sender.send(EngineEvent::StepLog {
            step_id: step_id.clone(),
            line: format!("{tag}: {line}"),
        });
    }
}
