use crate::scenario::DbKind;

/// Scenario Builder에서 편집 가능한 DB 연결 정보를 저장한다.
#[derive(Debug, Clone)]
pub struct DbConnectionEditor {
    /// 시나리오 YAML의 db 맵에서 사용할 키 이름이다.
    pub key: String,
    /// 접속 대상 DB 종류이다. (oracle/postgres)
    pub kind: DbKind,
    /// 연결 문자열 또는 DSN 값이다.
    pub dsn: String,
    /// 접속 사용자명이다.
    pub user: String,
    /// 접속 비밀번호이다.
    pub password: String,
}

impl DbConnectionEditor {
    /// 지정된 키와 종류를 기반으로 빈 DB 연결 편집 항목을 생성한다.
    pub fn new(key: String, kind: DbKind) -> Self {
        Self {
            key,
            kind,
            dsn: String::new(),
            user: String::new(),
            password: String::new(),
        }
    }
}
