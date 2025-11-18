use super::events::EngineEvent;
use crate::executor::{SharedExecutor, run_sqlldr};
use crate::scenario::{Step, StepKind};
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
