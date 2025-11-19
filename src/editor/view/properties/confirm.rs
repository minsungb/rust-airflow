use super::super::*;
use super::*;

/// 컨펌 설정 UI를 그린다.
pub(super) fn render_confirm_section(
    ui: &mut egui::Ui,
    confirm: &mut Option<crate::scenario::StepConfirmConfig>,
    mark_dirty: &mut bool,
) {
    egui::CollapsingHeader::new("실행 컨펌")
        .default_open(false)
        .show(ui, |ui| {
            let cfg = confirm.get_or_insert_with(|| crate::scenario::StepConfirmConfig {
                before: false,
                after: false,
                message_before: None,
                message_after: None,
                default_answer: ConfirmDefault::Yes,
            });
            if ui.checkbox(&mut cfg.before, "실행 전 확인").changed() {
                *mark_dirty = true;
            }
            if ui.checkbox(&mut cfg.after, "실행 후 확인").changed() {
                *mark_dirty = true;
            }
            ui.label("메시지 (실행 전)");
            let mut before_msg = cfg.message_before.clone().unwrap_or_default();
            if ui.text_edit_singleline(&mut before_msg).changed() {
                cfg.message_before = if before_msg.trim().is_empty() {
                    None
                } else {
                    Some(before_msg)
                };
                *mark_dirty = true;
            }
            ui.label("메시지 (실행 후)");
            let mut after_msg = cfg.message_after.clone().unwrap_or_default();
            if ui.text_edit_singleline(&mut after_msg).changed() {
                cfg.message_after = if after_msg.trim().is_empty() {
                    None
                } else {
                    Some(after_msg)
                };
                *mark_dirty = true;
            }
            egui::ComboBox::from_label("기본 응답")
                .selected_text(match cfg.default_answer {
                    ConfirmDefault::Yes => "예",
                    ConfirmDefault::No => "아니오",
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(matches!(cfg.default_answer, ConfirmDefault::Yes), "예")
                        .clicked()
                    {
                        cfg.default_answer = ConfirmDefault::Yes;
                        *mark_dirty = true;
                    }
                    if ui
                        .selectable_label(
                            matches!(cfg.default_answer, ConfirmDefault::No),
                            "아니오",
                        )
                        .clicked()
                    {
                        cfg.default_answer = ConfirmDefault::No;
                        *mark_dirty = true;
                    }
                });
        });
    if let Some(cfg) = confirm {
        let empty_before = cfg
            .message_before
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true);
        let empty_after = cfg
            .message_after
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true);
        if !cfg.before && !cfg.after && empty_before && empty_after {
            *confirm = None;
        }
    }
}
