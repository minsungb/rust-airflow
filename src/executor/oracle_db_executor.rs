use super::{DbExecutor, SharedExecutor};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// OracleDbExecutor는 sqlplus 프로세스를 사용해 Oracle DB에 SQL을 실행하는 구현체이다.
#[derive(Clone, Debug)]
pub struct OracleDbExecutor {
    /// 접속에 사용할 DSN 문자열이다.
    dsn: String,
    /// 접속 사용자명이다.
    user: String,
    /// 접속 비밀번호이다.
    password: String,
}

impl OracleDbExecutor {
    /// OracleDbExecutor를 초기화하여 sqlplus 호출에 필요한 정보를 보관한다.
    ///
    /// # 매개변수
    /// - `dsn`: `HOST:PORT/SERVICE_NAME` 형식의 Oracle DSN 문자열.
    /// - `user`: 데이터베이스 사용자명.
    /// - `password`: 해당 사용자의 비밀번호.
    ///
    /// # 반환값
    /// 생성된 [`OracleDbExecutor`] 인스턴스를 반환한다.
    pub fn new(
        dsn: impl Into<String>,
        user: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            dsn: dsn.into(),
            user: user.into(),
            password: password.into(),
        }
    }
}

#[async_trait]
impl DbExecutor for OracleDbExecutor {
    /// sqlplus 프로세스를 실행해 Oracle DB에 임의의 SQL을 전달한다.
    ///
    /// # 매개변수
    /// - `sql`: 실행할 SQL 문자열.
    ///
    /// # 반환값
    /// sqlplus 종료 코드에 따라 성공 또는 오류를 반환한다.
    async fn execute_sql(&self, sql: &str) -> Result<()> {
        let mut command = Command::new("sqlplus");
        command.arg("-S");
        command.arg(format!("{}/{}@{}", self.user, self.password, self.dsn));
        command.stdin(std::process::Stdio::piped());
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        let mut child = command.spawn().context("sqlplus 실행 실패")?;

        if let Some(mut stdin) = child.stdin.take() {
            let script = format!("SET HEADING OFF\nSET FEEDBACK OFF\n{sql}\n/\nEXIT\n");
            stdin
                .write_all(script.as_bytes())
                .await
                .context("sqlplus stdin 전송 실패")?;
        }

        let status = child.wait().await.context("sqlplus 종료 대기 실패")?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(format!("sqlplus 종료 코드: {status}")))
        }
    }
}

/// OracleDbExecutor를 [`SharedExecutor`] 형태로 생성한다.
///
/// # 매개변수
/// - `dsn`: Oracle DSN 문자열.
/// - `user`: 사용자명.
/// - `password`: 비밀번호.
///
/// # 반환값
/// [`SharedExecutor`]로 감싼 Oracle 실행기를 반환한다.
pub fn new_oracle_db_executor(
    dsn: impl Into<String>,
    user: impl Into<String>,
    password: impl Into<String>,
) -> SharedExecutor {
    Arc::new(OracleDbExecutor::new(dsn, user, password))
}