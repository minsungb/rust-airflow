use super::super::*;
use super::*;

/// Loop 전용 설정 섹션을 그려 Step 설정과 겹치지 않도록 배치한다.
pub(super) fn render_loop_section(
    ui: &mut egui::Ui,
    node: &mut EditorStepNode,
    mark_dirty: &mut bool,
    palette: ThemePalette,
    decorations: ThemeDecorations,
    db_keys: &[String],
) {
    let EditorStepConfig::Loop { config } = &mut node.config else {
        return;
    };
    ui.add_space(12.0);
    egui::Frame::none()
        .fill(palette.bg_panel)
        .stroke(egui::Stroke::new(1.0, palette.border_soft))
        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.heading("Loop 설정");
            ui.add_space(6.0);
            ui.label("for_each_glob");
            if ui.text_edit_singleline(&mut config.for_each_glob).changed() {
                *mark_dirty = true;
            }
            ui.label("as 변수명");
            if ui.text_edit_singleline(&mut config.as_var).changed() {
                *mark_dirty = true;
            }
            egui::ComboBox::from_label("실패 시 동작")
                .selected_text(match config.on_iteration_failure {
                    LoopIterationFailure::StopAll => "Stop All",
                    LoopIterationFailure::Continue => "Continue",
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            matches!(config.on_iteration_failure, LoopIterationFailure::StopAll),
                            "Stop All",
                        )
                        .clicked()
                    {
                        config.on_iteration_failure = LoopIterationFailure::StopAll;
                        *mark_dirty = true;
                    }
                    if ui
                        .selectable_label(
                            matches!(config.on_iteration_failure, LoopIterationFailure::Continue),
                            "Continue",
                        )
                        .clicked()
                    {
                        config.on_iteration_failure = LoopIterationFailure::Continue;
                        *mark_dirty = true;
                    }
                });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("하위 Step");
                ui.menu_button("추가", |ui| {
                    for (label, kind) in [
                        ("SQL", StepKind::Sql),
                        ("SQL 파일", StepKind::SqlFile),
                        ("SQL*Loader", StepKind::SqlLoaderPar),
                        ("Shell", StepKind::Shell),
                        ("Extract", StepKind::Extract),
                        ("Loop", StepKind::Loop),
                    ] {
                        if ui.button(label).clicked() {
                            let new_id = config.generate_child_id();
                            let mut child = EditorStepNode::new(
                                new_id.clone(),
                                format!("Loop Step {new_id}"),
                                kind,
                            );
                            child.position = egui::pos2(20.0, 20.0);
                            config.nodes.push(child);
                            config.selected_node_id = Some(new_id);
                            *mark_dirty = true;
                            ui.close_menu();
                        }
                    }
                });
                if let Some(selected_id) = config.selected_node_id.clone() {
                    if ui.button("선택 Step 삭제").clicked() {
                        config.remove_node(&selected_id);
                        *mark_dirty = true;
                    }
                }
            });
            for child in &config.nodes {
                let selected = config.selected_node_id.as_deref() == Some(child.id.as_str());
                if ui
                    .selectable_label(selected, format!("{} ({:?})", child.name, child.kind))
                    .clicked()
                {
                    config.selected_node_id = Some(child.id.clone());
                }
            }
            if let Some(selected_id) = config.selected_node_id.clone() {
                let deps = config.dependencies_of(&selected_id);
                let mut options: Vec<String> = config
                    .nodes
                    .iter()
                    .filter(|n| n.id != selected_id)
                    .map(|n| n.id.clone())
                    .collect();
                options.sort();

                let mut deps_to_remove: Vec<String> = Vec::new();
                let mut deps_to_add: Vec<String> = Vec::new();

                if let Some(child) = config.node_mut(&selected_id) {
                    ui.separator();
                    ui.heading("선택된 하위 Step");
                    ui.label(format!("ID: {}", child.id));

                    let mut name_buf = child.name.clone();
                    if ui.text_edit_singleline(&mut name_buf).changed() {
                        child.name = name_buf;
                        *mark_dirty = true;
                    }

                    if ui
                        .checkbox(&mut child.allow_parallel, "병렬 허용")
                        .changed()
                    {
                        *mark_dirty = true;
                    }

                    let mut retry = child.retry;
                    if ui
                        .add(egui::Slider::new(&mut retry, 0..=5).text("재시도"))
                        .changed()
                    {
                        child.retry = retry;
                        *mark_dirty = true;
                    }

                    let mut timeout = child.timeout_sec as i32;
                    if ui
                        .add(
                            egui::DragValue::new(&mut timeout)
                                .prefix("타임아웃 ")
                                .suffix("초"),
                        )
                        .changed()
                    {
                        child.timeout_sec = timeout.max(1) as u64;
                        *mark_dirty = true;
                    }

                    ui.separator();
                    super::step_config::render_step_config_ui(
                        ui,
                        &mut child.config,
                        mark_dirty,
                        db_keys,
                        child.id.as_str(),
                    );
                    super::confirm::render_confirm_section(ui, &mut child.confirm, mark_dirty);

                    egui::ScrollArea::vertical()
                        .max_height(120.0)
                        .show(ui, |ui| {
                            for dep in &deps {
                                let dep_id = dep.clone();
                                ui.horizontal(|ui| {
                                    ui.label(&dep_id);
                                    if ui.button("삭제").clicked() {
                                        deps_to_remove.push(dep_id.clone());
                                        *mark_dirty = true;
                                    }
                                });
                            }
                        });

                    egui::ComboBox::from_label("의존성 추가")
                        .selected_text("노드 선택")
                        .show_ui(ui, |ui| {
                            for option in &options {
                                if ui.selectable_label(false, option).clicked() {
                                    deps_to_add.push(option.clone());
                                    *mark_dirty = true;
                                }
                            }
                        });
                } else {
                    config.selected_node_id = None;
                }

                for dep_id in deps_to_remove {
                    config.remove_connection(&dep_id, &selected_id);
                }
                for option in deps_to_add {
                    config.add_connection(&option, &selected_id);
                }
            }
        });
}
