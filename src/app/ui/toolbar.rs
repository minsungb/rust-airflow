use super::*;

impl BatchOrchestratorApp {
    /// 실행 탭 상단 툴바를 그린다.
    pub(super) fn render_run_toolbar(&mut self, ui: &mut egui::Ui) {
        let decorations = *self.theme.decorations();
        let palette = *self.theme.palette();
        ui.vertical(|ui| {
            ui.label(
                RichText::new("✨ Rust Batch Orchestrator")
                    .size(20.0)
                    .color(palette.fg_text_primary)
                    .strong(),
            );
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
                ui.label(RichText::new(err).color(palette.accent_error).strong());
                ui.add_space(10.0);
            }
            ui.add_space(8.0);
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
                    .add_enabled(can_run, PrimaryButton::new(&self.theme, "실행").icon("▶"))
                    .clicked()
                {
                    self.start_scenario();
                }

                let can_stop = self.scenario_running;
                if ui
                    .add_enabled(can_stop, PrimaryButton::new(&self.theme, "정지").icon("⏹"))
                    .clicked()
                {
                    self.stop_scenario();
                }
            });
        });
    }

    /// 시나리오 빌더 전용 툴바를 렌더링한다.
    pub(super) fn render_builder_toolbar(&mut self, ui: &mut egui::Ui) {
        let palette = *self.theme.palette();
        ui.vertical(|ui| {
            ui.label(
                RichText::new("🛠️ Scenario Builder")
                    .size(20.0)
                    .color(palette.fg_text_primary)
                    .strong(),
            );
            if let Some(path) = &self.editor_state.current_file {
                let dirty = if self.editor_state.dirty {
                    " (수정됨)"
                } else {
                    ""
                };
                ui.label(
                    RichText::new(format!("파일 · {}{}", path.display(), dirty))
                        .color(palette.fg_text_secondary),
                );
            } else {
                let dirty = if self.editor_state.dirty {
                    " · 수정됨"
                } else {
                    ""
                };
                ui.label(
                    RichText::new(format!("새 시나리오{}", dirty)).color(palette.fg_text_secondary),
                );
            }
            if let Some(err) = &self.editor_error {
                ui.label(RichText::new(err).color(palette.accent_error).strong());
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add(PrimaryButton::new(&self.theme, "새 시나리오").icon("🆕"))
                    .clicked()
                {
                    self.editor_new_document();
                }
                if ui
                    .add(PrimaryButton::new(&self.theme, "열기...").icon("📂"))
                    .clicked()
                {
                    self.editor_open_dialog();
                }
                if ui
                    .add(PrimaryButton::new(&self.theme, "저장").icon("💾"))
                    .clicked()
                {
                    self.editor_save(false);
                }
                if ui
                    .add(PrimaryButton::new(&self.theme, "다른 이름으로").icon("📝"))
                    .clicked()
                {
                    self.editor_save(true);
                }
                // 시나리오 빌더에 실행 제거
                // if ui
                //     .add(PrimaryButton::new(&self.theme, "실행").icon("🚀"))
                //     .clicked()
                // {
                //     self.editor_run_current();
                // }
            });
        });
    }
}
