use crate::theme::{Theme, blend_color};
use eframe::egui::{self, RichText, Widget};

/// 단색 배경과 일정한 간격을 제공하는 기본 버튼 위젯.
pub(super) struct PrimaryButton<'a> {
    theme: &'a Theme,
    label: &'a str,
    icon:  &'a str,
}

impl<'a> PrimaryButton<'a> {
    pub(super) fn new(theme: &'a Theme, label: &'a str) -> Self {
        Self {
            theme,
            label,
            icon: "",
        }
    }

    pub(super) fn icon(mut self, icon: &'a str) -> Self {
        self.icon = icon;
        self
    }
}

impl<'a> Widget for PrimaryButton<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let decorations = self.theme.decorations();
        let palette     = self.theme.palette();

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
            .size(font_size)     // 글자 크기
            .strong()            // 굵게
            .color(if enabled {
                egui::Color32::WHITE
            } else {
                blend_color(palette.fg_text_secondary, palette.bg_panel, 0.4)
            });

        // 버튼 높이를 자동으로 글자 높이에 맞추기
        let text_height = ui.ctx().fonts(|f| {
            f.row_height(&egui::FontId::new(font_size, egui::FontFamily::Proportional))
        });
        let button_height = decorations.button_height.max(text_height + 6.0);

        let mut button = egui::Button::new(rich)
            .min_size(egui::vec2(
                decorations.button_min_width,
                button_height,       // 글자 크기에 따라 자동 증가
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
