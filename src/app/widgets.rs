use crate::theme::{Theme, blend_color};
use eframe::egui::{self, RichText, Widget};

/// StepCard는 좌측 Step 리스트에서 재사용할 수 있는 공통 카드 레이아웃을 제공한다.
pub(super) struct StepCard<'a> {
    theme: &'a Theme,
    name: &'a str,
    step_id: &'a str,
    status_icon: &'a str,
    status_text: &'a str,
    status_color: egui::Color32,
    is_selected: bool,
    height: f32,
}

impl<'a> StepCard<'a> {
    /// StepCard 기본 구성을 생성하며 이름과 ID 정보를 설정한다.
    pub(super) fn new(theme: &'a Theme, name: &'a str, step_id: &'a str) -> Self {
        let fallback_color = theme.palette().fg_text_secondary;
        Self {
            theme,
            name,
            step_id,
            status_icon: "",
            status_text: "",
            status_color: fallback_color,
            is_selected: false,
            height: 74.0,
        }
    }

    /// 상태 아이콘/텍스트와 색상을 지정해 카드에 상태 강조를 적용한다.
    pub(super) fn status(mut self, icon: &'a str, text: &'a str, color: egui::Color32) -> Self {
        self.status_icon = icon;
        self.status_text = text;
        self.status_color = color;
        self
    }

    /// 현재 카드가 선택되었는지 여부를 지정한다.
    pub(super) fn selected(mut self, selected: bool) -> Self {
        self.is_selected = selected;
        self
    }

    /// 카드 높이를 조정해 다양한 레이아웃 요구를 맞춘다.
    pub(super) fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl<'a> Widget for StepCard<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let palette = *self.theme.palette();
        let decorations = *self.theme.decorations();

        let desired_size = egui::vec2(ui.available_width(), self.height);

        // 여기서는 레이아웃만 확보 (Sense::hover 정도만 줘도 됨)
        let (rect, _dummy_response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        if !ui.is_rect_visible(rect) {
            // 아직 화면에 안 보이면 바로 반환
            return _dummy_response;
        }

        // ---------- 여기까지: 카드 배치/rect 계산 ----------

        // 배경/테두리 그리기
        let fill = if self.is_selected {
            palette.bg_panel
        } else {
            palette.bg_sidebar
        };

        let stroke_color = if self.is_selected {
            self.status_color
        } else {
            palette.border_soft
        };

        ui.painter().rect(
            rect,
            egui::Rounding::same(decorations.card_rounding),
            fill,
            egui::Stroke::new(1.5, stroke_color),
        );

        // 좌측 상태 인디케이터
        let indicator =
            egui::Rect::from_min_max(rect.min, egui::pos2(rect.min.x + 5.0, rect.max.y));
        ui.painter().rect_filled(
            indicator,
            egui::Rounding::same(decorations.card_rounding),
            self.status_color,
        );

        // 내용 영역 UI
        let content_rect = rect.shrink2(egui::vec2(
            decorations.card_inner_margin.left,
            decorations.card_inner_margin.top,
        ));

        let mut content_ui = ui.child_ui(
            content_rect,
            egui::Layout::left_to_right(egui::Align::Center),
        );

        content_ui.spacing_mut().item_spacing.x = 14.0;

        if !self.status_icon.is_empty() {
            content_ui.label(
                RichText::new(self.status_icon)
                    .size(26.0)
                    .color(self.status_color),
            );
        }

        content_ui.vertical(|ui| {
            ui.label(
                RichText::new(self.name)
                    .size(17.0)
                    .color(palette.fg_text_primary)
                    .strong(),
            );
            ui.label(
                RichText::new(format!("ID: {}", self.step_id)).color(palette.fg_text_secondary),
            );
        });

        content_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if !self.status_text.is_empty() {
                ui.label(
                    RichText::new(self.status_text)
                        .size(15.0)
                        .color(self.status_color)
                        .strong(),
                );
            }
        });

        let id = ui.id().with(self.step_id); // 또는 with("step_card")
        let mut response = ui.interact(rect, id, egui::Sense::click());
        response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

        response
    }
}

/// 단색 배경과 일정한 간격을 제공하는 기본 버튼 위젯.
pub(super) struct PrimaryButton<'a> {
    theme: &'a Theme,
    label: &'a str,
    icon: &'a str,
}

impl<'a> PrimaryButton<'a> {
    /// PrimaryButton 기본 인스턴스를 생성하며 라벨 텍스트를 지정한다.
    pub(super) fn new(theme: &'a Theme, label: &'a str) -> Self {
        Self {
            theme,
            label,
            icon: "",
        }
    }

    /// 버튼 좌측에 표시할 아이콘 텍스트를 정의한다.
    pub(super) fn icon(mut self, icon: &'a str) -> Self {
        self.icon = icon;
        self
    }
}

impl<'a> Widget for PrimaryButton<'a> {
    /// egui Widget 인터페이스를 구현해 기본 버튼을 렌더링한다.
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let decorations = self.theme.decorations();
        let palette = self.theme.palette();

        let enabled = ui.is_enabled();

        // 아이콘 + 텍스트 결합
        let text = if self.icon.is_empty() {
            self.label.to_string()
        } else {
            format!("{}  {}", self.icon, self.label)
        };

        // 여기서 폰트 크기/굵기를 마음대로 설정
        let font_size = 16.0; // 원하는 크기
        let rich = egui::RichText::new(text)
            .size(font_size) // 글자 크기
            .strong() // 굵게
            .color(if enabled {
                egui::Color32::WHITE
            } else {
                blend_color(palette.fg_text_secondary, palette.bg_panel, 0.4)
            });

        // 버튼 높이를 자동으로 글자 높이에 맞추기
        let text_height = ui.ctx().fonts(|f| {
            f.row_height(&egui::FontId::new(
                font_size,
                egui::FontFamily::Proportional,
            ))
        });
        let button_height = decorations.button_height.max(text_height + 6.0);

        let mut button = egui::Button::new(rich)
            .min_size(egui::vec2(
                decorations.button_min_width,
                button_height, // 글자 크기에 따라 자동 증가
            ))
            .rounding(egui::Rounding::same(decorations.button_rounding));

        // 배경색
        let base_fill = if enabled {
            palette.accent_primary
        } else {
            blend_color(palette.accent_primary, palette.border_soft, 0.5)
        };
        button = button.fill(base_fill);

        // 실제 버튼 추가
        let response = ui.add(button);

        // Hover 시 마우스 커서 모양 변경
        if enabled && response.hovered() {
            ui.output_mut(|o| {
                o.cursor_icon = egui::CursorIcon::PointingHand;
            });
        }

        response
    }
}

/// 단색 헤더를 그려 정보 영역의 시각적 위계를 만든다.
pub(super) fn solid_section_header(ui: &mut egui::Ui, theme: &Theme, icon: &str, title: &str) {
    let decorations = theme.decorations();
    let palette = theme.palette();
    let size = egui::vec2(ui.available_width(), decorations.header_height);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().rect_filled(
        rect,
        egui::Rounding::same(decorations.header_rounding),
        decorations.header_fill,
    );
    ui.painter().rect_stroke(
        rect,
        egui::Rounding::same(decorations.header_rounding),
        egui::Stroke::new(
            1.0,
            blend_color(decorations.header_fill, palette.bg_panel, 0.4),
        ),
    );
    let content_rect = rect.shrink2(egui::vec2(16.0, 0.0));
    ui.allocate_ui_at_rect(content_rect, |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            if !icon.is_empty() {
                ui.label(
                    RichText::new(icon)
                        .size(decorations.header_icon_size)
                        .color(decorations.header_text),
                );
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new(title)
                    .size(18.0)
                    .color(decorations.header_text)
                    .strong(),
            );
        });
    });
}
