use crate::engine::EngineEvent;
use crate::scenario::StepKind;
use anyhow::Context;
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::timeout;

/// DbExecutor는 SQL 실행을 위한 추상 계층을 정의한다.
#[async_trait]
pub trait DbExecutor: Send + Sync {
    /// SQL 문장을 실행한다.
    async fn execute_sql(&self, sql: &str) -> anyhow::Result<()>;
}

/// DummyExecutor는 실제 DB 연결 없이 로그만 출력하는 기본 구현이다.
#[derive(Debug, Default, Clone)]
pub struct DummyExecutor;

#[async_trait]
impl DbExecutor for DummyExecutor {
    /// Dummy 구현으로 SQL을 stdout으로 출력한다.
    async fn execute_sql(&self, sql: &str) -> anyhow::Result<()> {
        println!("[DummyExecutor] SQL 실행: {sql}");
        Ok(())
    }
}

/// DbExecutor를 공유하기 위한 Arc 타입 별칭이다.
pub type SharedExecutor = Arc<dyn DbExecutor>;

/// sqlldr 프로세스를 실행하고 로그를 EngineEvent로 전달한다.
pub async fn run_sqlldr(
    par_path: &Path,
    conn: &str,
    timeout_duration: Duration,
    sender: &UnboundedSender<EngineEvent>,
    step_id: &str,
) -> anyhow::Result<()> {
    let mut command = Command::new("sqlldr");
    command
        .arg(conn)
        .arg(format!("control={}", par_path.display()));
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    let mut child = command
        .spawn()
        .with_context(|| format!("sqlldr 실행 실패: {}", par_path.display()))?;
    if let Some(stdout) = child.stdout.take() {
        let tx = sender.clone();
        let id = step_id.to_string();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = tx.send(EngineEvent::StepLog {
                    step_id: id.clone(),
                    line: format!("sqlldr STDOUT: {line}"),
                });
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let tx = sender.clone();
        let id = step_id.to_string();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = tx.send(EngineEvent::StepLog {
                    step_id: id.clone(),
                    line: format!("sqlldr STDERR: {line}"),
                });
            }
        });
    }
    let status = timeout(timeout_duration, child.wait()).await??;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("sqlldr 종료 코드: {status}"))
    }
}
