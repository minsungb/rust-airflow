use super::super::context::SharedExecutionContext;
use super::super::events::EngineEvent;
use encoding_rs::WINDOWS_949;
use std::borrow::Cow;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc::UnboundedSender;

/// Step 로그를 전송한다.
///
/// # 인자
/// - `sender`: 로그 이벤트를 내보낼 채널 송신자
/// - `step_id`: 로그가 속한 스텝의 식별자
/// - `line`: 전송할 로그 문자열
pub(super) fn log_step(sender: &UnboundedSender<EngineEvent>, step_id: &str, line: &str) {
    let _ = sender.send(EngineEvent::StepLog {
        step_id: step_id.to_string(),
        line: line.to_string(),
    });
}

/// 경로 정보를 보기 좋게 변환한다.
///
/// # 인자
/// - `path`: 문자열로 표현할 파일 경로 객체
///
/// # 반환값
/// 경로를 OS 문자열에서 유니코드로 손실 복원 변환한 문자열
pub(super) fn display_path(path: &PathBuf) -> String {
    path.to_string_lossy().to_string()
}

/// 필수 경로 값을 치환한다.
///
/// # 인자
/// - `path`: 템플릿 변수를 포함할 수 있는 경로 객체
/// - `ctx`: 시나리오 실행 컨텍스트 공유 포인터
/// - `field`: 오류 메시지용 필드 이름
///
/// # 반환값
/// 치환 완료된 경로 문자열. 치환에 실패하면 에러를 반환한다.
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
///
/// # 인자
/// - `path`: 존재할 수도 없는 경로 참조
/// - `ctx`: 시나리오 실행 컨텍스트 공유 포인터
/// - `field`: 오류 메시지용 필드 이름
///
/// # 반환값
/// 치환된 경로 문자열 옵션. 입력이 없으면 `None`, 실패 시 에러를 반환한다.
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
///
/// # 동작
/// UTF-8로 해석할 수 없는 바이트가 발견되면 Windows-949(구 CP949)로
/// 재시도하고, 그래도 실패하면 손실 복원 문자열로 전달한다.
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

                let line = decode_log_line(&buffer);
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

/// 로그 라인을 적절한 인코딩으로 변환한다.
///
/// # 인자
/// - `buffer`: STDOUT/STDERR에서 읽은 단일 라인 바이트 슬라이스
///
/// # 반환값
/// UTF-8 또는 Windows-949로 디코딩한 문자열. 두 인코딩 모두 실패하면
/// 손실 복원된 문자열을 반환한다.
fn decode_log_line(buffer: &[u8]) -> Cow<'_, str> {
    match std::str::from_utf8(buffer) {
        Ok(valid_utf8) => Cow::Borrowed(valid_utf8),
        Err(_) => {
            let (decoded, _, had_errors) = WINDOWS_949.decode(buffer);
            if had_errors {
                String::from_utf8_lossy(buffer)
            } else {
                decoded
            }
        }
    }
}
