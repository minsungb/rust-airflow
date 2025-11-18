use crate::engine::{EngineEvent, StepRuntimeState, StepStatus, run_scenario};
use crate::executor::{DummyExecutor, SharedExecutor};
use crate::scenario::{Scenario, load_scenario_from_file};
use crate::theme::Theme;
use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio_util::sync::CancellationToken;

/// 메모리에 적재할 수 있는 최대 로그 라인 수를 정의한다.
pub(crate) const MAX_LOG_LINES: usize = 500;

/// egui 애플리케이션의 전체 상태를 보관한다.
pub struct BatchOrchestratorApp {
    /// UI 테마 정보.
    pub(crate) theme: Theme,
    /// 현재 로드된 시나리오.
    pub(crate) scenario: Option<Scenario>,
    /// 선택된 시나리오 경로.
    pub(crate) scenario_path: Option<PathBuf>,
    /// 선택된 Step ID.
    pub(crate) selected_step: Option<String>,
    /// Step별 상태 맵.
    pub(crate) step_states: HashMap<String, StepRuntimeState>,
    /// Step별 로그 버퍼.
    pub(crate) step_logs: HashMap<String, Vec<String>>,
    /// Tokio 런타임.
    runtime: Runtime,
    /// DB 실행기.
    pub(crate) executor: SharedExecutor,
    /// 엔진 이벤트 수신 채널.
    pub(crate) events_rx: Option<UnboundedReceiver<EngineEvent>>,
    /// 시나리오 취소 토큰.
    pub(crate) cancel_token: Option<CancellationToken>,
    /// 실행 중 여부.
    pub(crate) scenario_running: bool,
    /// 마지막 오류 메시지.
    pub(crate) last_error: Option<String>,
}

impl BatchOrchestratorApp {
    /// egui Context를 받아 초기 상태를 구성한다.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme = Theme::default();
        theme.apply(&cc.egui_ctx);
        let runtime = Runtime::new().expect("Tokio 런타임 생성 실패");
        Self {
            theme,
            scenario: None,
            scenario_path: None,
            selected_step: None,
            step_states: HashMap::new(),
            step_logs: HashMap::new(),
            runtime,
            executor: Arc::new(DummyExecutor::default()),
            events_rx: None,
            cancel_token: None,
            scenario_running: false,
            last_error: None,
        }
    }

    /// 엔진 이벤트를 모두 소비하여 UI 상태를 동기화한다.
    pub(super) fn drain_events(&mut self) {
        if let Some(mut rx) = self.events_rx.take() {
            while let Ok(event) = rx.try_recv() {
                match event {
                    EngineEvent::StepStarted { step_id } => {
                        self.mark_step_running(&step_id);
                    }
                    EngineEvent::StepLog { step_id, line } => {
                        self.push_log(&step_id, line);
                    }
                    EngineEvent::StepFinished { step_id, success } => {
                        self.mark_step_finished(&step_id, success);
                    }
                    EngineEvent::ScenarioFinished => {
                        self.scenario_running = false;
                        self.cancel_token = None;
                    }
                }
            }
            self.events_rx = Some(rx);
        }
    }

    /// Step 상태를 Running으로 갱신한다.
    fn mark_step_running(&mut self, step_id: &str) {
        let state = self.step_states.entry(step_id.to_string()).or_default();
        state.status = StepStatus::Running;
        state.started_at = Some(std::time::Instant::now());
    }

    /// Step이 종료되었음을 기록한다.
    fn mark_step_finished(&mut self, step_id: &str, success: bool) {
        let state = self.step_states.entry(step_id.to_string()).or_default();
        state.finished_at = Some(std::time::Instant::now());
        if success {
            state.status = StepStatus::Success;
        } else if !matches!(state.status, StepStatus::Failed(_)) {
            let fallback = self
                .step_logs
                .get(step_id)
                .and_then(|logs| logs.last())
                .cloned()
                .unwrap_or_else(|| "실패".into());
            state.status = StepStatus::Failed(fallback);
        }
    }

    /// Step별 로그를 버퍼에 적재한다.
    fn push_log(&mut self, step_id: &str, line: String) {
        let entry = self.step_logs.entry(step_id.to_string()).or_default();
        entry.push(line.clone());
        if entry.len() > MAX_LOG_LINES {
            let overflow = entry.len() - MAX_LOG_LINES;
            entry.drain(0..overflow);
        }
        let state = self.step_states.entry(step_id.to_string()).or_default();
        state.logs.push(line);
        if state.logs.len() > MAX_LOG_LINES {
            let overflow = state.logs.len() - MAX_LOG_LINES;
            state.logs.drain(0..overflow);
        }
    }

    /// 파일 다이얼로그로부터 시나리오를 로드한다.
    pub(super) fn load_scenario_from_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("YAML", &["yaml", "yml"])
            .pick_file()
        {
            self.apply_scenario_path(path.into());
        }
    }

    /// 주어진 경로의 YAML을 파싱한다.
    pub(super) fn apply_scenario_path(&mut self, path: PathBuf) {
        match load_scenario_from_file(&path) {
            Ok(scenario) => {
                self.step_states.clear();
                self.step_logs.clear();
                for step in &scenario.steps {
                    self.step_states
                        .insert(step.id.clone(), StepRuntimeState::new());
                    self.step_logs.insert(step.id.clone(), Vec::new());
                }
                self.selected_step = scenario.steps.first().map(|s| s.id.clone());
                self.scenario = Some(scenario);
                self.scenario_path = Some(path);
                self.last_error = None;
            }
            Err(err) => {
                self.last_error = Some(err.to_string());
            }
        }
    }

    /// 시나리오 실행을 시작한다.
    pub(super) fn start_scenario(&mut self) {
        if self.scenario_running {
            return;
        }
        let scenario = match self.scenario.clone() {
            Some(s) => s,
            None => {
                self.last_error = Some("시나리오가 로드되지 않았습니다.".into());
                return;
            }
        };
        self.step_logs.clear();
        self.step_states.clear();
        for step in &scenario.steps {
            self.step_states
                .insert(step.id.clone(), StepRuntimeState::new());
            self.step_logs.insert(step.id.clone(), Vec::new());
        }
        let (tx, rx) = mpsc::unbounded_channel();
        let token = CancellationToken::new();
        self.runtime.spawn(run_scenario(
            scenario.clone(),
            self.executor.clone(),
            tx,
            token.clone(),
        ));
        self.events_rx = Some(rx);
        self.cancel_token = Some(token);
        self.scenario_running = true;
        self.last_error = None;
    }

    /// 현재 실행 중인 시나리오를 중단한다.
    pub(super) fn stop_scenario(&mut self) {
        if let Some(token) = &self.cancel_token {
            token.cancel();
        }
        self.scenario_running = false;
    }

    /// 선택된 Step의 로그 배열을 반환한다.
    pub(super) fn selected_logs(&self) -> Vec<String> {
        if let Some(step_id) = &self.selected_step {
            if let Some(logs) = self.step_logs.get(step_id) {
                return logs.clone();
            }
        }
        Vec::new()
    }

    /// 전체 진행률을 계산한다.
    pub(super) fn progress_ratio(&self) -> f32 {
        if let Some(scenario) = &self.scenario {
            if scenario.steps.is_empty() {
                return 0.0;
            }
            let completed = self
                .step_states
                .values()
                .filter(|state| matches!(state.status, StepStatus::Success | StepStatus::Failed(_)))
                .count();
            completed as f32 / scenario.steps.len() as f32
        } else {
            0.0
        }
    }
}
