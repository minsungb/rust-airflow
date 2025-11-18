use crate::theme::{Theme, blend_color};
use eframe::egui::{self, RichText, Widget};

/// 단색 배경과 일정한 간격을 제공하는 기본 버튼 위젯.
pub(super) struct PrimaryButton<'a> {
    theme: &'a Theme,
    label: &'a str,
    icon: &'a str,
}

impl<'a> PrimaryButton<'a> {
    /// 버튼의 기본 정보를 생성한다.
    pub(super) fn new(theme: &'a Theme, label: &'a str) -> Self {
        Self {
            theme,
            label,
            icon: "",
        }
    }

    /// 버튼에 표시할 아이콘(이모지)을 설정한다.
    pub(super) fn icon(mut self, icon: &'a str) -> Self {
        self.icon = icon;
        self
    }
}

impl<'a> Widget for PrimaryButton<'a> {
    /// egui 위젯 트레이트를 구현하여 버튼을 화면에 그린다.
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let decorations = self.theme.decorations();
        let palette = self.theme.palette();
        let enabled = ui.is_enabled();
        let button_padding = ui.style().spacing.button_padding.x;
        let galley = ui.painter().layout_no_wrap(
            self.label.to_string(),
            egui::TextStyle::Button.resolve(ui.style()),
            palette.fg_text_primary,
        );
        let icon_space = if self.icon.is_empty() { 0.0 } else { 28.0 };
        let desired_width = galley.size().x
            + icon_space
            + button_padding * 2.0
            + decorations.button_min_width * 0.1;
        let size = egui::vec2(
            desired_width.max(decorations.button_min_width),
            decorations.button_height,
        );
        let (rect, mut response) = ui.allocate_exact_size(size, egui::Sense::click());
        let mut fill = palette.accent_primary;
        if !enabled {
            fill = blend_color(fill, palette.border_soft, 0.5);
        } else if response.is_pointer_button_down_on() {
            fill = blend_color(fill, palette.fg_text_primary, 0.2);
        } else if response.hovered() {
            fill = blend_color(fill, palette.bg_panel, 0.2);
        }
        ui.painter().rect_filled(
            rect,
            egui::Rounding::same(decorations.button_rounding),
            fill,
        );
        ui.painter().rect_stroke(
            rect,
            egui::Rounding::same(decorations.button_rounding),
            egui::Stroke::new(1.0, blend_color(fill, palette.border_soft, 0.6)),
        );
        let text_color = if enabled {
            egui::Color32::WHITE
        } else {
            blend_color(palette.fg_text_secondary, palette.bg_panel, 0.4)
        };
        let content_rect = rect.shrink2(egui::vec2(button_padding, 0.0));
        let label_response = ui.interact(
            content_rect,
            ui.id().with((self.label, "primary_button")),
            egui::Sense::click(),
        );
        response |= label_response;
        ui.allocate_ui_at_rect(content_rect, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if !self.icon.is_empty() {
                    ui.label(RichText::new(self.icon).size(18.0).color(text_color));
                }
                ui.label(
                    RichText::new(self.label)
                        .size(16.0)
                        .color(text_color)
                        .strong(),
                );
            });
        });
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
