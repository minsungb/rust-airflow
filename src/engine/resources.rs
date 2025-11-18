use super::context::SharedExecutionContext;
use crate::executor::{
    DummyExecutor, SharedExecutor, new_oracle_db_executor, new_real_db_executor,
};
use crate::scenario::{DbConnectionConfig, DbKind, Scenario};
use anyhow::Context;
use std::collections::HashMap;
use std::sync::Arc;

/// 엔진 실행 중 필요한 공용 리소스를 캡슐화한다.
#[derive(Debug, Clone)]
pub struct EngineHandles {
    /// DB 이름별 실행기 맵이다.
    pub(crate) db_map: HashMap<String, SharedExecutor>,
}

impl EngineHandles {
    /// 지정한 DB 타겟에 대한 실행기를 반환한다.
    pub fn get_db_executor(&self, name: &str) -> anyhow::Result<SharedExecutor> {
        self.db_map
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!(format!("정의되지 않은 DB 타겟: {name}")))
    }
}

/// Scenario 정의를 기반으로 DB 실행기 맵을 구성한다.
pub async fn prepare_engine_handles(
    scenario: &Scenario,
    default_executor: SharedExecutor,
    ctx: SharedExecutionContext,
) -> anyhow::Result<EngineHandles> {
    let mut db_map: HashMap<String, SharedExecutor> = HashMap::new();
    db_map.insert("default".to_string(), default_executor);
    for (name, config) in &scenario.db {
        let executor = build_executor_from_config(config, ctx.clone())
            .await
            .with_context(|| format!("DB 실행기 생성 실패: {name}"))?;
        db_map.insert(name.clone(), executor);
    }
    Ok(EngineHandles { db_map })
}

async fn build_executor_from_config(
    config: &DbConnectionConfig,
    ctx: SharedExecutionContext,
) -> anyhow::Result<SharedExecutor> {
    match config.kind {
        DbKind::Dummy => Ok(Arc::new(DummyExecutor::default()) as SharedExecutor),
        DbKind::Postgres => {
            let dsn = expand_required(ctx.clone(), config.dsn.clone(), "dsn").await?;
            let user = expand_optional(ctx.clone(), config.user.clone(), "user").await?;
            let password =
                expand_optional(ctx.clone(), config.password.clone(), "password").await?;
            new_real_db_executor(dsn, user, password).await
        }
        DbKind::Oracle => {
            let dsn = expand_required(ctx.clone(), config.dsn.clone(), "dsn").await?;
            let user = expand_required(ctx.clone(), config.user.clone(), "user").await?;
            let password = expand_required(ctx, config.password.clone(), "password").await?;
            Ok(new_oracle_db_executor(dsn, user, password))
        }
    }
}

async fn expand_required(
    ctx: SharedExecutionContext,
    value: Option<String>,
    field: &str,
) -> anyhow::Result<String> {
    let raw = value.ok_or_else(|| anyhow::anyhow!(format!("{field} 값이 누락되었습니다.")))?;
    let guard = ctx.read().await;
    guard.expand_required(&raw, field)
}

async fn expand_optional(
    ctx: SharedExecutionContext,
    value: Option<String>,
    field: &str,
) -> anyhow::Result<Option<String>> {
    match value {
        Some(raw) => {
            let guard = ctx.read().await;
            guard.expand_required(&raw, field).map(Some)
        }
        None => Ok(None),
    }
}
