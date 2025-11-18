use crate::engine::StepStatus;
use eframe::egui::{self, Color32};

include!(concat!(env!("OUT_DIR"), "/custom_font.rs"));

/// 테마 종류를 정의한다. 현재는 라이트 테마만 구현되어 있으며 추후 다크 테마를 추가할 수 있다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeVariant {
    /// 부드러운 파스텔 라이트 테마.
    Light,
}

/// 라이트/다크 모드 공통으로 사용할 색상 팔레트를 정의한다.
#[derive(Clone, Copy, Debug)]
pub struct ThemePalette {
    /// 전체 배경색.
    pub bg_main: Color32,
    /// 패널 및 카드 배경색.
    pub bg_panel: Color32,
    /// 툴바 배경색.
    pub bg_toolbar: Color32,
    /// 좌측 사이드바 배경색.
    pub bg_sidebar: Color32,
    /// 로그 영역 배경색.
    pub bg_log: Color32,
    /// 기본 텍스트 색상.
    pub fg_text_primary: Color32,
    /// 보조 텍스트 색상.
    pub fg_text_secondary: Color32,
    /// 기본 강조 색상.
    pub accent_primary: Color32,
    /// 성공 상태 색상.
    pub accent_success: Color32,
    /// 경고/진행 중 상태 색상.
    pub accent_warning: Color32,
    /// 오류 상태 색상.
    pub accent_error: Color32,
    /// 부드러운 테두리 색상.
    pub border_soft: Color32,
    /// 대기 상태 색상.
    pub accent_pending: Color32,
}

impl ThemePalette {
    /// 라이트 테마용 파스텔 팔레트를 반환한다.
    pub const fn light() -> Self {
        Self {
            bg_main: Color32::from_rgb(245, 242, 238),
            bg_panel: Color32::from_rgb(255, 255, 252),
            bg_toolbar: Color32::from_rgb(253, 248, 240),
            bg_sidebar: Color32::from_rgb(249, 246, 240),
            bg_log: Color32::from_rgb(252, 249, 245),
            fg_text_primary: Color32::from_rgb(40, 40, 40),
            fg_text_secondary: Color32::from_rgb(102, 99, 92),
            accent_primary: Color32::from_rgb(96, 159, 210),
            accent_success: Color32::from_rgb(104, 186, 148),
            accent_warning: Color32::from_rgb(241, 180, 76),
            accent_error: Color32::from_rgb(229, 107, 111),
            border_soft: Color32::from_rgb(215, 206, 194),
            accent_pending: Color32::from_rgb(223, 217, 207),
        }
    }
}

/// UI 전체에서 참조할 공통 테마 정보.
#[derive(Clone, Debug)]
pub struct Theme {
    /// 적용 중인 테마 종류.
    variant: ThemeVariant,
    /// 테마 팔레트 데이터.
    palette: ThemePalette,
}

impl Theme {
    /// 라이트 테마 인스턴스를 생성한다.
    pub fn light() -> Self {
        Self {
            variant: ThemeVariant::Light,
            palette: ThemePalette::light(),
        }
    }

    /// 현재 테마 팔레트를 반환한다.
    pub fn palette(&self) -> &ThemePalette {
        &self.palette
    }

    /// egui Context에 테마 기반 스타일을 적용한다.
    pub fn apply(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::light();
        let palette = self.palette;
        visuals.override_text_color = Some(palette.fg_text_primary);
        visuals.window_fill = palette.bg_panel;
        visuals.panel_fill = palette.bg_panel;
        visuals.extreme_bg_color = palette.bg_main;
        visuals.widgets.noninteractive.bg_fill = palette.bg_panel;
        visuals.widgets.noninteractive.fg_stroke.color = palette.fg_text_secondary;
        visuals.widgets.inactive.bg_fill = palette.bg_panel;
        visuals.widgets.inactive.fg_stroke.color = palette.fg_text_primary;
        visuals.widgets.inactive.rounding = egui::Rounding::same(10.0);
        visuals.widgets.hovered.bg_fill =
            blend_color(palette.accent_primary, palette.bg_panel, 0.2);
        visuals.widgets.hovered.fg_stroke.color = palette.fg_text_primary;
        visuals.widgets.hovered.rounding = egui::Rounding::same(10.0);
        visuals.widgets.active.bg_fill = palette.accent_primary;
        visuals.widgets.active.fg_stroke.color = palette.fg_text_primary;
        visuals.widgets.active.rounding = egui::Rounding::same(10.0);
        visuals.selection.bg_fill = palette.accent_primary;
        visuals.selection.stroke.color = palette.border_soft;
        visuals.window_rounding = egui::Rounding::same(14.0);
        visuals.button_frame = true;
        visuals.faint_bg_color = palette.bg_main;
        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(12.0, 10.0);
        style.spacing.button_padding = egui::vec2(14.0, 10.0);
        style.spacing.window_margin = egui::Margin::symmetric(18.0, 16.0);
        style.visuals = ctx.style().visuals.clone();
        ctx.set_style(style);
        install_custom_font(ctx);
    }

    /// StepStatus에 대응하는 색상을 반환한다.
    pub fn status_color(&self, status: &StepStatus) -> Color32 {
        match status {
            StepStatus::Pending => self.palette.accent_pending,
            StepStatus::Running => self.palette.accent_warning,
            StepStatus::Success => self.palette.accent_success,
            StepStatus::Failed(_) => self.palette.accent_error,
        }
    }

    /// 현재 테마의 종류를 반환한다.
    pub fn variant(&self) -> ThemeVariant {
        self.variant
    }
}

impl Default for Theme {
    /// 기본 테마 색상을 정의한다.
    fn default() -> Self {
        Self::light()
    }
}

/// 지정한 두 색상을 비율에 맞춰 혼합한다.
fn blend_color(foreground: Color32, background: Color32, ratio: f32) -> Color32 {
    let mix = |fg: u8, bg: u8| -> u8 {
        let fg_f = fg as f32;
        let bg_f = bg as f32;
        (fg_f * (1.0 - ratio) + bg_f * ratio)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color32::from_rgb(
        mix(foreground.r(), background.r()),
        mix(foreground.g(), background.g()),
        mix(foreground.b(), background.b()),
    )
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
