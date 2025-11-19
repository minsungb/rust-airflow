use super::*;

mod confirm;
mod db;
mod loop_panel;
mod step_config;

impl<'a> ScenarioBuilderUi<'a> {
    /// 우측 속성 패널을 렌더링한다.
    pub(super) fn render_properties(&mut self, ui: &mut egui::Ui) {
        let mut mark_dirty = false;
        let palette = *self.get_theme().palette();
        let decorations = *self.get_theme().decorations();
        let state = self.get_state_mut();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(320.0);
                let mut selected_runtime_id: Option<String> = None;
                db::render_db_section(ui, state, &mut mark_dirty, palette, decorations);
                ui.separator();
                ui.heading("⚙️ Step 속성");
                let db_keys = state.db_key_list();

                if let Some(selected_id) = state.selected_node_id.clone() {
                    if let Some(selected) = state.node_mut(&selected_id) {
                        selected_runtime_id = Some(selected.id.clone());

                        let mut id_buf = selected.id.clone();
                        ui.label("ID");
                        if ui.text_edit_singleline(&mut id_buf).changed() {
                            selected.id = id_buf.clone();
                            selected_runtime_id = Some(id_buf);
                            mark_dirty = true;
                        }

                        let mut name_buf = selected.name.clone();
                        ui.label("이름");
                        if ui.text_edit_singleline(&mut name_buf).changed() {
                            selected.name = name_buf;
                            mark_dirty = true;
                        }

                        ui.label(format!("유형: {:?}", selected.kind));

                        if ui
                            .checkbox(&mut selected.allow_parallel, "병렬 허용")
                            .changed()
                        {
                            mark_dirty = true;
                        }

                        let mut retry = selected.retry;
                        if ui
                            .add(egui::Slider::new(&mut retry, 0..=5).text("재시도"))
                            .changed()
                        {
                            selected.retry = retry;
                            mark_dirty = true;
                        }

                        let mut timeout = selected.timeout_sec as i32;
                        if ui
                            .add(
                                egui::DragValue::new(&mut timeout)
                                    .prefix("타임아웃 ")
                                    .suffix("초"),
                            )
                            .changed()
                        {
                            selected.timeout_sec = timeout.max(1) as u64;
                            mark_dirty = true;
                        }

                        ui.separator();

                        step_config::render_step_config_ui(
                            ui,
                            &mut selected.config,
                            &mut mark_dirty,
                            &db_keys,
                            selected.id.as_str(),
                        );
                        confirm::render_confirm_section(ui, &mut selected.confirm, &mut mark_dirty);
                        if selected.kind == StepKind::Loop {
                            loop_panel::render_loop_section(
                                ui,
                                selected,
                                &mut mark_dirty,
                                palette,
                                decorations,
                                &db_keys,
                            );
                        }
                    } else {
                        ui.label("선택된 Step 정보를 찾을 수 없습니다.");
                    }
                } else {
                    ui.label("선택된 Step이 없습니다.");
                }

                if let Some(selected_id) = selected_runtime_id.clone() {
                    ui.separator();
                    ui.label("의존성");

                    if !state.nodes.is_empty() {
                        egui::ScrollArea::vertical()
                            .max_height(120.0)
                            .show(ui, |ui| {
                                let deps = state.dependencies_of(&selected_id);
                                for dep in deps {
                                    let dep_id = dep.clone();
                                    ui.horizontal(|ui| {
                                        ui.label(&dep_id);
                                        if ui.button("삭제").clicked() {
                                            state.remove_connection(&dep_id, &selected_id);
                                            mark_dirty = true;
                                        }
                                    });
                                }
                            });
                    }

                    ui.add_space(6.0);

                    let mut options: Vec<String> = state
                        .nodes
                        .iter()
                        .filter(|node| node.id != selected_id)
                        .map(|node| node.id.clone())
                        .collect();
                    options.sort();

                    egui::ComboBox::from_label("의존성 추가")
                        .selected_text("노드 선택")
                        .show_ui(ui, |ui| {
                            for option in &options {
                                if ui.selectable_label(false, option).clicked() {
                                    state.add_connection(option, &selected_id);
                                    mark_dirty = true;
                                }
                            }
                        });

                    ui.separator();
                    if ui.button("이 Step 삭제").clicked() {
                        state.remove_node(&selected_id);
                        mark_dirty = true;
                    }
                }
            });
        state.dirty = mark_dirty;
    }
}
