use anyhow::Context;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

/// 실행 중 Step 간 값을 공유하기 위한 컨텍스트이다.
#[derive(Debug, Default)]
pub struct ExecutionContext {
    /// 문자열 기반 변수 저장소이다.
    vars: HashMap<String, String>,
}

impl ExecutionContext {
    /// 비어 있는 실행 컨텍스트를 생성한다.
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    /// 컨텍스트 변수 값을 설정한다.
    ///
    /// # 매개변수
    /// - `key`: 저장할 변수명.
    /// - `value`: 저장할 문자열 값.
    pub fn set_var(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.vars.insert(key.into(), value.into());
    }

    /// 변수 값을 조회한다.
    ///
    /// # 매개변수
    /// - `key`: 조회할 변수명.
    ///
    /// # 반환값
    /// 존재하면 문자열 슬라이스를 반환하고 없으면 `None`을 반환한다.
    pub fn get_var(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(|s| s.as_str())
    }

    /// 컨텍스트 또는 환경 변수에서 값을 조회한다.
    ///
    /// # 매개변수
    /// - `key`: 찾을 변수명.
    ///
    /// # 반환값
    /// 우선 컨텍스트에서 찾고 없으면 환경 변수에서 조회한 값을 반환한다.
    pub fn get_or_env(&self, key: &str) -> Option<String> {
        if let Some(value) = self.get_var(key) {
            return Some(value.to_string());
        }
        std::env::var(key).ok()
    }

    /// `${VAR}` 패턴을 실제 값으로 치환한다.
    ///
    /// # 매개변수
    /// - `template`: 치환할 원본 문자열.
    ///
    /// # 반환값
    /// 성공 시 치환 결과 문자열을 반환한다.
    pub fn expand_placeholders(&self, template: &str) -> anyhow::Result<String> {
        static PLACEHOLDER: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"\$\{([A-Z0-9_]+)\}").expect("정규식 컴파일 실패"));
        let result = PLACEHOLDER.replace_all(template, |caps: &regex::Captures| {
            let key = &caps[1];
            if let Some(val) = self.get_var(key) {
                return val.to_string();
            }
            if let Ok(env_val) = std::env::var(key) {
                return env_val;
            }
            format!("${{{key}}}")
        });
        let result = result.to_string();
        if PLACEHOLDER.is_match(&result) {
            anyhow::bail!("플레이스홀더 치환 실패: {result}");
        }
        Ok(result)
    }

    /// `template` 문자열을 치환하되 값이 없을 경우 명시적인 오류를 발생시킨다.
    pub fn expand_required(&self, template: &str, field: &str) -> anyhow::Result<String> {
        self.expand_placeholders(template)
            .with_context(|| format!("{field} 필드의 플레이스홀더를 치환할 수 없습니다."))
    }
}

/// ExecutionContext를 비동기 환경에서 공유하기 위한 타입 별칭이다.
pub type SharedExecutionContext = std::sync::Arc<tokio::sync::RwLock<ExecutionContext>>;
