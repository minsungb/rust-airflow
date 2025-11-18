use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// 시나리오에서 사용 가능한 DB 연결 정의이다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConnectionConfig {
    /// 연결 종류이다.
    pub kind: DbKind,
    /// 연결 문자열 또는 DSN이다.
    pub dsn: Option<String>,
    /// 사용자명이다.
    pub user: Option<String>,
    /// 비밀번호이다.
    pub password: Option<String>,
}

/// 지원하는 DB 종류를 표현한다.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DbKind {
    /// 더미 실행기.
    Dummy,
    /// PostgreSQL 연결.
    Postgres,
    /// Oracle sqlplus 기반 연결.
    Oracle,
}

/// sqlldr Step 구성을 표현한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlLoaderParConfig {
    /// control 파일 경로.
    pub control_file: PathBuf,
    /// 데이터 파일 경로.
    pub data_file: Option<PathBuf>,
    /// 로그 파일 경로.
    pub log_file: Option<PathBuf>,
    /// bad 파일 경로.
    pub bad_file: Option<PathBuf>,
    /// discard 파일 경로.
    pub discard_file: Option<PathBuf>,
    /// SQL*Loader 접속 문자열.
    pub conn: Option<String>,
}

/// Shell Step 실행 설정이다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    /// 실제 실행할 스크립트/명령 문자열.
    #[serde(alias = "command")]
    pub script: String,
    /// 사용할 셸 프로그램 경로.
    pub shell_program: Option<String>,
    /// 셸 프로그램 추가 인자 목록.
    #[serde(default)]
    pub shell_args: Vec<String>,
    /// 스크립트 실행 시 적용할 환경 변수.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// 실행 전 변경할 작업 디렉터리.
    pub working_dir: Option<PathBuf>,
    /// 명령을 실행할 사용자 계정.
    pub run_as: Option<String>,
    /// 비정상 종료 시 처리 정책.
    #[serde(default)]
    pub error_policy: ShellErrorPolicy,
}

/// Shell Step 실패 처리 정책이다.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum ShellErrorPolicy {
    /// 비정상 종료 시 Step 전체를 실패로 처리한다.
    Fail,
    /// 비정상 종료여도 Step을 성공으로 간주한다.
    Ignore,
    /// 지정 횟수만큼 재시도한다.
    Retry { max_retries: u32, delay_secs: u64 },
}

impl Default for ShellErrorPolicy {
    /// 기본 정책은 실패 시 Step을 중단한다.
    fn default() -> Self {
        ShellErrorPolicy::Fail
    }
}

impl<'de> Deserialize<'de> for ShellErrorPolicy {
    /// 문자열 또는 구조체 형태의 설정을 모두 지원하도록 역직렬화한다.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper {
            Simple(String),
            Detailed {
                #[serde(rename = "type")]
                policy_type: String,
                max_retries: Option<u32>,
                delay_secs: Option<u64>,
            },
        }

        match Helper::deserialize(deserializer)? {
            Helper::Simple(value) => match value.as_str() {
                "fail" => Ok(ShellErrorPolicy::Fail),
                "ignore" => Ok(ShellErrorPolicy::Ignore),
                "retry" => Ok(ShellErrorPolicy::Retry {
                    max_retries: 3,
                    delay_secs: 5,
                }),
                other => Err(de::Error::custom(format!(
                    "알 수 없는 shell error policy: {other}"
                ))),
            },
            Helper::Detailed {
                policy_type,
                max_retries,
                delay_secs,
            } => match policy_type.as_str() {
                "fail" => Ok(ShellErrorPolicy::Fail),
                "ignore" => Ok(ShellErrorPolicy::Ignore),
                "retry" => Ok(ShellErrorPolicy::Retry {
                    max_retries: max_retries.unwrap_or(3),
                    delay_secs: delay_secs.unwrap_or(5),
                }),
                other => Err(de::Error::custom(format!(
                    "알 수 없는 shell error policy: {other}"
                ))),
            },
        }
    }
}

/// StepKind는 배치 엔진이 수행할 개별 작업 유형을 표현한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractVarFromFileConfig {
    /// 읽을 파일 경로.
    pub file_path: String,
    /// 1 기반 라인 번호.
    pub line: usize,
    /// 매칭에 사용할 정규식 패턴.
    pub pattern: String,
    /// 사용할 캡처 그룹 번호.
    pub group: usize,
    /// 저장할 변수명.
    pub var_name: String,
}

/// Loop Step 구성을 표현한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopStepConfig {
    /// 반복에 사용할 glob 패턴.
    pub for_each_glob: String,
    /// 현재 항목을 저장할 변수명.
    pub as_var: String,
    /// 반복 내에서 실행할 Step 목록.
    pub steps: Vec<Step>,
}

/// StepKind는 배치 엔진이 수행할 개별 작업 유형을 표현한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepKind {
    /// SQL 문자열을 직접 실행한다.
    Sql {
        /// 실행할 SQL 문자열.
        sql: String,
        /// 사용할 DB 타겟.
        #[serde(default)]
        target_db: Option<String>,
    },
    /// SQL 파일을 읽어 실행한다.
    SqlFile {
        /// SQL 파일 경로.
        #[serde(rename = "sql_file")]
        path: PathBuf,
        /// 사용할 DB 타겟.
        #[serde(default)]
        target_db: Option<String>,
    },
    /// sqlldr par 파일을 실행한다.
    SqlLoaderPar {
        /// sqlldr 실행 구성.
        #[serde(rename = "sqlldr")]
        config: SqlLoaderParConfig,
    },
    /// 쉘 명령을 실행한다.
    Shell {
        /// 쉘 실행 구성.
        #[serde(rename = "shell")]
        config: ShellConfig,
    },
    /// 파일에서 변수를 추출한다.
    ExtractVarFromFile {
        /// 추출 설정.
        #[serde(rename = "extract")]
        config: ExtractVarFromFileConfig,
    },
    /// 지정된 glob 목록에 대해 Step 블록을 반복 실행한다.
    Loop {
        /// 반복 실행 설정.
        #[serde(rename = "loop")]
        config: LoopStepConfig,
    },
}

/// Step은 Scenario 내 최소 실행 단위를 표현한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// 고유 Step ID.
    pub id: String,
    /// 사용자 친화적인 Step 이름.
    pub name: String,
    /// Step에서 실행할 Kind 정보.
    #[serde(flatten)]
    pub kind: StepKind,
    /// 선행 Step ID 목록.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// 같은 레벨에서 병렬 실행 가능한지 여부.
    #[serde(default)]
    pub allow_parallel: bool,
    /// 실패 시 재시도 횟수.
    #[serde(default = "default_retry")]
    pub retry: u8,
    /// 실행 제한 시간(초 단위).
    #[serde(default = "default_timeout")]
    pub timeout_sec: u64,
    /// Step 실행 컨펌 설정.
    #[serde(default)]
    pub confirm: Option<StepConfirmConfig>,
}

/// Scenario는 여러 Step으로 구성된 전체 배치 정의다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    /// 시나리오의 표시 이름.
    pub name: String,
    /// DB 실행기 정의 맵.
    #[serde(default)]
    pub db: HashMap<String, DbConnectionConfig>,
    /// Step 목록.
    pub steps: Vec<Step>,
}

impl Scenario {
    /// Step ID를 키로 하는 조회 맵을 생성한다.
    pub fn as_map(&self) -> HashMap<String, Step> {
        self.steps
            .iter()
            .cloned()
            .map(|s| (s.id.clone(), s))
            .collect()
    }

    /// 전체 Step 수를 반환한다.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Step 수가 비었는지 여부를 확인한다.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

/// Step 실행 컨펌 구성을 표현한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepConfirmConfig {
    /// 실행 전 확인 여부.
    #[serde(default)]
    pub before: bool,
    /// 실행 후 확인 여부.
    #[serde(default)]
    pub after: bool,
    /// 실행 전 메시지.
    pub message_before: Option<String>,
    /// 실행 후 메시지.
    pub message_after: Option<String>,
    /// UI 없는 환경에서 사용할 기본 응답.
    #[serde(default)]
    pub default_answer: ConfirmDefault,
}

/// 컨펌 기본 응답 값을 표현한다.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmDefault {
    /// 예(진행)를 기본값으로 사용한다.
    Yes,
    /// 아니오(거부)를 기본값으로 사용한다.
    No,
}

impl Default for ConfirmDefault {
    /// 기본값은 예로 설정한다.
    fn default() -> Self {
        ConfirmDefault::Yes
    }
}

fn default_retry() -> u8 {
    0
}

fn default_timeout() -> u64 {
    60
}

/// YAML 파일을 읽어 Scenario로 역직렬화한다.
pub fn load_scenario_from_file(path: &Path) -> anyhow::Result<Scenario> {
    let mut file = File::open(path)?;
    load_scenario_from_reader(&mut file)
}

/// Reader에서 YAML을 읽어 Scenario 구조체로 파싱한다.
pub fn load_scenario_from_reader<R: Read>(reader: &mut R) -> anyhow::Result<Scenario> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    let scenario: Scenario = serde_yaml::from_str(&buf)?;
    Ok(scenario)
}
