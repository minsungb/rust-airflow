use super::context::SharedExecutionContext;
use super::events::EngineEvent;
use super::resources::EngineHandles;
use crate::scenario::{
    ConfirmDefault, ExtractVarFromFileConfig, LoopStepConfig, ShellConfig, ShellErrorPolicy,
    SqlLoaderParConfig, Step, StepConfirmConfig, StepKind,
};
use anyhow::Context;
use futures::StreamExt;
use glob::glob;
use regex::Regex;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
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

/// SQL 문자열을 실행한다.
async fn execute_sql(
    sql: &str,
    target_db: Option<&str>,
    handles: Arc<EngineHandles>,
    ctx: SharedExecutionContext,
) -> anyhow::Result<()> {
    let expanded_sql = {
        let guard = ctx.read().await;
        guard.expand_required(sql, "sql")?
    };
    let target = target_db.unwrap_or("default");
    let executor = handles.get_db_executor(target)?;
    executor.execute_sql(&expanded_sql).await
}

/// SQL 파일을 읽어 문자열을 반환한다.
async fn load_sql_file(path: &PathBuf, ctx: SharedExecutionContext) -> anyhow::Result<String> {
    let raw = path.to_string_lossy().to_string();
    let actual_path = {
        let guard = ctx.read().await;
        guard.expand_required(&raw, "sql_file")?
    };
    let content = fs::read_to_string(&actual_path)
        .await
        .with_context(|| format!("SQL 파일 읽기 실패: {actual_path}"))?;
    let guard = ctx.read().await;
    guard.expand_required(&content, "sql_file_content")
}

/// sqlldr 프로세스를 실행한다.
async fn run_sqlldr(
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

/// Extract Step을 실행한다.
async fn execute_extract_step(
    config: &ExtractVarFromFileConfig,
    ctx: SharedExecutionContext,
    step_id: &str,
    sender: &UnboundedSender<EngineEvent>,
) -> anyhow::Result<()> {
    let file_path = {
        let guard = ctx.read().await;
        guard.expand_required(&config.file_path, "extract.file_path")?
    };
    let file = File::open(&file_path)
        .await
        .with_context(|| format!("파일을 열 수 없습니다: {file_path}"))?;
    let mut reader = BufReader::new(file).lines();
    let mut current_line = None;
    for i in 1..=config.line {
        if let Some(line) = reader.next_line().await? {
            if i == config.line {
                current_line = Some(line);
                break;
            }
        } else {
            anyhow::bail!("{file_path}에서 {}번째 줄을 찾을 수 없습니다.", config.line);
        }
    }
    let content = current_line.unwrap_or_default();
    let re = Regex::new(&config.pattern)
        .with_context(|| format!("정규식 컴파일 실패: {}", config.pattern))?;
    let captures = re
        .captures(&content)
        .ok_or_else(|| anyhow::anyhow!("패턴이 매칭되지 않았습니다: {content}"))?;
    let value = captures
        .get(config.group)
        .ok_or_else(|| anyhow::anyhow!("캡처 그룹 {}을 찾을 수 없습니다.", config.group))?
        .as_str()
        .to_string();
    {
        let mut guard = ctx.write().await;
        guard.set_var(&config.var_name, &value);
    }
    log_step(
        sender,
        step_id,
        &format!("변수 {} = {}", config.var_name, value),
    );
    Ok(())
}

/// Loop Step을 실행한다.
async fn execute_loop_step(
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
async fn run_embedded_step(
    step: &Step,
    log_step_id: &str,
    handles: Arc<EngineHandles>,
    ctx: SharedExecutionContext,
    sender: UnboundedSender<EngineEvent>,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
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
        let exec_future = execute_step_kind(
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
                    if !evaluate_confirm(step, log_step_id, confirm, ConfirmPhase::After, &sender) {
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
}

/// 쉘 명령을 실행하고 실시간 로그를 전달한다.
async fn run_shell_command(
    config: &ShellConfig,
    ctx: SharedExecutionContext,
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
    let script = {
        let guard = ctx.read().await;
        guard.expand_required(&config.script, "shell.command")?
    };
    let shell_args = {
        let mut args = Vec::new();
        for arg in &config.shell_args {
            let guard = ctx.read().await;
            args.push(guard.expand_required(arg, "shell.arg")?);
        }
        args
    };
    let env_map = {
        let mut map = std::collections::HashMap::new();
        for (key, value) in &config.env {
            let guard = ctx.read().await;
            map.insert(key.clone(), guard.expand_required(value, "shell.env")?);
        }
        map
    };
    let working_dir = if let Some(dir) = &config.working_dir {
        let guard = ctx.read().await;
        Some(guard.expand_required(&dir.to_string_lossy(), "shell.working_dir")?)
    } else {
        None
    };
    let run_as = config.run_as.clone();
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        let mut command = Command::new(&program);
        if cfg!(target_os = "windows") {
            command.arg("/C");
            command.arg(&script);
        } else {
            command.arg("-c");
            command.arg(&script);
        }
        if !shell_args.is_empty() {
            command.args(&shell_args);
        }
        if let Some(dir) = &working_dir {
            command.current_dir(dir);
        }
        if !env_map.is_empty() {
            command.envs(&env_map);
        }
        if let Some(user) = &run_as {
            apply_user_context(&mut command, user)?;
        }
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        let mut child = command
            .spawn()
            .with_context(|| format!("쉘 명령 실행 실패: {script}"))?;
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
        match &config.error_policy {
            ShellErrorPolicy::Fail => {
                return Err(anyhow::anyhow!(format!("쉘 명령 종료 코드: {status}")));
            }
            ShellErrorPolicy::Ignore => {
                log_step(
                    sender,
                    step_id,
                    &format!("비정상 종료 코드 {status}, 정책에 따라 무시"),
                );
                return Ok(());
            }
            ShellErrorPolicy::Retry {
                max_retries,
                delay_secs,
            } => {
                if attempt > max_retries + 1 {
                    return Err(anyhow::anyhow!(format!(
                        "재시도 한도를 초과했습니다: {status}"
                    )));
                }
                log_step(
                    sender,
                    step_id,
                    &format!(
                        "쉘 명령 실패, {}초 후 재시도 ({}/{})",
                        delay_secs,
                        attempt,
                        max_retries + 1
                    ),
                );
                sleep(Duration::from_secs(*delay_secs)).await;
            }
        }
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
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = user;
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

/// Confirm 단계를 구분하기 위한 열거형이다.
enum ConfirmPhase {
    /// 실행 전 확인 단계.
    Before,
    /// 실행 후 확인 단계.
    After,
}

/// 컨펌 설정을 기반으로 기본 응답을 평가한다.
fn evaluate_confirm(
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
        line: format!(
            "[Confirm:{}] {msg} (기본응답: {default_text})",
            step.id
        ),
    });
    matches!(confirm.default_answer, ConfirmDefault::Yes)
}

/// Step 로그를 전송한다.
fn log_step(sender: &UnboundedSender<EngineEvent>, step_id: &str, line: &str) {
    let _ = sender.send(EngineEvent::StepLog {
        step_id: step_id.to_string(),
        line: line.to_string(),
    });
}

/// 경로 정보를 보기 좋게 변환한다.
fn display_path(path: &PathBuf) -> String {
    path.to_string_lossy().to_string()
}

/// 필수 경로 값을 치환한다.
async fn expand_path(
    path: &PathBuf,
    ctx: SharedExecutionContext,
    field: &str,
) -> anyhow::Result<String> {
    let raw = path.to_string_lossy().to_string();
    let guard = ctx.read().await;
    guard.expand_required(&raw, field)
}

/// 선택 경로 값을 치환한다.
async fn expand_option_path(
    path: Option<&PathBuf>,
    ctx: SharedExecutionContext,
    field: &str,
) -> anyhow::Result<Option<String>> {
    match path {
        Some(p) => expand_path(p, ctx, field).await.map(Some),
        None => Ok(None),
    }
}
