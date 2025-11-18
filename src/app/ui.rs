use crate::engine::StepStatus;
use eframe::egui::{self, RichText};

use super::state::BatchOrchestratorApp;
use super::widgets::{PrimaryButton, solid_section_header};

impl BatchOrchestratorApp {
    /// 좌측 Step 리스트 패널을 그린다.
    pub(super) fn render_step_panel(&mut self, ui: &mut egui::Ui) {
        solid_section_header(ui, &self.theme, "🧭", "작업 단계");
        ui.add_space(12.0);
        ui.spacing_mut().item_spacing.y = 12.0;
        let palette = *self.theme.palette();
        let decorations = *self.theme.decorations();
        if let Some(scenario) = &self.scenario {
            for step in &scenario.steps {
                let state = self.step_states.get(&step.id).cloned().unwrap_or_default();
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
    pub(super) fn render_step_detail(&self, ui: &mut egui::Ui) {
        solid_section_header(ui, &self.theme, "🧩", "Step 정보");
        ui.add_space(10.0);
        let palette = *self.theme.palette();
        if let Some(step_id) = &self.selected_step {
            if let Some(scenario) = &self.scenario {
                if let Some(step) = scenario.steps.iter().find(|s| &s.id == step_id) {
                    let state = self.step_states.get(step_id).cloned().unwrap_or_default();
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
    pub(super) fn render_log_panel(&self, ui: &mut egui::Ui) {
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
    pub(super) fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        let decorations = *self.theme.decorations();
        let palette = *self.theme.palette();
        ui.set_min_height(150.0);
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
            if let Some(err) = &self.last_error {
                ui.label(
                    RichText::new(err)
                        .color(palette.accent_error)
                        .strong(),
                );
                ui.add_space(10.0);
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = decorations.button_gap;

                if ui
                    .add(PrimaryButton::new(&self.theme, "열기").icon("📂"))
                    .clicked()
                {
                    self.load_scenario_from_dialog();
                }

                let can_run = self.scenario.is_some() && !self.scenario_running;
                if ui
                    .add_enabled(
                        can_run,
                        PrimaryButton::new(&self.theme, "실행").icon("▶"),
                    )
                    .clicked()
                {
                    self.start_scenario();
                }

                let can_stop = self.scenario_running;
                if ui
                    .add_enabled(
                        can_stop,
                        PrimaryButton::new(&self.theme, "정지").icon("⏹"),
                    )
                    .clicked()
                {
                    self.stop_scenario();
                }
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
                    egui::ScrollArea::vertical()
                    .auto_shrink([false; 2]) // 작은 크기일 때 자동 축소 방지
                    .max_height(400.0) // 최대 높이 설정
                    .show(ui, |ui| {
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
