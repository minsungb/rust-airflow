use super::*;

impl BatchOrchestratorApp {
    /// 좌측 Step 리스트 패널을 그린다.
    pub(super) fn render_step_panel(&mut self, ui: &mut egui::Ui) {
        let palette = *self.theme.palette();
        solid_section_header(ui, &self.theme, "🧭", "작업 단계");
        ui.add_space(12.0);
        ui.spacing_mut().item_spacing.y = 12.0;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(scenario) = &self.scenario {
                    for step in &scenario.steps {
                        let state = self.step_states.get(&step.id).cloned().unwrap_or_default();
                        let status_color = self.theme.status_color(&state.status);
                        let (status_icon, status_text) = status::status_indicator(&state.status);
                        let is_selected = self.selected_step.as_deref() == Some(step.id.as_str());

                        let response = ui.add(
                            StepCard::new(&self.theme, step.name.as_str(), step.id.as_str())
                                .status(status_icon, status_text, status_color)
                                .selected(is_selected),
                        );

                        if response.clicked() {
                            self.selected_step = Some(step.id.clone());
                        }
                    }
                } else {
                    let info = egui::RichText::new("시나리오를 먼저 불러오세요.")
                        .color(palette.fg_text_secondary)
                        .italics();
                    ui.label(info);
                }
            });
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
                    let (_, status_text) = status::status_indicator(&state.status);
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
}
