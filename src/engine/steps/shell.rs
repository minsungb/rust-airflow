use super::super::context::SharedExecutionContext;
use super::super::events::EngineEvent;
use crate::scenario::{ShellConfig, ShellErrorPolicy};
use anyhow::Context;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;

use super::utils::log_step;
use super::utils::pipe_forwarder;

/// 쉘 명령을 실행하고 실시간 로그를 전달한다.
///
/// # 인자
/// - `config`: 쉘 프로그램, 스크립트, 환경 변수 설정을 담은 구성
/// - `ctx`: 변수 치환에 사용되는 실행 컨텍스트 공유 포인터
/// - `sender`: 로그 및 이벤트를 내보낼 채널 송신자
/// - `step_id`: 실행 중인 스텝의 식별자 문자열
/// - `timeout_duration`: 명령 최대 대기 시간
///
/// # 반환값
/// 성공 시 `Ok(())`를 반환하며, 실행 실패 또는 재시도 초과 시 에러를 반환한다.
pub(super) async fn run_shell_command(
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
///
/// # 인자
/// - `command`: UID/GID를 설정할 프로세스 명령 빌더
/// - `user`: 실행할 사용자의 계정 이름
///
/// # 반환값
/// UID/GID 설정 성공 시 `Ok(())`, 지원하지 않거나 조회 실패 시 에러를 반환한다.
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
///
/// # 인자
/// - `user`: 검색할 사용자 계정명
///
/// # 반환값
/// `(uid, gid)` 튜플을 반환하며, 존재하지 않을 경우 에러를 반환한다.
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
