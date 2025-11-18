use crate::engine::{EngineEvent, StepRuntimeState, StepStatus, run_scenario};
use crate::executor::{DummyExecutor, SharedExecutor};
use crate::scenario::{Scenario, load_scenario_from_file};
use crate::theme::{Theme, blend_color};
use eframe::egui::{self, RichText, Widget};
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
        solid_section_header(ui, &self.theme, "🧭", "작업 단계");
        ui.add_space(12.0);
        ui.spacing_mut().item_spacing.y = 12.0;
        let palette = *self.theme.palette();
        let decorations = *self.theme.decorations();
        if let Some(scenario) = &self.scenario {
            for step in &scenario.steps {
                let state = self
                    .step_states
                    .get(&step.id)
                    .cloned()
                    .unwrap_or_else(StepRuntimeState::new);
                let status_color = self.theme.status_color(&state.status);
                let (status_icon, status_text) = status_indicator(&state.status);
                let is_selected = self.selected_step.as_deref() == Some(step.id.as_str());
                let card_height = 74.0;
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), card_height),
                    egui::Sense::click(),
                );
                if ui.is_rect_visible(rect) {
                    let fill = if is_selected {
                        palette.bg_panel
                    } else {
                        palette.bg_sidebar
                    };
                    let stroke_color = if is_selected {
                        status_color
                    } else {
                        palette.border_soft
                    };
                    ui.painter().rect(
                        rect,
                        egui::Rounding::same(decorations.card_rounding),
                        fill,
                        egui::Stroke::new(1.5, stroke_color),
                    );
                    let indicator = egui::Rect::from_min_max(
                        rect.min,
                        egui::pos2(rect.min.x + 5.0, rect.max.y),
                    );
                    ui.painter().rect_filled(
                        indicator,
                        egui::Rounding::same(decorations.card_rounding),
                        status_color,
                    );
                    let content_rect = rect.shrink2(egui::vec2(
                        decorations.card_inner_margin.left,
                        decorations.card_inner_margin.top,
                    ));
                    let mut content_ui = ui.child_ui(
                        content_rect,
                        egui::Layout::left_to_right(egui::Align::Center),
                    );
                    content_ui.spacing_mut().item_spacing.x = 14.0;
                    content_ui.label(RichText::new(status_icon).size(26.0).color(status_color));
                    content_ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&step.name)
                                .size(17.0)
                                .color(palette.fg_text_primary)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("ID: {}", step.id))
                                .color(palette.fg_text_secondary),
                        );
                    });
                    content_ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label(
                                RichText::new(status_text)
                                    .size(15.0)
                                    .color(status_color)
                                    .strong(),
                            );
                        },
                    );
                }
                if response.clicked() {
                    self.selected_step = Some(step.id.clone());
                }
            }
        } else {
            let info = RichText::new("시나리오를 먼저 불러오세요.")
                .color(palette.fg_text_secondary)
                .italics();
            ui.label(info);
        }
    }

    /// Step 상세 정보를 표시한다.
    fn render_step_detail(&self, ui: &mut egui::Ui) {
        solid_section_header(ui, &self.theme, "🧩", "Step 정보");
        ui.add_space(10.0);
        let palette = *self.theme.palette();
        if let Some(step_id) = &self.selected_step {
            if let Some(scenario) = &self.scenario {
                if let Some(step) = scenario.steps.iter().find(|s| &s.id == step_id) {
                    let state = self
                        .step_states
                        .get(step_id)
                        .cloned()
                        .unwrap_or_else(StepRuntimeState::new);
                    let status_color = self.theme.status_color(&state.status);
                    let (_, status_text) = status_indicator(&state.status);
                    ui.label(
                        RichText::new(step.name.clone())
                            .size(20.0)
                            .color(palette.fg_text_primary)
                            .strong(),
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("상태 · {}", status_text))
                                .color(status_color)
                                .strong(),
                        );
                    });
                    ui.add_space(10.0);
                    egui::Grid::new("step_detail_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("ID");
                            ui.label(format!(": {}", step.id));
                            ui.end_row();
                            ui.label("병렬 허용");
                            ui.label(format!(": {}", step.allow_parallel));
                            ui.end_row();
                            ui.label("재시도");
                            ui.label(format!(": {}회", step.retry));
                            ui.end_row();
                            ui.label("타임아웃");
                            ui.label(format!(": {}초", step.timeout_sec));
                            ui.end_row();
                            ui.label("의존성");
                            let deps = if step.depends_on.is_empty() {
                                "없음".to_string()
                            } else {
                                step.depends_on.join(", ")
                            };
                            ui.label(format!(": {}", deps));
                            ui.end_row();
                        });
                }
            }
        } else {
            ui.label(RichText::new("선택된 Step이 없습니다.").color(palette.fg_text_secondary));
        }
    }

    /// 로그 영역을 렌더링한다.
    fn render_log_panel(&self, ui: &mut egui::Ui) {
        solid_section_header(ui, &self.theme, "📝", "로그");
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                let text_color = self.theme.palette().fg_text_secondary;
                for line in self.selected_logs() {
                    ui.label(RichText::new(line).color(text_color));
                }
            });
    }

    /// 상단 툴바를 그린다.
    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        let decorations = *self.theme.decorations();
        let palette = *self.theme.palette();
        ui.set_min_height(220.0);
        ui.vertical(|ui| {
            ui.label(
                RichText::new("✨ Rust Batch Orchestrator")
                    .size(22.0)
                    .color(palette.fg_text_primary)
                    .strong(),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new("Rust 기반 배치 시나리오를 안전하게 실행하세요.")
                    .color(palette.fg_text_secondary),
            );
            ui.add_space(10.0);
            if let Some(path) = &self.scenario_path {
                        ui.label(
                            RichText::new(format!("로드됨 · {}", path.display()))
                                .color(palette.fg_text_secondary),
                        );
                    } else {
                        ui.label(
                            RichText::new("시나리오 파일을 선택해 시작하세요.")
                                .color(palette.fg_text_secondary),
                        );
                    }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = decorations.button_gap;
                if let Some(err) = &self.last_error {
                    ui.label(RichText::new(err).color(palette.accent_error).strong());
                }
                if ui
                    .add(PrimaryButton::new(&self.theme, "시나리오 열기").icon("📂"))
                    .clicked()
                {
                    self.load_scenario_from_dialog();
                }
                ui.add_enabled_ui(self.scenario.is_some() && !self.scenario_running, |ui| {
                    if ui
                        .add(PrimaryButton::new(&self.theme, "실행").icon("▶"))
                        .clicked()
                    {
                        self.start_scenario();
                    }
                });
                ui.add_enabled_ui(self.scenario_running, |ui| {
                    if ui
                        .add(PrimaryButton::new(&self.theme, "정지").icon("⏹"))
                        .clicked()
                    {
                        self.stop_scenario();
                    }
                });
            });
        });
    }
}

impl eframe::App for BatchOrchestratorApp {
    /// egui 메인 루프에서 호출되어 UI를 갱신한다.
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.drain_events();
        self.theme.apply(ctx);
        let palette = *self.theme.palette();
        let decorations = *self.theme.decorations();
        let toolbar_frame = egui::Frame {
            fill: palette.bg_toolbar,
            stroke: egui::Stroke::new(1.0, palette.border_soft),
            rounding: egui::Rounding::same(decorations.toolbar_rounding),
            inner_margin: egui::Margin::symmetric(20.0, 20.0),
            ..Default::default()
        };
        egui::TopBottomPanel::top("toolbar")
            .frame(toolbar_frame)
            .resizable(false)
            .show(ctx, |ui| {
                self.render_toolbar(ui);
            });
        let sidebar_frame = egui::Frame {
            fill: palette.bg_sidebar,
            stroke: egui::Stroke::new(1.0, palette.border_soft),
            rounding: egui::Rounding::same(decorations.container_rounding),
            inner_margin: decorations.card_inner_margin,
            ..Default::default()
        };
        egui::SidePanel::left("steps")
            .resizable(false)
            .default_width(280.0)
            .frame(sidebar_frame)
            .show(ctx, |ui| {
                self.render_step_panel(ui);
            });
        let central_frame = egui::Frame {
            fill: palette.bg_main,
            stroke: egui::Stroke::new(1.0, palette.border_soft),
            rounding: egui::Rounding::same(decorations.container_rounding),
            inner_margin: egui::Margin::symmetric(22.0, 18.0),
            ..Default::default()
        };
        egui::CentralPanel::default()
            .frame(central_frame)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 18.0;
                    egui::Frame::none()
                        .fill(palette.bg_panel)
                        .stroke(egui::Stroke::new(1.0, palette.border_soft))
                        .rounding(egui::Rounding::same(decorations.card_rounding))
                        .inner_margin(decorations.card_inner_margin)
                        .show(ui, |ui| {
                            self.render_step_detail(ui);
                        });
                    egui::Frame::none()
                        .fill(palette.bg_log)
                        .stroke(egui::Stroke::new(1.0, palette.border_soft))
                        .rounding(egui::Rounding::same(decorations.card_rounding))
                        .inner_margin(decorations.card_inner_margin)
                        .show(ui, |ui| {
                            self.render_log_panel(ui);
                        });
                });
            });
        let progress_frame = egui::Frame {
            fill: palette.bg_panel,
            stroke: egui::Stroke::new(1.0, palette.border_soft),
            rounding: egui::Rounding::same(decorations.card_rounding),
            inner_margin: egui::Margin::symmetric(20.0, 12.0),
            ..Default::default()
        };
        egui::TopBottomPanel::bottom("progress")
            .frame(progress_frame)
            .show(ctx, |ui| {
                let ratio = self.progress_ratio();
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("📈 전체 진행률")
                            .color(palette.fg_text_primary)
                            .strong(),
                    );
                    ui.add_space(6.0);
                    ui.add(
                        egui::ProgressBar::new(ratio)
                            .fill(palette.accent_primary)
                            .text(format!("진행률: {:.0}%", ratio * 100.0)),
                    );
                });
            });
    }
}

/// StepStatus를 기반으로 직관적인 아이콘과 텍스트를 반환한다.
fn status_indicator(status: &StepStatus) -> (&'static str, &'static str) {
    match status {
        StepStatus::Pending => ("⏳", "대기 중"),
        StepStatus::Running => ("⚙️", "실행 중"),
        StepStatus::Success => ("✅", "성공"),
        StepStatus::Failed(_) => ("❌", "실패"),
    }
}

/// 단색 헤더를 그려 정보 영역의 시각적 위계를 만든다.
fn solid_section_header(ui: &mut egui::Ui, theme: &Theme, icon: &str, title: &str) {
    let decorations = theme.decorations();
    let palette = theme.palette();
    let size = egui::vec2(ui.available_width(), decorations.header_height);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().rect_filled(
        rect,
        egui::Rounding::same(decorations.header_rounding),
        decorations.header_fill,
    );
    ui.painter().rect_stroke(
        rect,
        egui::Rounding::same(decorations.header_rounding),
        egui::Stroke::new(
            1.0,
            blend_color(decorations.header_fill, palette.bg_panel, 0.4),
        ),
    );
    let content_rect = rect.shrink2(egui::vec2(16.0, 0.0));
    ui.allocate_ui_at_rect(content_rect, |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            if !icon.is_empty() {
                ui.label(
                    RichText::new(icon)
                        .size(decorations.header_icon_size)
                        .color(decorations.header_text),
                );
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new(title)
                    .size(18.0)
                    .color(decorations.header_text)
                    .strong(),
            );
        });
    });
}

/// 단색 배경과 일정한 간격을 제공하는 기본 버튼 위젯.
struct PrimaryButton<'a> {
    theme: &'a Theme,
    label: &'a str,
    icon: &'a str,
}

impl<'a> PrimaryButton<'a> {
    /// 버튼의 기본 정보를 생성한다.
    fn new(theme: &'a Theme, label: &'a str) -> Self {
        Self {
            theme,
            label,
            icon: "",
        }
    }

    /// 버튼에 표시할 아이콘(이모지)을 설정한다.
    fn icon(mut self, icon: &'a str) -> Self {
        self.icon = icon;
        self
    }
}

impl<'a> Widget for PrimaryButton<'a> {
    /// egui 위젯 트레이트를 구현하여 버튼을 화면에 그린다.
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let decorations = self.theme.decorations();
        let palette = self.theme.palette();
        let enabled = ui.is_enabled();
        let button_padding = ui.style().spacing.button_padding.x;

        // 텍스트 레이아웃
        let galley = ui.painter().layout_no_wrap(
            self.label.to_string(),
            egui::TextStyle::Button.resolve(ui.style()),
            palette.fg_text_primary,
        );

        // 아이콘 공간 계산
        let icon_space = if self.icon.is_empty() { 0.0 } else { 28.0 };

        // 버튼의 원하는 너비 계산
        let desired_width = galley.size().x + icon_space + button_padding * 2.0 + decorations.button_min_width * 0.1;
        let size = egui::vec2(
            desired_width.max(decorations.button_min_width), // 최소 너비
            decorations.button_height, // 버튼 높이
        );

        // 버튼 배치 및 클릭 감지 (Button 위젯을 사용하여 클릭 가능 영역 확장)
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

        // 버튼 상태에 따라 색상 변경
        let mut fill = palette.accent_primary;
        if !enabled {
            fill = blend_color(fill, palette.border_soft, 0.5);
        } else if response.is_pointer_button_down_on() {
            fill = blend_color(fill, palette.fg_text_primary, 0.2);
        } else if response.hovered() {
            fill = blend_color(fill, palette.bg_panel, 0.2);
        }

        // 버튼 그리기 (배경색)
        ui.painter().rect_filled(
            rect,
            egui::Rounding::same(decorations.button_rounding),
            fill,
        );

        // 버튼 테두리 그리기
        ui.painter().rect_stroke(
            rect,
            egui::Rounding::same(decorations.button_rounding),
            egui::Stroke::new(1.0, blend_color(fill, palette.border_soft, 0.6)),
        );

        // 텍스트 색상 (활성화 여부에 따라 다르게 설정)
        let text_color = if enabled {
            egui::Color32::WHITE
        } else {
            blend_color(palette.fg_text_secondary, palette.bg_panel, 0.4)
        };

        // 버튼 내용(아이콘과 텍스트) 그리기
        let content_rect = rect.shrink2(egui::vec2(button_padding, 0.0));

        // 버튼 클릭 가능 영역에 텍스트 및 아이콘 추가
        ui.allocate_ui_at_rect(content_rect, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;  // 아이콘과 텍스트 간 간격 조정

                // 아이콘 표시 (빈 경우 제외)
                if !self.icon.is_empty() {
                    ui.label(RichText::new(self.icon).size(18.0).color(text_color));
                }

                // 텍스트 표시
                ui.label(
                    RichText::new(self.label)
                        .size(16.0)
                        .color(text_color)
                        .strong(),
                );
            });
        });

        response
    }
}


