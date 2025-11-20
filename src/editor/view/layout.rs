use super::*;

/// Scenario Builder 화면 전체를 담당하는 뷰이다.
pub struct ScenarioBuilderUi<'a> {
    /// 테마 참조.
    theme: &'a Theme,
    /// 에디터 상태 참조.
    state: &'a mut ScenarioEditorState,
}

impl<'a> ScenarioBuilderUi<'a> {
    /// 에디터 상태에 대한 불변 참조를 반환한다.
    pub fn get_state(&self) -> &ScenarioEditorState {
        &self.state
    }

    /// 에디터 상태에 대한 가변 참조를 반환한다.
    pub fn get_state_mut(&mut self) -> &mut ScenarioEditorState {
        &mut self.state
    }

    /// 전달된 ID를 선택 상태로 설정하고 동일한 ID를 반환한다.
    pub fn select_node(&mut self, id: &Option<String>) -> Option<String> {
        self.state.select_node(id.clone());
        id.clone()
    }

    /// 노드 선택 상태를 초기화한다.
    pub fn clear_selection(&mut self) {
        self.state.select_node(None);
    }

    /// 현재 테마에 대한 참조를 반환한다.
    pub fn get_theme(&self) -> &Theme {
        self.theme
    }

    /// 뷰 인스턴스를 생성한다.
    pub fn new(theme: &'a Theme, state: &'a mut ScenarioEditorState) -> Self {
        Self { theme, state }
    }

    /// 좌/중앙/우 패널을 구성한다.
    pub fn show(&mut self, ctx: &egui::Context) {
        let palette = *self.theme.palette();
        let decorations = *self.theme.decorations();
        let builder_colors = self.theme.builder_colors();
        let palette_frame = egui::Frame {
            fill: palette.bg_sidebar,
            stroke: egui::Stroke::new(1.0, palette.border_soft),
            rounding: egui::Rounding::same(decorations.container_rounding),
            inner_margin: decorations.card_inner_margin,
            ..Default::default()
        };
        egui::SidePanel::left("builder_palette")
            .frame(palette_frame)
            .resizable(false)
            .default_width(220.0)
            .show(ctx, |ui| {
                self.render_palette(ui);
            });
        let property_frame = egui::Frame {
            fill: palette.bg_sidebar,
            stroke: egui::Stroke::new(1.0, palette.border_soft),
            rounding: egui::Rounding::same(decorations.container_rounding),
            inner_margin: decorations.card_inner_margin,
            ..Default::default()
        };
        egui::SidePanel::right("builder_properties")
            .frame(property_frame)
            .resizable(false)
            .show(ctx, |ui| {
                self.render_properties(ui);
            });
        let canvas_frame = egui::Frame {
            fill: builder_colors.canvas_fill,
            stroke: egui::Stroke::new(1.0, palette.border_soft),
            rounding: egui::Rounding::same(decorations.container_rounding),
            inner_margin: egui::Margin::same(12.0),
            ..Default::default()
        };
        egui::CentralPanel::default()
            .frame(canvas_frame)
            .show(ctx, |ui| {
                self.render_canvas(ui, builder_colors);
            });
    }
}
