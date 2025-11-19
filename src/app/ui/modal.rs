use super::*;

impl BatchOrchestratorApp {
    /// 컨펌 모달을 렌더링해 사용자 응답을 수집한다.
    pub(super) fn render_confirm_modal(&mut self, ctx: &egui::Context) {
        if let Some(request) = self.pending_confirms.first().cloned() {
            let palette = *self.theme.palette();
            egui::Window::new("Step 실행 확인")
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .title_bar(false)
                .show(ctx, |ui| {
                    ui.set_width(420.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("사용자 컨펌이 필요합니다")
                                .size(20.0)
                                .color(palette.fg_text_primary)
                                .strong(),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(format!(
                                "Step · {} ({})",
                                request.step_name, request.step_id
                            ))
                            .color(palette.fg_text_primary)
                            .strong(),
                        );
                        let phase_label = match request.phase {
                            ConfirmPhase::Before => "실행 전",
                            ConfirmPhase::After => "실행 후",
                        };
                        ui.label(format!(
                            "종류: {} · 단계: {}",
                            request.step_kind, phase_label
                        ));
                        if let Some(summary) = &request.summary {
                            ui.add_space(6.0);
                            ui.label("요약");
                            let mut summary_buf = summary.clone();
                            ui.add(
                                egui::TextEdit::multiline(&mut summary_buf)
                                    .desired_rows(4)
                                    .desired_width(f32::INFINITY)
                                    .font(egui::TextStyle::Monospace)
                                    .interactive(false),
                            );
                        }
                        if let Some(message) = &request.message {
                            ui.add_space(6.0);
                            ui.label(RichText::new(message).strong());
                        }
                        ui.add_space(6.0);
                        ui.label(format!(
                            "기본 응답: {}",
                            match request.default_answer {
                                ConfirmDefault::Yes => "예",
                                ConfirmDefault::No => "아니오",
                            }
                        ));
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add(PrimaryButton::new(&self.theme, "예 · 계속").icon("✅"))
                                .clicked()
                            {
                                self.respond_confirm(request.request_id, true);
                            }
                            if ui
                                .add(PrimaryButton::new(&self.theme, "아니오 · 중단").icon("🛑"))
                                .clicked()
                            {
                                self.respond_confirm(request.request_id, false);
                            }
                        });
                    });
                });
        }
    }
}
