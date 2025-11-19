use super::super::*;
use super::*;

/// DB 연결 목록을 편집할 수 있는 섹션을 렌더링한다.
pub(super) fn render_db_section(
    ui: &mut egui::Ui,
    state: &mut ScenarioEditorState,
    mark_dirty: &mut bool,
    palette: ThemePalette,
    decorations: ThemeDecorations,
) {
    ui.heading("🗄 DB 설정");
    ui.label("SQL/SQL 파일 Step에서 사용할 DB 접속 정보를 정의합니다.");
    if !state.has_default_db() {
        ui.colored_label(
            palette.accent_warning,
            "default 키가 없으면 target_db 미지정 Step이 실패합니다.",
        );
    }
    if state.db_connections.is_empty() {
        ui.label("등록된 DB 연결이 없습니다. 'DB 연결 추가' 버튼으로 새 항목을 만드세요.");
    }
    let mut remove_idx: Option<usize> = None;
    for (idx, conn) in state.db_connections.iter_mut().enumerate() {
        ui.add_space(6.0);
        ui.push_id(idx, |ui| {
            egui::Frame::none()
                .fill(palette.bg_panel)
                .stroke(egui::Stroke::new(1.0, palette.border_soft))
                .rounding(egui::Rounding::same(decorations.card_rounding))
                .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("키");
                        if ui.text_edit_singleline(&mut conn.key).changed() {
                            *mark_dirty = true;
                        }
                        if ui.button("삭제").clicked() {
                            remove_idx = Some(idx);
                        }
                    });
                    if conn.key.trim() == "default" {
                        ui.small("default는 target_db 미지정 시 사용됩니다.");
                    }
                    egui::ComboBox::from_label("종류")
                        .selected_text(match conn.kind {
                            DbKind::Oracle => "Oracle",
                            DbKind::Postgres => "PostgreSQL",
                            DbKind::Dummy => "(미지원)",
                        })
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(matches!(conn.kind, DbKind::Oracle), "Oracle")
                                .clicked()
                            {
                                conn.kind = DbKind::Oracle;
                                *mark_dirty = true;
                            }
                            if ui
                                .selectable_label(
                                    matches!(conn.kind, DbKind::Postgres),
                                    "PostgreSQL",
                                )
                                .clicked()
                            {
                                conn.kind = DbKind::Postgres;
                                *mark_dirty = true;
                            }
                        });
                    ui.label("DSN / 접속 문자열");
                    if ui.text_edit_singleline(&mut conn.dsn).changed() {
                        *mark_dirty = true;
                    }
                    ui.label("사용자");
                    if ui.text_edit_singleline(&mut conn.user).changed() {
                        *mark_dirty = true;
                    }
                    ui.label("비밀번호");
                    if ui.text_edit_singleline(&mut conn.password).changed() {
                        *mark_dirty = true;
                    }
                });
        });
    }
    if let Some(idx) = remove_idx {
        state.db_connections.remove(idx);
        *mark_dirty = true;
    }
    if ui.button("DB 연결 추가").clicked() {
        let new_key = state.generate_db_key();
        state
            .db_connections
            .push(DbConnectionEditor::new(new_key, DbKind::Oracle));
        *mark_dirty = true;
    }
    ui.add_space(8.0);
}
