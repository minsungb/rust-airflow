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

const MAX_LOG_LINES: usize = 500;

/// egui 애플리케이션의 전체 상태를 보관한다.
pub struct BatchOrchestratorApp {
    /// UI 테마 정보.
    theme: Theme,
    /// 현재 로드된 시나리오.
    scenario: Option<Scenario>,
    /// 선택된 시나리오 경로.
    scenario_path: Option<PathBuf>,
    /// 선택된 Step ID.
    selected_step: Option<String>,
    /// Step별 상태 맵.
    step_states: HashMap<String, StepRuntimeState>,
    /// Step별 로그 버퍼.
    step_logs: HashMap<String, Vec<String>>,
    /// Tokio 런타임.
    runtime: Runtime,
    /// DB 실행기.
    executor: SharedExecutor,
    /// 엔진 이벤트 수신 채널.
    events_rx: Option<UnboundedReceiver<EngineEvent>>,
    /// 시나리오 취소 토큰.
    cancel_token: Option<CancellationToken>,
    /// 실행 중 여부.
    scenario_running: bool,
    /// 마지막 오류 메시지.
    last_error: Option<String>,
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
    fn drain_events(&mut self) {
        // events_rx를 일단 self에서 빼내서 소유권을 가져온다.
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

            // 다 처리한 뒤에 다시 self 안에 되돌려 놓는다.
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
    fn load_scenario_from_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("YAML", &["yaml", "yml"])
            .pick_file()
        {
            self.apply_scenario_path(path.into());
        }
    }

    /// 주어진 경로의 YAML을 파싱한다.
    fn apply_scenario_path(&mut self, path: PathBuf) {
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
    fn start_scenario(&mut self) {
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
    fn stop_scenario(&mut self) {
        if let Some(token) = &self.cancel_token {
            token.cancel();
        }
        self.scenario_running = false;
    }

    /// 선택된 Step의 로그 배열을 반환한다.
    fn selected_logs(&self) -> Vec<String> {
        if let Some(step_id) = &self.selected_step {
            if let Some(logs) = self.step_logs.get(step_id) {
                return logs.clone();
            }
        }
        Vec::new()
    }

    /// 전체 진행률을 계산한다.
    fn progress_ratio(&self) -> f32 {
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

    /// 좌측 Step 리스트 패널을 그린다.
    fn render_step_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Steps");
        if let Some(scenario) = &self.scenario {
            for step in &scenario.steps {
                let state = self
                    .step_states
                    .get(&step.id)
                    .cloned()
                    .unwrap_or_else(StepRuntimeState::new);
                let color = self.theme.status_color(&state.status);
                let label = format!("{} · {}", step.id, step.name);
                let selectable = egui::SelectableLabel::new(
                    self.selected_step.as_deref() == Some(step.id.as_str()),
                    label,
                );
                let response = ui.add(selectable);
                let rect = response.rect;
                let painter = ui.painter();
                let indicator =
                    egui::Rect::from_min_max(rect.min, egui::pos2(rect.min.x + 4.0, rect.max.y));
                painter.rect_filled(indicator, 2.0, color);
                if response.clicked() {
                    self.selected_step = Some(step.id.clone());
                }
            }
        } else {
            ui.label("시나리오를 먼저 불러오세요.");
        }
    }

    /// Step 상세 정보를 표시한다.
    fn render_step_detail(&self, ui: &mut egui::Ui) {
        ui.heading("Step 정보");
        if let Some(step_id) = &self.selected_step {
            if let Some(scenario) = &self.scenario {
                if let Some(step) = scenario.steps.iter().find(|s| &s.id == step_id) {
                    let state = self
                        .step_states
                        .get(step_id)
                        .cloned()
                        .unwrap_or_else(StepRuntimeState::new);
                    ui.label(format!("ID: {}", step.id));
                    ui.label(format!("이름: {}", step.name));
                    ui.label(format!("병렬 허용: {}", step.allow_parallel));
                    ui.label(format!("재시도 횟수: {}", step.retry));
                    ui.label(format!("타임아웃: {}초", step.timeout_sec));
                    ui.label(format!("의존성: {}", step.depends_on.join(", ")));
                    ui.label(format!("상태: {:?}", state.status));
                }
            }
        } else {
            ui.label("선택된 Step이 없습니다.");
        }
    }

    /// 로그 영역을 렌더링한다.
    fn render_log_panel(&self, ui: &mut egui::Ui) {
        ui.heading("로그");
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in self.selected_logs() {
                    ui.monospace(line);
                }
            });
    }

    /// 상단 툴바를 그린다.
    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        if ui.button("시나리오 열기").clicked() {
            self.load_scenario_from_dialog();
        }
        ui.add_enabled_ui(self.scenario.is_some() && !self.scenario_running, |ui| {
            if ui.button("실행").clicked() {
                self.start_scenario();
            }
        });
        ui.add_enabled_ui(self.scenario_running, |ui| {
            if ui.button("정지").clicked() {
                self.stop_scenario();
            }
        });
        if let Some(path) = &self.scenario_path {
            ui.label(format!("로드됨: {}", path.display()));
        }
        if let Some(err) = &self.last_error {
            ui.colored_label(egui::Color32::RED, err);
        }
    }
}

impl eframe::App for BatchOrchestratorApp {
    /// egui 메인 루프에서 호출되어 UI를 갱신한다.
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.drain_events();
        self.theme.apply(ctx);
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            self.render_toolbar(ui);
        });
        egui::SidePanel::left("steps")
            .resizable(false)
            .default_width(280.0)
            .show(ctx, |ui| {
                self.render_step_panel(ui);
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    self.render_step_detail(ui);
                });
                ui.add_space(8.0);
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    self.render_log_panel(ui);
                });
            });
        });
        egui::TopBottomPanel::bottom("progress").show(ctx, |ui| {
            let ratio = self.progress_ratio();
            ui.add(egui::ProgressBar::new(ratio).text(format!("진행률: {:.0}%", ratio * 100.0)));
        });
    }
}
