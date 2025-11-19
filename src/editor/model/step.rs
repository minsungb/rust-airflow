use crate::scenario::{
    ExtractVarFromFileConfig, LoopStepConfig, ShellConfig, SqlLoaderParConfig, Step,
    StepConfirmConfig, StepKind as ScenarioStepKind,
};
use eframe::egui;
use std::path::PathBuf;

use super::error::EditorError;
use super::loop_config::LoopEditorConfig;

/// 에디터에서 지원하는 Step 유형을 정의한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    /// SQL 문자열을 직접 작성하는 Step이다.
    Sql,
    /// SQL 파일을 참조하는 Step이다.
    SqlFile,
    /// SQL*Loader parfile 기반 Step이다.
    SqlLoaderPar,
    /// Shell 명령 Step이다.
    Shell,
    /// 파일에서 값을 추출하는 Step이다.
    Extract,
    /// Loop 컨테이너 Step이다.
    Loop,
}

/// Step별 상세 구성을 저장한다.
#[derive(Debug, Clone)]
pub enum EditorStepConfig {
    /// SQL Step 구성이다.
    Sql {
        /// 실행할 SQL 텍스트.
        sql: String,
        /// 대상 DB 명칭.
        target_db: Option<String>,
    },
    /// SQL 파일 Step 구성이다.
    SqlFile {
        /// SQL 파일 경로.
        path: PathBuf,
        /// 대상 DB 명칭.
        target_db: Option<String>,
    },
    /// SQL*Loader Step 구성이다.
    SqlLoaderPar {
        /// SQL*Loader 실행 구체 구성.
        config: SqlLoaderParConfig,
    },
    /// Shell Step 구성이다.
    Shell {
        /// Shell 실행 구성.
        config: ShellConfig,
    },
    /// Extract Step 구성이다.
    Extract {
        /// 파일 추출 설정.
        config: ExtractVarFromFileConfig,
    },
    /// Loop Step 구성이다.
    Loop {
        /// Loop 실행 설정.
        config: LoopEditorConfig,
    },
}

impl EditorStepConfig {
    /// 지정한 StepKind에 맞는 기본 구성을 반환한다.
    pub fn default_for(kind: StepKind) -> Self {
        match kind {
            StepKind::Sql => EditorStepConfig::Sql {
                sql: String::new(),
                target_db: None,
            },
            StepKind::SqlFile => EditorStepConfig::SqlFile {
                path: PathBuf::new(),
                target_db: None,
            },
            StepKind::SqlLoaderPar => EditorStepConfig::SqlLoaderPar {
                config: SqlLoaderParConfig {
                    control_file: PathBuf::new(),
                    data_file: None,
                    log_file: None,
                    bad_file: None,
                    discard_file: None,
                    conn: None,
                },
            },
            StepKind::Shell => EditorStepConfig::Shell {
                config: ShellConfig {
                    script: String::new(),
                    shell_program: None,
                    shell_args: Vec::new(),
                    env: Default::default(),
                    working_dir: None,
                    run_as: None,
                    error_policy: Default::default(),
                },
            },
            StepKind::Extract => EditorStepConfig::Extract {
                config: ExtractVarFromFileConfig {
                    file_path: String::new(),
                    line: 1,
                    pattern: String::new(),
                    group: 1,
                    var_name: String::new(),
                },
            },
            StepKind::Loop => EditorStepConfig::Loop {
                config: LoopEditorConfig::new(),
            },
        }
    }

    /// Scenario StepKind를 에디터 구성으로 변환한다.
    pub fn from_scenario_kind(kind: &ScenarioStepKind) -> (StepKind, Self) {
        match kind {
            ScenarioStepKind::Sql { sql, target_db } => (
                StepKind::Sql,
                EditorStepConfig::Sql {
                    sql: sql.clone(),
                    target_db: target_db.clone(),
                },
            ),
            ScenarioStepKind::SqlFile { path, target_db } => (
                StepKind::SqlFile,
                EditorStepConfig::SqlFile {
                    path: path.clone(),
                    target_db: target_db.clone(),
                },
            ),
            ScenarioStepKind::SqlLoaderPar { config } => (
                StepKind::SqlLoaderPar,
                EditorStepConfig::SqlLoaderPar {
                    config: config.clone(),
                },
            ),
            ScenarioStepKind::Shell { config } => (
                StepKind::Shell,
                EditorStepConfig::Shell {
                    config: config.clone(),
                },
            ),
            ScenarioStepKind::Extract { config } => (
                StepKind::Extract,
                EditorStepConfig::Extract {
                    config: config.clone(),
                },
            ),
            ScenarioStepKind::Loop { config } => (
                StepKind::Loop,
                EditorStepConfig::Loop {
                    config: LoopEditorConfig::from_scenario_config(config),
                },
            ),
        }
    }
}

/// 플로우 캔버스에 표시될 Step 노드이다.
#[derive(Debug, Clone)]
pub struct EditorStepNode {
    /// Step 고유 ID.
    pub id: String,
    /// 사용자 친화적인 이름.
    pub name: String,
    /// Step 유형.
    pub kind: StepKind,
    /// 노드 배치 좌표.
    pub position: egui::Pos2,
    /// 노드 크기.
    pub size: egui::Vec2,
    /// 선택 여부.
    pub selected: bool,
    /// Step 상세 구성.
    pub config: EditorStepConfig,
    /// 병렬 실행 허용 여부.
    pub allow_parallel: bool,
    /// 재시도 횟수.
    pub retry: u8,
    /// 타임아웃(초).
    pub timeout_sec: u64,
    /// 컨펌 설정.
    pub confirm: Option<StepConfirmConfig>,
}

impl EditorStepNode {
    /// 새로운 노드를 생성한다.
    pub fn new(id: String, name: String, kind: StepKind) -> Self {
        Self {
            config: EditorStepConfig::default_for(kind),
            id,
            name,
            kind,
            position: egui::pos2(40.0, 40.0),
            size: egui::vec2(220.0, 110.0),
            selected: false,
            allow_parallel: false,
            retry: 0,
            timeout_sec: 60,
            confirm: None,
        }
    }

    /// Scenario Step으로 변환한다.
    pub fn to_scenario_step(&self, depends_on: Vec<String>) -> Result<Step, EditorError> {
        let kind = match &self.config {
            EditorStepConfig::Sql { sql, target_db } => ScenarioStepKind::Sql {
                sql: sql.clone(),
                target_db: target_db.clone(),
            },
            EditorStepConfig::SqlFile { path, target_db } => ScenarioStepKind::SqlFile {
                path: path.clone(),
                target_db: target_db.clone(),
            },
            EditorStepConfig::SqlLoaderPar { config } => ScenarioStepKind::SqlLoaderPar {
                config: config.clone(),
            },
            EditorStepConfig::Shell { config } => ScenarioStepKind::Shell {
                config: config.clone(),
            },
            EditorStepConfig::Extract { config } => ScenarioStepKind::Extract {
                config: config.clone(),
            },
            EditorStepConfig::Loop { config } => ScenarioStepKind::Loop {
                config: config.to_loop_step_config()?,
            },
        };
        Ok(Step {
            id: self.id.clone(),
            name: self.name.clone(),
            kind,
            depends_on,
            allow_parallel: self.allow_parallel,
            retry: self.retry,
            timeout_sec: self.timeout_sec,
            confirm: self.confirm.clone(),
        })
    }

    /// Scenario Step을 기반으로 노드를 생성한다.
    pub fn from_scenario_step(step: &Step) -> Self {
        let (kind, config) = EditorStepConfig::from_scenario_kind(&step.kind);
        Self {
            id: step.id.clone(),
            name: step.name.clone(),
            kind,
            config,
            position: egui::pos2(40.0, 40.0),
            size: egui::vec2(220.0, 110.0),
            selected: false,
            allow_parallel: step.allow_parallel,
            retry: step.retry,
            timeout_sec: step.timeout_sec,
            confirm: step.confirm.clone(),
        }
    }
}
