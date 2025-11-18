use async_trait::async_trait;
use std::sync::Arc;

mod oracle_db_executor;
mod real_db_executor;
pub use oracle_db_executor::{OracleDbExecutor, new_oracle_db_executor};
pub use real_db_executor::{RealDbExecutor, new_real_db_executor};

/// DbExecutor는 SQL 실행을 위한 추상 계층을 정의한다.
#[async_trait]
pub trait DbExecutor: Send + Sync {
    /// SQL 문장을 실행한다.
    async fn execute_sql(&self, sql: &str) -> anyhow::Result<()>;
}

/// DummyExecutor는 실제 DB 연결 없이 로그만 출력하는 기본 구현이다.
#[derive(Debug, Default, Clone)]
pub struct DummyExecutor;

#[async_trait]
impl DbExecutor for DummyExecutor {
    /// Dummy 구현으로 SQL을 stdout으로 출력한다.
    async fn execute_sql(&self, sql: &str) -> anyhow::Result<()> {
        println!("[DummyExecutor] SQL 실행: {sql}");
        Ok(())
    }
}

/// DbExecutor를 공유하기 위한 Arc 타입 별칭이다.
pub type SharedExecutor = Arc<dyn DbExecutor>;
