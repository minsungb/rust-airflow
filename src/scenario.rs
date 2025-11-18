use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShellErrorPolicy {
    /// 비정상 종료 시 Step 전체를 실패로 처리한다.
    Fail,
    /// 비정상 종료여도 Step을 성공으로 간주한다.
    Ignore,
}

impl Default for ShellErrorPolicy {
    /// 기본 정책은 실패 시 Step을 중단한다.
    fn default() -> Self {
        ShellErrorPolicy::Fail
    }
}

/// StepKind는 배치 엔진이 수행할 개별 작업 유형을 표현한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepKind {
    /// SQL 문자열을 직접 실행한다.
    Sql { sql: String },
    /// SQL 파일을 읽어 실행한다.
    SqlFile {
        /// SQL 파일 경로.
        #[serde(rename = "sql_file")]
        path: PathBuf,
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
    pub depends_on: Vec<String>,
    /// 같은 레벨에서 병렬 실행 가능한지 여부.
    pub allow_parallel: bool,
    /// 실패 시 재시도 횟수.
    pub retry: u8,
    /// 실행 제한 시간(초 단위).
    pub timeout_sec: u64,
}

/// Scenario는 여러 Step으로 구성된 전체 배치 정의다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    /// 시나리오의 표시 이름.
    pub name: String,
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
