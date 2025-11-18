use super::context::SharedExecutionContext;
use crate::executor::{
    DummyExecutor, SharedExecutor, new_oracle_db_executor, new_real_db_executor,
};
use crate::scenario::{DbConnectionConfig, DbKind, Scenario};
use anyhow::Context;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// 엔진 실행 중 필요한 공용 리소스를 캡슐화한다.
#[derive(Clone)]
pub struct EngineHandles {
    /// DB 이름별 실행기 맵이다.
    pub(crate) db_map: HashMap<String, SharedExecutor>,
}

impl fmt::Debug for EngineHandles {
    /// EngineHandles의 디버그 출력을 DB 이름 목록만 포함하도록 생성한다.
    ///
    /// # 매개변수
    /// * `f` - 문자열을 작성할 [`fmt::Formatter`] 참조
    ///
    /// # 반환값
    /// * [`fmt::Result`] - 포매팅 성공 여부
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let keys: Vec<&String> = self.db_map.keys().collect();
        f.debug_struct("EngineHandles")
            .field("db_map_keys", &keys)
            .finish()
    }
}

impl EngineHandles {
    /// 지정한 DB 타겟에 대한 실행기를 반환한다.
    ///
    /// # 매개변수
    /// * `name` - 조회할 DB 타겟 이름
    ///
    /// # 반환값
    /// * [`SharedExecutor`] - 타겟이 존재할 경우 실행기를 담은 스마트 포인터
    ///
    /// # 오류
    /// * 존재하지 않는 타겟을 요청하면 [`anyhow::Error`]를 반환한다.
    pub fn get_db_executor(&self, name: &str) -> anyhow::Result<SharedExecutor> {
        self.db_map
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!(format!("정의되지 않은 DB 타겟: {name}")))
    }
}

/// Scenario 정의를 기반으로 DB 실행기 맵을 구성한다.
///
/// # 매개변수
/// * `scenario` - DB 설정 정보를 포함한 시나리오
/// * `default_executor` - default 키로 등록할 기본 실행기
/// * `ctx` - 환경 변수를 확장하기 위한 실행 컨텍스트
///
/// # 반환값
/// * [`EngineHandles`] - 구축된 DB 실행기 맵을 담은 핸들러
///
/// # 오류
/// * 실행기 생성에 실패하면 [`anyhow::Error`]를 반환한다.
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

/// DB 연결 설정을 바탕으로 적절한 실행기를 생성한다.
///
/// # 매개변수
/// * `config` - 대상 DB 연결 정보
/// * `ctx` - 변수 확장을 수행할 실행 컨텍스트
///
/// # 반환값
/// * [`SharedExecutor`] - 생성된 실행기를 감싼 스마트 포인터
///
/// # 오류
/// * 연결 정보가 누락되었거나 생성 중 오류가 발생하면 [`anyhow::Error`]를 반환한다.
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

/// 필수 문자열 값을 컨텍스트 기반으로 확장한다.
///
/// # 매개변수
/// * `ctx` - 환경 값을 제공하는 실행 컨텍스트
/// * `value` - 원본 문자열 옵션
/// * `field` - 오류 메시지에 사용할 필드명
///
/// # 반환값
/// * `String` - 확장된 문자열
///
/// # 오류
/// * 값이 없거나 확장 실패 시 [`anyhow::Error`]를 반환한다.
async fn expand_required(
    ctx: SharedExecutionContext,
    value: Option<String>,
    field: &str,
) -> anyhow::Result<String> {
    let raw = value.ok_or_else(|| anyhow::anyhow!(format!("{field} 값이 누락되었습니다.")))?;
    let guard = ctx.read().await;
    guard.expand_required(&raw, field)
}

/// 선택적 문자열 값을 컨텍스트 기반으로 확장한다.
///
/// # 매개변수
/// * `ctx` - 환경 값을 제공하는 실행 컨텍스트
/// * `value` - 원본 문자열 옵션
/// * `field` - 오류 메시지에 사용할 필드명
///
/// # 반환값
/// * `Option<String>` - 확장된 문자열 또는 값이 없을 경우 `None`
///
/// # 오류
/// * 값이 존재하지만 확장에 실패하면 [`anyhow::Error`]를 반환한다.
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
