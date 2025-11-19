use crate::scenario::{
    ExtractVarFromFileConfig, LoopIterationFailure, LoopStepConfig, ShellConfig,
    SqlLoaderParConfig, Step, StepConfirmConfig, StepKind as ScenarioStepKind,
};
use eframe::egui;
use std::collections::HashSet;
use std::path::PathBuf;

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

/// 노드 간의 방향성 연결을 표현한다.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EditorConnection {
    /// 의존성을 제공하는 노드 ID.
    pub from_id: String,
    /// 의존성을 갖는 노드 ID.
    pub to_id: String,
}

/// Loop 하위 흐름을 표현하는 구성체이다.
#[derive(Debug, Clone)]
pub struct LoopEditorConfig {
    /// 반복 대상 glob 패턴.
    pub for_each_glob: String,
    /// 변수명.
    pub as_var: String,
    /// 실패 정책.
    pub on_iteration_failure: LoopIterationFailure,
    /// 하위 노드 목록.
    pub nodes: Vec<EditorStepNode>,
    /// 하위 연결 목록.
    pub connections: Vec<EditorConnection>,
    /// 선택된 하위 노드.
    pub selected_node_id: Option<String>,
}

impl LoopEditorConfig {
    /// 기본 Loop 구성을 생성한다.
    pub fn new() -> Self {
        Self {
            for_each_glob: String::new(),
            as_var: "ITEM".into(),
            on_iteration_failure: LoopIterationFailure::StopAll,
            nodes: Vec::new(),
            connections: Vec::new(),
            selected_node_id: None,
        }
    }

    /// 시나리오 Loop 설정을 에디터 구성으로 변환한다.
    pub fn from_scenario_config(config: &LoopStepConfig) -> Self {
        let mut nodes = Vec::new();
        let mut connections = Vec::new();
        for step in &config.steps {
            nodes.push(EditorStepNode::from_scenario_step(step));
            for dep in &step.depends_on {
                connections.push(EditorConnection {
                    from_id: dep.clone(),
                    to_id: step.id.clone(),
                });
            }
        }
        Self {
            for_each_glob: config.for_each_glob.clone(),
            as_var: config.as_var.clone(),
            on_iteration_failure: config.on_iteration_failure.clone(),
            nodes,
            connections,
            selected_node_id: None,
        }
    }

    /// 하위 노드용 고유 ID를 생성한다.
    pub fn generate_child_id(&self) -> String {
        let mut idx = 1;
        let ids: HashSet<&str> = self.nodes.iter().map(|n| n.id.as_str()).collect();
        loop {
            let candidate = format!("loop_step_{idx}");
            if !ids.contains(candidate.as_str()) {
                return candidate;
            }
            idx += 1;
        }
    }

    /// 하위 노드를 조회한다.
    pub fn node_mut(&mut self, id: &str) -> Option<&mut EditorStepNode> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    /// 하위 노드를 조회한다.
    pub fn node(&self, id: &str) -> Option<&EditorStepNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    /// 특정 노드의 의존성 목록을 반환한다.
    pub fn dependencies_of(&self, id: &str) -> Vec<String> {
        self.connections
            .iter()
            .filter(|conn| conn.to_id == id)
            .map(|conn| conn.from_id.clone())
            .collect()
    }

    /// 연결을 추가한다.
    pub fn add_connection(&mut self, from_id: &str, to_id: &str) {
        if from_id == to_id {
            return;
        }
        let conn = EditorConnection {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
        };
        if !self.connections.contains(&conn) {
            self.connections.push(conn);
        }
    }

    /// 연결을 제거한다.
    pub fn remove_connection(&mut self, from_id: &str, to_id: &str) {
        self.connections
            .retain(|conn| !(conn.from_id == from_id && conn.to_id == to_id));
    }

    /// 노드를 제거하고 연결을 정리한다.
    pub fn remove_node(&mut self, id: &str) {
        self.nodes.retain(|node| node.id != id);
        self.connections
            .retain(|conn| conn.from_id != id && conn.to_id != id);
        if self.selected_node_id.as_deref() == Some(id) {
            self.selected_node_id = None;
        }
    }
}

/// 시나리오 에디터 전체 상태를 저장한다.
#[derive(Debug, Clone)]
pub struct ScenarioEditorState {
    /// 노드 목록.
    pub nodes: Vec<EditorStepNode>,
    /// 연결 목록.
    pub connections: Vec<EditorConnection>,
    /// 선택된 노드 ID.
    pub selected_node_id: Option<String>,
    /// 현재 파일 경로.
    pub current_file: Option<PathBuf>,
    /// 캔버스 오프셋.
    pub canvas_offset: egui::Vec2,
    /// 캔버스 줌 비율.
    pub canvas_zoom: f32,
    /// 미완성 연결 시작 노드.
    pub pending_connection: Option<String>,
    /// 저장되지 않은 변경 여부.
    pub dirty: bool,
}

impl ScenarioEditorState {
    /// 빈 에디터 상태를 생성한다.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            connections: Vec::new(),
            selected_node_id: None,
            current_file: None,
            canvas_offset: egui::vec2(0.0, 0.0),
            canvas_zoom: 1.0,
            pending_connection: None,
            dirty: false,
        }
    }

    /// 고유한 Step ID를 생성한다.
    pub fn generate_id(&self, prefix: &str) -> String {
        let mut idx = 1;
        let ids: HashSet<&str> = self.nodes.iter().map(|n| n.id.as_str()).collect();
        loop {
            let candidate = format!("{prefix}_{idx}");
            if !ids.contains(candidate.as_str()) {
                return candidate;
            }
            idx += 1;
        }
    }

    /// 새 노드를 추가하고 선택한다.
    pub fn add_node(&mut self, kind: StepKind) -> String {
        let id = self.generate_id("step");
        let mut node = EditorStepNode::new(id.clone(), format!("새 Step {id}"), kind);
        node.position = egui::pos2(80.0, 80.0);
        self.nodes.push(node);
        self.select_node(Some(id.clone()));
        self.dirty = true;
        id
    }

    /// 지정된 노드를 제거한다.
    pub fn remove_node(&mut self, id: &str) {
        self.nodes.retain(|node| node.id != id);
        self.connections
            .retain(|conn| conn.from_id != id && conn.to_id != id);
        if self.selected_node_id.as_deref() == Some(id) {
            self.selected_node_id = None;
        }
        self.dirty = true;
    }

    /// 노드 선택 상태를 갱신한다.
    pub fn select_node(&mut self, id: Option<String>) {
        self.selected_node_id = id.clone();
        for node in &mut self.nodes {
            node.selected = Some(node.id.as_str()) == id.as_deref();
        }
    }

    /// ID로 노드를 조회한다.
    pub fn node_mut(&mut self, id: &str) -> Option<&mut EditorStepNode> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    /// ID로 노드를 조회한다.
    pub fn node(&self, id: &str) -> Option<&EditorStepNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    /// depends_on 목록을 반환한다.
    pub fn dependencies_of(&self, id: &str) -> Vec<String> {
        self.connections
            .iter()
            .filter(|conn| conn.to_id == id)
            .map(|conn| conn.from_id.clone())
            .collect()
    }

    /// 연결을 추가한다.
    pub fn add_connection(&mut self, from_id: &str, to_id: &str) {
        if from_id == to_id {
            return;
        }
        let conn = EditorConnection {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
        };
        if !self.connections.contains(&conn) {
            self.connections.push(conn);
            self.dirty = true;
        }
    }

    /// 연결을 제거한다.
    pub fn remove_connection(&mut self, from_id: &str, to_id: &str) {
        self.connections
            .retain(|conn| !(conn.from_id == from_id && conn.to_id == to_id));
        self.dirty = true;
    }
}

impl Default for ScenarioEditorState {
    /// 기본 상태를 반환한다.
    fn default() -> Self {
        Self::new()
    }
}

/// 에디터 작업 중 발생 가능한 오류를 표현한다.
#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    /// Step ID가 중복된 경우이다.
    #[error("중복된 Step ID가 존재합니다: {0}")]
    DuplicateStepId(String),
    /// 존재하지 않는 노드를 참조하는 연결이다.
    #[error("존재하지 않는 노드를 참조하는 연결입니다: {from_id} -> {to_id}")]
    MissingNode { from_id: String, to_id: String },
    /// 순환 의존성이 감지된 경우이다.
    #[error("순환 의존성이 감지되었습니다. 연결 구성을 확인하세요.")]
    CyclicDependency,
}

impl LoopEditorConfig {
    /// Loop 구성을 Scenario 구조로 변환한다.
    pub fn to_loop_step_config(&self) -> Result<LoopStepConfig, EditorError> {
        let mut steps = Vec::new();
        for node in &self.nodes {
            let deps = self.dependencies_of(&node.id);
            steps.push(node.to_scenario_step(deps)?);
        }
        Ok(LoopStepConfig {
            for_each_glob: self.for_each_glob.clone(),
            as_var: self.as_var.clone(),
            steps,
            on_iteration_failure: self.on_iteration_failure.clone(),
        })
    }
}
