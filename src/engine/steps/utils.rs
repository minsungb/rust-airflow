use super::super::context::SharedExecutionContext;
use super::super::events::EngineEvent;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc::UnboundedSender;

/// Step 로그를 전송한다.
pub(super) fn log_step(sender: &UnboundedSender<EngineEvent>, step_id: &str, line: &str) {
    let _ = sender.send(EngineEvent::StepLog {
        step_id: step_id.to_string(),
        line: line.to_string(),
    });
}

/// 경로 정보를 보기 좋게 변환한다.
pub(super) fn display_path(path: &PathBuf) -> String {
    path.to_string_lossy().to_string()
}

/// 필수 경로 값을 치환한다.
pub(super) async fn expand_path(
    path: &PathBuf,
    ctx: SharedExecutionContext,
    field: &str,
) -> anyhow::Result<String> {
    let raw = path.to_string_lossy().to_string();
    let guard = ctx.read().await;
    guard.expand_required(&raw, field)
}

/// 선택 경로 값을 치환한다.
pub(super) async fn expand_option_path(
    path: Option<&PathBuf>,
    ctx: SharedExecutionContext,
    field: &str,
) -> anyhow::Result<Option<String>> {
    match path {
        Some(p) => expand_path(p, ctx, field).await.map(Some),
        None => Ok(None),
    }
}

/// 프로세스 파이프를 읽어 로그 이벤트로 중계한다.
///
/// # 인자
/// - `reader`: STDOUT/STDERR 스트림을 비동기로 읽을 리더
/// - `sender`: 로그 이벤트를 전달할 채널 송신자
/// - `step_id`: 로그를 구분하기 위한 스텝 식별자
/// - `tag`: STDOUT/STDERR 태그 문자열
pub(super) async fn pipe_forwarder<R>(
    reader: R,
    sender: UnboundedSender<EngineEvent>,
    step_id: String,
    tag: &'static str,
) where
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::new();

    loop {
        buffer.clear();
        match reader.read_until(b'\n', &mut buffer).await {
            Ok(0) => break,
            Ok(_) => {
                while let Some(last) = buffer.last() {
                    if *last == b'\n' || *last == b'\r' {
                        buffer.pop();
                    } else {
                        break;
                    }
                }

                let line = String::from_utf8_lossy(&buffer);
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
