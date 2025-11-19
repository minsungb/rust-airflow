use super::super::context::SharedExecutionContext;
use super::super::events::EngineEvent;
use super::super::resources::EngineHandles;
use crate::scenario::{LoopStepConfig, Step};
use anyhow::Context;
use futures::future::BoxFuture;
use glob::glob;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use super::confirm::{ConfirmPhase, evaluate_confirm};
use super::utils::log_step;

/// Loop Step을 실행한다.
pub(super) async fn execute_loop_step(
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
fn run_embedded_step<'a>(
    step: &'a Step,
    log_step_id: &'a str,
    handles: Arc<EngineHandles>,
    ctx: SharedExecutionContext,
    sender: UnboundedSender<EngineEvent>,
    cancel: CancellationToken,
) -> BoxFuture<'a, anyhow::Result<()>> {
    Box::pin(async move {
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

            let exec_future = super::execute_step_kind(
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
                        if !evaluate_confirm(
                            step,
                            log_step_id,
                            confirm,
                            ConfirmPhase::After,
                            &sender,
                        ) {
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::context::ExecutionContext;
    use crate::engine::resources::EngineHandles;
    use crate::executor::{DbExecutor, SharedExecutor};
    use crate::scenario::{Step, StepKind};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::sync::Mutex;

    /// 실행된 SQL 문장을 누적 기록하는 목업 실행기이다.
    #[derive(Clone)]
    struct RecordingExecutor {
        /// Step 실행 시 받은 SQL 목록이다.
        executed: std::sync::Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl DbExecutor for RecordingExecutor {
        /// 전달받은 SQL을 내부 벡터에 저장한다.
        async fn execute_sql(&self, sql: &str) -> anyhow::Result<()> {
            let mut guard = self.executed.lock().await;
            guard.push(sql.to_string());
            Ok(())
        }
    }

    /// Loop Step에서 설정한 변수 값이 하위 Step SQL 파라미터에 반영되는지 검증한다.
    #[tokio::test]
    async fn loop_step_propagates_context_variables_to_child_steps() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("시스템 시간이 UTC epoch 이전입니다.")
            .as_nanos();
        let base_dir = std::env::temp_dir().join(format!("loop_step_test_{timestamp}"));
        std::fs::create_dir_all(&base_dir).expect("임시 디렉터리 생성 실패");
        let file_a = base_dir.join("file_a.txt");
        let file_b = base_dir.join("file_b.txt");
        std::fs::write(&file_a, "a").expect("파일 작성 실패");
        std::fs::write(&file_b, "b").expect("파일 작성 실패");
        let pattern = format!("{}/file_*.txt", base_dir.display());

        let child_step = Step {
            id: "child".into(),
            name: "Loop Child".into(),
            kind: StepKind::Sql {
                sql: "INSERT ${CURRENT_FILE}".into(),
                target_db: None,
            },
            depends_on: Vec::new(),
            allow_parallel: false,
            retry: 0,
            timeout_sec: 60,
            confirm: None,
        };

        let config = LoopStepConfig {
            for_each_glob: pattern,
            as_var: "CURRENT_FILE".into(),
            steps: vec![child_step],
        };

        let ctx: SharedExecutionContext =
            std::sync::Arc::new(tokio::sync::RwLock::new(ExecutionContext::new()));
        let recorded_sql = std::sync::Arc::new(Mutex::new(Vec::new()));
        let executor = RecordingExecutor {
            executed: recorded_sql.clone(),
        };
        let mut db_map: HashMap<String, SharedExecutor> = HashMap::new();
        db_map.insert("default".into(), std::sync::Arc::new(executor) as SharedExecutor);
        let handles = std::sync::Arc::new(EngineHandles { db_map });
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let cancel = CancellationToken::new();

        execute_loop_step(&config, "loop", handles, ctx, tx, cancel)
            .await
            .expect("Loop Step 실행 실패");

        let mut actual = {
            let guard = recorded_sql.lock().await;
            guard.clone()
        };
        actual.sort();
        let mut expected = vec![
            format!("INSERT {}", file_a.to_string_lossy()),
            format!("INSERT {}", file_b.to_string_lossy()),
        ];
        expected.sort();
        assert_eq!(expected, actual);

        let _ = std::fs::remove_dir_all(&base_dir);
    }
}
