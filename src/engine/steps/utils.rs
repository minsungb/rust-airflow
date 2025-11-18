use super::super::context::SharedExecutionContext;
use super::super::events::EngineEvent;
use futures::StreamExt;
use std::path::PathBuf;
use tokio::io::AsyncRead;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::codec::{FramedRead, LinesCodec};

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
pub(super) async fn pipe_forwarder<R>(
    reader: R,
    sender: UnboundedSender<EngineEvent>,
    step_id: String,
    tag: &'static str,
) where
    R: AsyncRead + Unpin + Send + 'static,
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
