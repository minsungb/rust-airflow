use super::events::EngineEvent;
use crate::executor::{SharedExecutor, run_sqlldr};
use crate::scenario::{ShellConfig, ShellErrorPolicy, Step, StepKind};
use anyhow::Context;
use futures::StreamExt;
use std::time::Duration;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::sync::CancellationToken;

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
        StepKind::SqlLoaderPar { config } => {
            run_sqlldr(
                config,
                Duration::from_secs(step.timeout_sec.max(1)),
                &sender,
                &step.id,
            )
            .await?;
        }
        StepKind::Shell { config } => {
            run_shell_command(
                config,
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
    config: &ShellConfig,
    sender: &UnboundedSender<EngineEvent>,
    step_id: &str,
    timeout_duration: Duration,
) -> anyhow::Result<()> {
    let program = config.shell_program.clone().unwrap_or_else(|| {
        if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "sh"
        }
        .to_string()
    });
    let mut command = Command::new(program);
    if cfg!(target_os = "windows") {
        command.arg("/C");
    } else {
        command.arg("-c");
    }
    command.arg(&config.script);
    if !config.shell_args.is_empty() {
        command.args(&config.shell_args);
    }
    if let Some(dir) = &config.working_dir {
        command.current_dir(dir);
    }
    if !config.env.is_empty() {
        command.envs(&config.env);
    }
    if let Some(user) = &config.run_as {
        apply_user_context(&mut command, user)?;
    }
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    let mut child = command
        .spawn()
        .with_context(|| format!("쉘 명령 실행 실패: {}", config.script))?;
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
        return Ok(());
    }
    if config.error_policy == ShellErrorPolicy::Ignore {
        let _ = sender.send(EngineEvent::StepLog {
            step_id: step_id.to_string(),
            line: format!("비정상 종료 코드 {status}, 정책에 따라 무시"),
        });
        Ok(())
    } else {
        Err(anyhow::anyhow!("쉘 명령 종료 코드: {status}"))
    }
}

/// 플랫폼별 사용자 실행 맥락을 적용한다.
fn apply_user_context(command: &mut Command, user: &str) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let (uid, gid) = lookup_unix_user(user)?;
        command.uid(uid);
        command.gid(gid);
        return Ok(());
    }
    #[cfg(not(unix))]
    {
        Err(anyhow::anyhow!(
            "run_as는 현재 운영체제에서 지원되지 않습니다."
        ))
    }
}

/// /etc/passwd에서 사용자 UID/GID를 조회한다.
#[cfg(unix)]
fn lookup_unix_user(user: &str) -> anyhow::Result<(u32, u32)> {
    let content = std::fs::read_to_string("/etc/passwd")
        .with_context(|| "/etc/passwd 파일을 읽을 수 없습니다.".to_string())?;
    for line in content.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 4 {
            continue;
        }
        if parts[0] == user {
            let uid: u32 = parts[2]
                .parse()
                .with_context(|| format!("UID 파싱 실패: {}", parts[2]))?;
            let gid: u32 = parts[3]
                .parse()
                .with_context(|| format!("GID 파싱 실패: {}", parts[3]))?;
            return Ok((uid, gid));
        }
    }
    Err(anyhow::anyhow!("사용자 {user} 정보를 찾을 수 없습니다."))
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
    let mut lines = FramedRead::new(reader, LinesCodec::new());
    while let Some(line_result) = lines.next().await {
        match line_result {
            Ok(line) => {
                let _ = sender.send(EngineEvent::StepLog {
                    step_id: step_id.clone(),
                    line: format!("{tag}: {line}"),
                });
            }
            Err(err) => {
                let _ = sender.send(EngineEvent::StepLog {
                    step_id: step_id.clone(),
                    line: format!("{tag} 읽기 오류: {err}"),
                });
                break;
            }
        }
    }
}
