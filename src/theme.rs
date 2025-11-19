use crate::engine::StepStatus;
use eframe::egui::{self, Color32};

include!(concat!(env!("OUT_DIR"), "/custom_font.rs"));

/// 테마와 관련된 여백, 모서리, 색상 설정을 담는다.
#[derive(Clone, Copy, Debug)]
pub struct ThemeDecorations {
    /// 카드와 패널에 적용할 라운딩 값.
    pub card_rounding: f32,
    /// 사이드바 및 대형 컨테이너용 라운딩 값.
    pub container_rounding: f32,
    /// 버튼용 라운딩 값.
    pub button_rounding: f32,
    /// 툴바 라운딩 값.
    pub toolbar_rounding: f32,
    /// 섹션 헤더 라운딩 값.
    pub header_rounding: f32,
    /// 버튼 최소 너비.
    pub button_min_width: f32,
    /// 버튼 높이.
    pub button_height: f32,
    /// 버튼 간격.
    pub button_gap: f32,
    /// 섹션 헤더 높이.
    pub header_height: f32,
    /// 헤더 아이콘 크기.
    pub header_icon_size: f32,
    /// 섹션 헤더 배경 색.
    pub header_fill: Color32,
    /// 섹션 헤더 텍스트 색.
    pub header_text: Color32,
    /// 카드 안쪽 여백.
    pub card_inner_margin: egui::Margin,
}

impl ThemeDecorations {
    /// 라이트 테마 기본 장식 값을 반환한다.
    pub fn light() -> Self {
        Self {
            card_rounding: 0.0,
            container_rounding: 0.0,
            button_rounding: 0.0,
            toolbar_rounding: 0.0,
            header_rounding: 0.0,
            button_min_width: 150.0,
            button_height: 40.0,
            button_gap: 16.0,
            header_height: 42.0,
            header_icon_size: 24.0,
            header_fill: Color32::from_rgb(76, 128, 255),
            header_text: Color32::from_rgb(255, 255, 255),
            card_inner_margin: egui::Margin::symmetric(18.0, 16.0),
        }
    }
}

/// 테마 종류를 정의한다. 현재는 라이트 테마만 구현되어 있으며 추후 다크 테마를 추가할 수 있다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeVariant {
    /// 부드러운 파스텔 라이트 테마.
    Light,
}

/// 시나리오 빌더 전용 색상 묶음이다.
#[derive(Clone, Copy, Debug)]
pub struct BuilderColors {
    /// 캔버스 배경색.
    pub canvas_fill: Color32,
    /// 노드 배경색.
    pub node_fill: Color32,
    /// 선택된 노드 배경색.
    pub node_selected: Color32,
    /// 노드 테두리 색상.
    pub node_border: Color32,
    /// 연결선 색상.
    pub connection_stroke: Color32,
    /// 텍스트 주 색상.
    pub text_primary: Color32,
    /// 텍스트 보조 색상.
    pub text_secondary: Color32,
    /// 핸들 색상.
    pub handle_fill: Color32,
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
    /// 인터랙티브 아이콘 하이라이트 색상.
    pub icon_emphasis: Color32,
    /// 빌더 캔버스 배경색.
    pub builder_canvas: Color32,
    /// 빌더 노드 배경색.
    pub builder_node: Color32,
    /// 선택된 노드 배경색.
    pub builder_node_selected: Color32,
    /// 빌더 연결선 색상.
    pub builder_connection: Color32,
    /// 빌더 노드 테두리 색상.
    pub builder_node_border: Color32,
    /// 빌더 핸들 색상.
    pub builder_handle: Color32,
}

impl ThemePalette {
    /// 라이트 테마용 파스텔 팔레트를 반환한다.
    pub const fn light() -> Self {
        Self {
            bg_main: Color32::from_rgb(240, 240, 240),
            bg_panel: Color32::from_rgb(250, 250, 250),
            bg_toolbar: Color32::from_rgb(245, 245, 245),
            bg_sidebar: Color32::from_rgb(248, 248, 248),
            bg_log: Color32::from_rgb(255, 255, 255),
            fg_text_primary: Color32::from_rgb(51, 51, 51),
            fg_text_secondary: Color32::from_rgb(102, 102, 102),
            accent_primary: Color32::from_rgb(76, 128, 255),
            accent_success: Color32::from_rgb(76, 175, 80),
            accent_warning: Color32::from_rgb(255, 191, 0),
            accent_error: Color32::from_rgb(234, 67, 53),
            border_soft: Color32::from_rgb(210, 210, 210),
            accent_pending: Color32::from_rgb(189, 189, 189),
            icon_emphasis: Color32::from_rgb(76, 128, 255),
            builder_canvas: Color32::from_rgb(237, 240, 247),
            builder_node: Color32::from_rgb(255, 255, 255),
            builder_node_selected: Color32::from_rgb(223, 235, 255),
            builder_connection: Color32::from_rgb(96, 125, 139),
            builder_node_border: Color32::from_rgb(164, 177, 190),
            builder_handle: Color32::from_rgb(76, 128, 255),
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
    /// 세부 장식 정보.
    decorations: ThemeDecorations,
}

impl Theme {
    /// 라이트 테마 인스턴스를 생성한다.
    pub fn light() -> Self {
        Self {
            variant: ThemeVariant::Light,
            palette: ThemePalette::light(),
            decorations: ThemeDecorations::light(),
        }
    }

    /// 현재 테마 팔레트를 반환한다.
    pub fn palette(&self) -> &ThemePalette {
        &self.palette
    }

    /// 현재 테마의 장식 설정을 반환한다.
    pub fn decorations(&self) -> &ThemeDecorations {
        &self.decorations
    }

    /// egui Context에 테마 기반 스타일을 적용한다.
    pub fn apply(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::light();
        let palette = self.palette;
        let decorations = self.decorations;
        visuals.override_text_color = Some(palette.fg_text_primary);
        visuals.window_fill = palette.bg_panel;
        visuals.panel_fill = palette.bg_panel;
        visuals.extreme_bg_color = palette.bg_main;
        visuals.widgets.noninteractive.bg_fill = palette.bg_panel;
        visuals.widgets.noninteractive.fg_stroke.color = palette.fg_text_secondary;
        visuals.widgets.inactive.bg_fill = palette.bg_panel;
        visuals.widgets.inactive.fg_stroke.color = palette.fg_text_primary;
        visuals.widgets.inactive.rounding = egui::Rounding::same(decorations.button_rounding);
        visuals.widgets.hovered.bg_fill =
            blend_color(palette.accent_primary, palette.bg_panel, 0.35);
        visuals.widgets.hovered.fg_stroke.color = palette.fg_text_primary;
        visuals.widgets.hovered.rounding = egui::Rounding::same(decorations.button_rounding);
        visuals.widgets.active.bg_fill = palette.accent_primary;
        visuals.widgets.active.fg_stroke.color = palette.fg_text_primary;
        visuals.widgets.active.rounding = egui::Rounding::same(decorations.button_rounding);
        visuals.selection.bg_fill = palette.accent_primary;
        visuals.selection.stroke.color = palette.border_soft;
        visuals.window_rounding = egui::Rounding::same(decorations.container_rounding);
        visuals.button_frame = true;
        visuals.faint_bg_color = palette.bg_main;
        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(14.0, 12.0);
        style.spacing.button_padding = egui::vec2(18.0, 12.0);
        style.spacing.window_margin = egui::Margin::symmetric(20.0, 16.0);
        style.animation_time = 0.28;
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

    /// 시나리오 빌더 색상을 반환한다.
    pub fn builder_colors(&self) -> BuilderColors {
        let palette = self.palette;
        BuilderColors {
            canvas_fill: palette.builder_canvas,
            node_fill: palette.builder_node,
            node_selected: palette.builder_node_selected,
            node_border: palette.builder_node_border,
            connection_stroke: palette.builder_connection,
            text_primary: palette.fg_text_primary,
            text_secondary: palette.fg_text_secondary,
            handle_fill: palette.builder_handle,
        }
    }
}

impl Default for Theme {
    /// 기본 테마 색상을 정의한다.
    fn default() -> Self {
        Self::light()
    }
}

/// 지정한 두 색상을 비율에 맞춰 혼합한다.
pub fn blend_color(foreground: Color32, background: Color32, ratio: f32) -> Color32 {
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
