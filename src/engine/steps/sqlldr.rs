use super::super::context::SharedExecutionContext;
use super::super::events::EngineEvent;
use crate::scenario::SqlLoaderParConfig;
use anyhow::Context;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;

use super::utils::{expand_option_path, expand_path, pipe_forwarder};

/// sqlldr 프로세스를 실행한다.
pub(super) async fn run_sqlldr(
    config: &SqlLoaderParConfig,
    ctx: SharedExecutionContext,
    sender: &UnboundedSender<EngineEvent>,
    step_id: &str,
    timeout_duration: Duration,
) -> anyhow::Result<()> {
    let conn = if let Some(conn) = &config.conn {
        let guard = ctx.read().await;
        guard.expand_required(conn, "sqlldr.conn")?
    } else {
        let guard = ctx.read().await;
        guard
            .get_or_env("SQLLDR_CONN")
            .ok_or_else(|| anyhow::anyhow!("SQLLDR_CONN 값을 찾을 수 없습니다."))?
    };
    let control = expand_path(&config.control_file, ctx.clone(), "control").await?;
    let data = expand_option_path(config.data_file.as_ref(), ctx.clone(), "data").await?;
    let log = expand_option_path(config.log_file.as_ref(), ctx.clone(), "log").await?;
    let bad = expand_option_path(config.bad_file.as_ref(), ctx.clone(), "bad").await?;
    let discard = expand_option_path(config.discard_file.as_ref(), ctx, "discard").await?;
    let mut command = Command::new("sqlldr");
    command.arg(conn);
    command.arg(format!("control={control}"));
    if let Some(val) = data {
        command.arg(format!("data={val}"));
    }
    if let Some(val) = log {
        command.arg(format!("log={val}"));
    }
    if let Some(val) = bad {
        command.arg(format!("bad={val}"));
    }
    if let Some(val) = discard {
        command.arg(format!("discard={val}"));
    }
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    let mut child = command.spawn().context("sqlldr 실행 실패")?;
    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(pipe_forwarder(
            stdout,
            sender.clone(),
            step_id.to_string(),
            "sqlldr STDOUT",
        ));
    }
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(pipe_forwarder(
            stderr,
            sender.clone(),
            step_id.to_string(),
            "sqlldr STDERR",
        ));
    }
    let status = tokio::time::timeout(timeout_duration, child.wait()).await??;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("sqlldr 종료 코드: {status}"))
    }
}
