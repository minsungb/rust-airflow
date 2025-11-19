use super::*;

impl BatchOrchestratorApp {
    /// 탭 선택 바를 렌더링한다.
    fn render_tab_selector(&mut self, ctx: &egui::Context) {
        let palette = *self.theme.palette();
        let decorations = *self.theme.decorations();
        let frame = egui::Frame {
            fill: palette.bg_panel,
            stroke: egui::Stroke::new(1.0, palette.border_soft),
            rounding: egui::Rounding::same(decorations.container_rounding),
            inner_margin: egui::Margin::symmetric(12.0, 8.0),
            ..Default::default()
        };
        egui::TopBottomPanel::top("tab_selector")
            .frame(frame)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    let tabs = [
                        (AppTab::Run, "실행"),
                        (AppTab::ScenarioBuilder, "Scenario Builder"),
                    ];
                    for (tab, label) in tabs {
                        let selected = self.active_tab == tab;
                        if ui.selectable_label(selected, label).clicked() {
                            self.active_tab = tab;
                        }
                    }
                });
            });
    }

    /// 실행 탭 전체 레이아웃을 렌더링한다.
    fn render_run_view(&mut self, ctx: &egui::Context) {
        let palette = *self.theme.palette();
        let decorations = *self.theme.decorations();
        let toolbar_frame = egui::Frame {
            fill: palette.bg_toolbar,
            stroke: egui::Stroke::new(1.0, palette.border_soft),
            rounding: egui::Rounding::same(decorations.toolbar_rounding),
            inner_margin: decorations.card_inner_margin,
            ..Default::default()
        };
        egui::TopBottomPanel::top("run_toolbar")
            .frame(toolbar_frame)
            .resizable(false)
            .show(ctx, |ui| {
                self.render_run_toolbar(ui);
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
            .default_width(320.0)
            .frame(sidebar_frame)
            .show(ctx, |ui| {
                self.render_step_panel(ui);
            });
        let central_frame = egui::Frame {
            fill: palette.bg_main,
            stroke: egui::Stroke::new(1.0, palette.border_soft),
            rounding: egui::Rounding::same(decorations.container_rounding),
            inner_margin: decorations.card_inner_margin,
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
                        .auto_shrink([false; 2])
                        .max_height(400.0)
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

    /// 시나리오 빌더 탭 전체 레이아웃을 렌더링한다.
    fn render_builder_view(&mut self, ctx: &egui::Context) {
        let palette = *self.theme.palette();
        let decorations = *self.theme.decorations();
        let toolbar_frame = egui::Frame {
            fill: palette.bg_toolbar,
            stroke: egui::Stroke::new(1.0, palette.border_soft),
            rounding: egui::Rounding::same(decorations.toolbar_rounding),
            inner_margin: decorations.card_inner_margin,
            ..Default::default()
        };
        egui::TopBottomPanel::top("builder_toolbar")
            .frame(toolbar_frame)
            .resizable(false)
            .show(ctx, |ui| {
                self.render_builder_toolbar(ui);
            });
        let mut builder_ui = ScenarioBuilderUi::new(&self.theme, &mut self.editor_state);
        builder_ui.show(ctx);
    }
}

impl eframe::App for BatchOrchestratorApp {
    /// egui 메인 루프에서 호출되어 UI를 갱신한다.
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.drain_events();
        self.theme.apply(ctx);
        self.render_tab_selector(ctx);
        match self.active_tab {
            AppTab::Run => self.render_run_view(ctx),
            AppTab::ScenarioBuilder => self.render_builder_view(ctx),
        }
        self.render_confirm_modal(ctx);
    }
}
