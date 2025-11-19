use super::*;

impl<'a> ScenarioBuilderUi<'a> {
    /// Step 팔레트를 렌더링한다.
    pub(super) fn render_palette(&mut self, ui: &mut egui::Ui) {
        ui.heading("🧱 Step 팔레트");
        ui.separator();
        ui.label("추가할 Step 유형을 선택하세요.");
        ui.add_space(10.0);
        for (label, kind) in [
            ("SQL", StepKind::Sql),
            ("SQL 파일", StepKind::SqlFile),
            ("SQL*Loader", StepKind::SqlLoaderPar),
            ("Shell", StepKind::Shell),
            ("Extract (값 추출)", StepKind::Extract),
            ("Loop (반복)", StepKind::Loop),
        ] {
            if ui.button(label).clicked() {
                self.get_state_mut().add_node(kind);
            }
        }
    }
}
