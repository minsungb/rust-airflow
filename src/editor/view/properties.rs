use super::*;

impl<'a> ScenarioBuilderUi<'a> {
    /// DB 연결 목록을 편집할 수 있는 섹션을 렌더링한다.
    fn render_db_section(
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
                Self::render_db_section(ui, state, &mut mark_dirty, palette, decorations);
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

                        Self::render_step_config_ui(
                            ui,
                            &mut selected.config,
                            &mut mark_dirty,
                            &db_keys,
                            selected.id.as_str(),
                        );
                        Self::render_confirm_section(ui, &mut selected.confirm, &mut mark_dirty);
                        if selected.kind == StepKind::Loop {
                            Self::render_loop_section(
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

    /// Step 구성 UI를 노출한다.
    fn render_step_config_ui(
        ui: &mut egui::Ui,
        config: &mut EditorStepConfig,
        mark_dirty: &mut bool,
        db_keys: &[String],
        id_hint: &str,
    ) {
        match config {
            EditorStepConfig::Sql { sql, target_db } => {
                Self::render_target_db_picker(ui, target_db, db_keys, mark_dirty, id_hint);
                ui.label("SQL");
                if ui.text_edit_multiline(sql).changed() {
                    *mark_dirty = true;
                }
            }
            EditorStepConfig::SqlFile { path, target_db } => {
                Self::render_target_db_picker(ui, target_db, db_keys, mark_dirty, id_hint);
                ui.label("SQL 파일 경로");
                let mut path_buf = path.display().to_string();
                if ui.text_edit_singleline(&mut path_buf).changed() {
                    *path = std::path::PathBuf::from(path_buf);
                    *mark_dirty = true;
                }
            }
            EditorStepConfig::SqlLoaderPar { config } => {
                Self::render_sqlldr(ui, config, mark_dirty);
            }
            EditorStepConfig::Shell { config } => {
                Self::render_shell(ui, config, mark_dirty);
            }
            EditorStepConfig::Extract { config } => {
                Self::render_extract(ui, config, mark_dirty);
            }
            EditorStepConfig::Loop { .. } => {}
        }
    }

    /// target_db를 선택할 수 있는 공용 콤보박스를 렌더링한다.
    fn render_target_db_picker(
        ui: &mut egui::Ui,
        target_db: &mut Option<String>,
        db_keys: &[String],
        mark_dirty: &mut bool,
        id_hint: &str,
    ) {
        ui.label("DB 타겟(target_db)");
        let selected_text = target_db
            .as_deref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "(기본 DB 사용)".to_string());
        ui.push_id(format!("target_db_{id_hint}"), |ui| {
            egui::ComboBox::from_id_source("target_db_combo")
                .selected_text(selected_text)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(target_db.is_none(), "(기본 DB 사용)")
                        .clicked()
                    {
                        if target_db.is_some() {
                            *target_db = None;
                            *mark_dirty = true;
                        }
                    }
                    for key in db_keys {
                        let selected = target_db.as_deref() == Some(key.as_str());
                        if ui.selectable_label(selected, key).clicked() && !selected {
                            *target_db = Some(key.clone());
                            *mark_dirty = true;
                        }
                    }
                });
        });
    }

    /// 컨펌 설정 UI를 그린다.
    fn render_confirm_section(
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
                            .selectable_label(
                                matches!(cfg.default_answer, ConfirmDefault::Yes),
                                "예",
                            )
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

    /// Loop 전용 설정 섹션을 그려 Step 설정과 겹치지 않도록 배치한다.
    fn render_loop_section(
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
                                matches!(
                                    config.on_iteration_failure,
                                    LoopIterationFailure::StopAll
                                ),
                                "Stop All",
                            )
                            .clicked()
                        {
                            config.on_iteration_failure = LoopIterationFailure::StopAll;
                            *mark_dirty = true;
                        }
                        if ui
                            .selectable_label(
                                matches!(
                                    config.on_iteration_failure,
                                    LoopIterationFailure::Continue
                                ),
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
                        Self::render_step_config_ui(
                            ui,
                            &mut child.config,
                            mark_dirty,
                            db_keys,
                            child.id.as_str(),
                        );
                        Self::render_confirm_section(ui, &mut child.confirm, mark_dirty);

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

    /// SQL*Loader 속성 UI를 렌더링한다.
    fn render_sqlldr(
        ui: &mut egui::Ui,
        config: &mut crate::scenario::SqlLoaderParConfig,
        mark_dirty: &mut bool,
    ) {
        let mut control = config.control_file.display().to_string();
        ui.label("control 파일");
        if ui.text_edit_singleline(&mut control).changed() {
            config.control_file = control.into();
            *mark_dirty = true;
        }

        Self::optional_path_field_ui(ui, "data 파일", &mut config.data_file, mark_dirty);
        Self::optional_path_field_ui(ui, "log 파일", &mut config.log_file, mark_dirty);
        Self::optional_path_field_ui(ui, "bad 파일", &mut config.bad_file, mark_dirty);
        Self::optional_path_field_ui(ui, "discard 파일", &mut config.discard_file, mark_dirty);

        let mut conn = config.conn.clone().unwrap_or_default();
        ui.label("접속 문자열");
        if ui.text_edit_singleline(&mut conn).changed() {
            config.conn = if conn.is_empty() { None } else { Some(conn) };
            *mark_dirty = true;
        }
        ui.small("비워두면 SQLLDR_CONN 환경 변수를 사용합니다.");
    }

    /// 선택적 경로 필드를 렌더링한다.
    fn optional_path_field_ui(
        ui: &mut egui::Ui,
        label: &str,
        path: &mut Option<std::path::PathBuf>,
        mark_dirty: &mut bool,
    ) {
        ui.label(label);
        let mut buf = path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        if ui.text_edit_singleline(&mut buf).changed() {
            let trimmed = buf.trim();
            if trimmed.is_empty() {
                *path = None;
            } else {
                *path = Some(std::path::PathBuf::from(trimmed));
            }
            *mark_dirty = true;
        }
    }

    /// Shell 속성 UI를 렌더링한다.
    fn render_shell(
        ui: &mut egui::Ui,
        config: &mut crate::scenario::ShellConfig,
        mark_dirty: &mut bool,
    ) {
        ui.label("스크립트");
        if ui.text_edit_multiline(&mut config.script).changed() {
            *mark_dirty = true;
        }

        let mut program = config.shell_program.clone().unwrap_or_default();
        ui.label("셸 프로그램");
        if ui.text_edit_singleline(&mut program).changed() {
            config.shell_program = if program.is_empty() {
                None
            } else {
                Some(program)
            };
            *mark_dirty = true;
        }

        let mut args = config.shell_args.join(", ");
        ui.label("인자 목록(쉼표 구분)");
        if ui.text_edit_singleline(&mut args).changed() {
            config.shell_args = args
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            *mark_dirty = true;
        }

        let mut work_dir = config
            .working_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        ui.label("작업 디렉터리");
        if ui.text_edit_singleline(&mut work_dir).changed() {
            config.working_dir = if work_dir.is_empty() {
                None
            } else {
                Some(work_dir.into())
            };
            *mark_dirty = true;
        }

        let mut run_as = config.run_as.clone().unwrap_or_default();
        ui.label("실행 사용자");
        if ui.text_edit_singleline(&mut run_as).changed() {
            config.run_as = if run_as.is_empty() {
                None
            } else {
                Some(run_as)
            };
            *mark_dirty = true;
        }

        ui.label("환경 변수 (KEY=VALUE 한 줄씩)");
        let mut env_text = config
            .env
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("\n");
        if ui.text_edit_multiline(&mut env_text).changed() {
            config.env = Self::parse_env(&env_text);
            *mark_dirty = true;
        }
    }

    /// Extract Step 속성 UI를 렌더링한다.
    fn render_extract(
        ui: &mut egui::Ui,
        config: &mut ExtractVarFromFileConfig,
        mark_dirty: &mut bool,
    ) {
        ui.label("파일 경로");
        if ui.text_edit_singleline(&mut config.file_path).changed() {
            *mark_dirty = true;
        }
        let mut line = config.line as i32;
        if ui
            .add(egui::DragValue::new(&mut line).prefix("라인 "))
            .changed()
        {
            config.line = line.max(1) as usize;
            *mark_dirty = true;
        }
        ui.label("정규식 패턴");
        if ui.text_edit_singleline(&mut config.pattern).changed() {
            *mark_dirty = true;
        }
        let mut group = config.group as i32;
        if ui
            .add(egui::DragValue::new(&mut group).prefix("캡처 그룹 "))
            .changed()
        {
            config.group = group.max(0) as usize;
            *mark_dirty = true;
        }
        ui.label("저장할 변수명");
        if ui.text_edit_singleline(&mut config.var_name).changed() {
            *mark_dirty = true;
        }
    }

    /// Shell env 문자열을 파싱한다.
    fn parse_env(text: &str) -> HashMap<String, String> {
        text.lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return None;
                }
                trimmed
                    .split_once('=')
                    .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
            })
            .collect()
    }
}
