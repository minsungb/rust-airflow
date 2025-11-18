use crate::engine::StepStatus;
use eframe::egui::{self, Color32};

include!(concat!(env!("OUT_DIR"), "/custom_font.rs"));

/// UI 전체에서 참조할 공통 테마 정보.
pub struct Theme {
    /// 대기 상태 색상.
    pub pending: Color32,
    /// 실행 중 상태 색상.
    pub running: Color32,
    /// 성공 상태 색상.
    pub success: Color32,
    /// 실패 상태 색상.
    pub failure: Color32,
    /// 기본 패널 테두리 색상.
    pub frame: Color32,
}

impl Default for Theme {
    /// 기본 테마 색상을 정의한다.
    fn default() -> Self {
        Self {
            pending: Color32::from_rgb(200, 200, 200),
            running: Color32::from_rgb(80, 160, 255),
            success: Color32::from_rgb(60, 180, 120),
            failure: Color32::from_rgb(220, 80, 80),
            frame: Color32::from_rgb(50, 50, 50),
        }
    }
}

impl Theme {
    /// egui Context에 테마 기반 스타일을 적용한다.
    pub fn apply(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.window_rounding = egui::Rounding::same(6.0);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(30, 30, 30);
        visuals.panel_fill = Color32::from_rgb(24, 24, 24);
        ctx.set_visuals(visuals);
        install_custom_font(ctx);
    }

    /// StepStatus에 대응하는 색상을 반환한다.
    pub fn status_color(&self, status: &StepStatus) -> Color32 {
        match status {
            StepStatus::Pending => self.pending,
            StepStatus::Running => self.running,
            StepStatus::Success => self.success,
            StepStatus::Failed(_) => self.failure,
        }
    }
}

/// build.rs에서 추출한 폰트를 egui에 등록한다.
pub fn install_custom_font(ctx: &egui::Context) {
    if let Some(bytes) = embedded_font_bytes() {
        let mut fonts = egui::FontDefinitions::default();
        fonts
            .font_data
            .insert("custom".into(), egui::FontData::from_static(bytes));
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "custom".into());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "custom".into());
        ctx.set_fonts(fonts);
    }
}
