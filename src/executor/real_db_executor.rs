use super::{DbExecutor, SharedExecutor};
use anyhow::{Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Config as PoolConfig, ManagerConfig, Pool, RecyclingMethod, Runtime};
use std::sync::Arc;
use tokio_postgres::NoTls;

/// RealDbExecutor는 PostgreSQL 연결 풀을 통해 SQL을 실행하는 실제 구현체이다.
#[derive(Clone)]
pub struct RealDbExecutor {
    /// 연결 문자열을 보관한다.
    dsn: String,
    /// 접속 사용자명을 보관한다.
    user: Option<String>,
    /// 접속 비밀번호를 보관한다.
    password: Option<String>,
    /// deadpool 기반 연결 풀이다.
    pool: Pool,
}

impl RealDbExecutor {
    /// 주어진 접속 정보를 기반으로 PostgreSQL 연결 풀을 초기화한다.
    ///
    /// # 매개변수
    /// - `dsn`: `host`, `port`, `dbname` 등이 포함된 PostgreSQL DSN 문자열.
    /// - `user`: 데이터베이스 사용자명.
    /// - `password`: 해당 사용자 비밀번호.
    ///
    /// # 반환값
    /// 초기화된 `RealDbExecutor` 인스턴스를 포함한 [`Result`]를 반환한다.
    pub async fn new(
        dsn: impl Into<String>,
        user: Option<String>,
        password: Option<String>,
    ) -> Result<Self> {
        let dsn = dsn.into();
        let mut config = PoolConfig::new();
        config.url = Some(dsn.clone());
        config.user = user.clone();
        config.password = password.clone();
        config.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        let pool = config
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .context("PostgreSQL 커넥션 풀 생성 실패")?;

        Ok(Self {
            dsn,
            user,
            password,
            pool,
        })
    }
}

#[async_trait]
impl DbExecutor for RealDbExecutor {
    /// deadpool 풀에서 커넥션을 획득해 주어진 SQL을 실행한다.
    ///
    /// # 매개변수
    /// - `sql`: 실행할 SQL 문자열.
    ///
    /// # 반환값
    /// 실행 결과에 따라 성공 또는 오류를 반환한다.
    async fn execute_sql(&self, sql: &str) -> Result<()> {
        let client = self
            .pool
            .get()
            .await
            .context("PostgreSQL 커넥션 획득 실패")?;
        client
            .batch_execute(sql)
            .await
            .context("PostgreSQL SQL 실행 실패")?;
        Ok(())
    }
}

/// RealDbExecutor를 [`SharedExecutor`] 형태로 감싸 애플리케이션에서 쉽게 사용할 수 있게 한다.
///
/// # 매개변수
/// - `dsn`: PostgreSQL DSN 문자열.
/// - `user`: 데이터베이스 사용자명.
/// - `password`: 사용자 비밀번호.
///
/// # 반환값
/// 생성된 실행기를 담은 [`SharedExecutor`]를 반환한다.
pub async fn new_real_db_executor(
    dsn: String,
    user: Option<String>,
    password: Option<String>,
) -> Result<SharedExecutor> {
    let executor = RealDbExecutor::new(dsn, user, password).await?;
    Ok(Arc::new(executor) as SharedExecutor)
}
