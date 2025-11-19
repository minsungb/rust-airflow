use super::model::{EditorStepConfig, EditorStepNode, ScenarioEditorState, StepKind};
use crate::scenario::{ConfirmDefault, ExtractVarFromFileConfig, LoopIterationFailure};
use crate::theme::{BuilderColors, StepVisualKind, Theme, ThemeDecorations, ThemePalette};
use eframe::egui;
use eframe::epaint::{CubicBezierShape, Stroke};
use std::collections::HashMap;

/// Scenario Builder 화면 전체를 담당하는 뷰이다.
pub struct ScenarioBuilderUi<'a> {
    /// 테마 참조.
    theme: &'a Theme,
    /// 에디터 상태 참조.
    state: &'a mut ScenarioEditorState,
}

impl<'a> ScenarioBuilderUi<'a> {
    /// 뷰 인스턴스를 생성한다.
    pub fn new(theme: &'a Theme, state: &'a mut ScenarioEditorState) -> Self {
        Self { theme, state }
    }

    /// 좌/중앙/우 패널을 구성한다.
    pub fn show(&mut self, ctx: &egui::Context) {
        let palette = *self.theme.palette();
        let decorations = *self.theme.decorations();
        let builder_colors = self.theme.builder_colors();
        let palette_frame = egui::Frame {
            fill: palette.bg_sidebar,
            stroke: egui::Stroke::new(1.0, palette.border_soft),
            rounding: egui::Rounding::same(decorations.container_rounding),
            inner_margin: decorations.card_inner_margin,
            ..Default::default()
        };
        egui::SidePanel::left("builder_palette")
            .frame(palette_frame)
            .resizable(false)
            .default_width(220.0)
            .show(ctx, |ui| {
                self.render_palette(ui);
            });
        let property_frame = egui::Frame {
            fill: palette.bg_sidebar,
            stroke: egui::Stroke::new(1.0, palette.border_soft),
            rounding: egui::Rounding::same(decorations.container_rounding),
            inner_margin: decorations.card_inner_margin,
            ..Default::default()
        };
        egui::SidePanel::right("builder_properties")
            .frame(property_frame)
            .resizable(false)
            .default_width(320.0)
            .show(ctx, |ui| {
                self.render_properties(ui);
            });
        let canvas_frame = egui::Frame {
            fill: builder_colors.canvas_fill,
            stroke: egui::Stroke::new(1.0, palette.border_soft),
            rounding: egui::Rounding::same(decorations.container_rounding),
            inner_margin: egui::Margin::same(12.0),
            ..Default::default()
        };
        egui::CentralPanel::default()
            .frame(canvas_frame)
            .show(ctx, |ui| {
                self.render_canvas(ui, builder_colors);
            });
    }

    /// Step 팔레트를 렌더링한다.
    fn render_palette(&mut self, ui: &mut egui::Ui) {
        ui.heading("🧱 Step 팔레트");
        ui.separator();
        ui.label("추가할 Step 유형을 선택하세요.");
        ui.add_space(10.0);
        for (label, kind) in [
            ("SQL", StepKind::Sql),
            ("SQL 파일", StepKind::SqlFile),
            ("SQL*Loader", StepKind::SqlLoaderPar),
            ("Shell", StepKind::Shell),
            ("Extract (값 추출)", StepKind::Extract),
            ("Loop (반복)", StepKind::Loop),
        ] {
            if ui.button(label).clicked() {
                self.state.add_node(kind);
            }
        }
    }

    /// 우측 속성 패널을 렌더링한다.
    fn render_properties(&mut self, ui: &mut egui::Ui) {
        let mut mark_dirty = false;

        ui.heading("⚙️ Step 속성");
        ui.separator();

        egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(320.0);
            // 이 렌더 사이클에서 최종적으로 사용될 선택된 Step의 ID를 저장할 변수
            let mut selected_runtime_id: Option<String> = None;

            if let Some(selected_id) = self.state.selected_node_id.clone() {
                if let Some(selected) = self.state.node_mut(&selected_id) {
                    // 현재 선택된 노드의 id를 runtime 변수에 저장
                    selected_runtime_id = Some(selected.id.clone());

                    // ---- 여기부터: 선택된 노드의 속성 편집 ----
                    let mut id_buf = selected.id.clone();
                    ui.label("ID");
                    if ui.text_edit_singleline(&mut id_buf).changed() {
                        selected.id = id_buf.clone();
                        selected_runtime_id = Some(id_buf); // id가 바뀌면 runtime id도 갱신
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

                    Self::render_step_config_ui(ui, &mut selected.config, &mut mark_dirty);
                    Self::render_confirm_section(ui, &mut selected.confirm, &mut mark_dirty);
                    if selected.kind == StepKind::Loop {
                        let palette = *self.theme.palette();
                        let decorations = *self.theme.decorations();
                        Self::render_loop_section(
                            ui,
                            selected,
                            &mut mark_dirty,
                            palette,
                            decorations,
                        );
                    }
                    // ---- 여기까지 selected에 대한 편집만 수행 (self.state 다른 메서드 호출 X) ----
                } else {
                    ui.label("선택된 Step 정보를 찾을 수 없습니다.");
                }
            } else {
                ui.label("선택된 Step이 없습니다.");
            }

            // ---------- 여기부터: 의존성 / 삭제 UI (self.state 를 마음대로 써도 됨) ----------
            if let Some(selected_id) = selected_runtime_id.clone() {
                ui.separator();
                ui.label("의존성");

                if !self.state.nodes.is_empty() {
                    // 의존성 목록 표시
                    egui::ScrollArea::vertical()
                        .max_height(120.0)
                        .show(ui, |ui| {
                            let deps = self.state.dependencies_of(&selected_id);
                            for dep in deps {
                                let dep_id = dep.clone();
                                ui.horizontal(|ui| {
                                    ui.label(&dep_id);
                                    if ui.button("삭제").clicked() {
                                        self.state.remove_connection(&dep_id, &selected_id);
                                        mark_dirty = true;
                                    }
                                });
                            }
                        });

                    ui.add_space(6.0);

                    // 의존성 추가용 옵션 목록 생성
                    let mut options: Vec<String> = self
                        .state
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
                                    self.state.add_connection(option, &selected_id);
                                    mark_dirty = true;
                                }
                            }
                        });
                }

                ui.separator();
                if ui.button("이 Step 삭제").clicked() {
                    self.state.remove_node(&selected_id);
                    mark_dirty = true;
                }
            }
        });
        self.state.dirty = mark_dirty;
    }

    /// Step 구성 UI를 노출한다.
    fn render_step_config_ui(
        ui: &mut egui::Ui,
        config: &mut EditorStepConfig,
        mark_dirty: &mut bool,
    ) {
        match config {
            EditorStepConfig::Sql { sql, target_db } => {
                ui.label("대상 DB");
                let mut db_buf = target_db.clone().unwrap_or_default();
                if ui.text_edit_singleline(&mut db_buf).changed() {
                    *target_db = if db_buf.is_empty() {
                        None
                    } else {
                        Some(db_buf)
                    };
                    *mark_dirty = true;
                }
                ui.label("SQL");
                if ui.text_edit_multiline(sql).changed() {
                    *mark_dirty = true;
                }
            }
            EditorStepConfig::SqlFile { path, target_db } => {
                ui.label("대상 DB");
                let mut db_buf = target_db.clone().unwrap_or_default();
                if ui.text_edit_singleline(&mut db_buf).changed() {
                    *target_db = if db_buf.is_empty() {
                        None
                    } else {
                        Some(db_buf)
                    };
                    *mark_dirty = true;
                }
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
                    let selected = config.selected_node_id.as_deref()
                        == Some(child.id.as_str());
                    if ui
                        .selectable_label(
                            selected,
                            format!("{} ({:?})", child.name, child.kind),
                        )
                        .clicked()
                    {
                        config.selected_node_id = Some(child.id.clone());
                    }
                }
                if let Some(selected_id) = config.selected_node_id.clone() {
                    // 1) 먼저 불변 빌림으로 deps / options를 계산
                    let deps = config.dependencies_of(&selected_id);
                    let mut options: Vec<String> = config
                        .nodes
                        .iter()
                        .filter(|n| n.id != selected_id)
                        .map(|n| n.id.clone())
                        .collect();
                    options.sort();

                    // 2) UI에서 클릭 결과를 임시로 모아둘 버퍼
                    let mut deps_to_remove: Vec<String> = Vec::new();
                    let mut deps_to_add: Vec<String> = Vec::new();

                    // 3) 이제 가변 빌림으로 child 편집 + 의존성 UI 렌더링
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
                        Self::render_step_config_ui(ui, &mut child.config, mark_dirty);
                        Self::render_confirm_section(ui, &mut child.confirm, mark_dirty);

                        ui.separator();
                        ui.label("의존성");

                        // ← 이미 계산한 deps를 사용하면서,
                        //    실제 remove는 나중에 처리하기 위해 deps_to_remove에 기록만 한다
                        for dep_id in &deps {
                            let dep_id = dep_id.clone();
                            ui.horizontal(|ui| {
                                ui.label(&dep_id);
                                if ui.button("삭제").clicked() {
                                    deps_to_remove.push(dep_id.clone());
                                    *mark_dirty = true;
                                }
                            });
                        }

                        // 마찬가지로 options도 미리 계산된 걸 사용
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

                    // 4) child에 대한 &mut borrow가 끝난 이후에
                    //    실제로 config를 다시 &mut로 빌려서 연결 변경을 반영
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

        // optional_path_field도 self 없이 쓰는 버전으로 분리하는 게 베스트
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
    }

    // 기존 self.optional_path_field(...) 가 있었다면,
    // 이렇게 "self 없는 버전" 헬퍼로 분리
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

    /// 선택적 Path 입력 필드를 렌더링한다.
    fn optional_path_field(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        target: &mut Option<std::path::PathBuf>,
    ) {
        ui.label(label);
        let mut buf = target
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        if ui.text_edit_singleline(&mut buf).changed() {
            *target = if buf.is_empty() {
                None
            } else {
                Some(buf.into())
            };
            self.state.dirty = true;
        }
    }

    /// 캔버스를 렌더링하고 노드/연결 상호작용을 처리한다.
    fn render_canvas(&mut self, ui: &mut egui::Ui, colors: BuilderColors) {
        let desired_size = egui::vec2(2400.0, 1600.0);
        egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
                let painter = ui.painter_at(rect);
                let mut pending_selection: Option<String> = None;
                if response.clicked() && !response.dragged() {
                    self.state.select_node(None);
                }
                let origin = rect.min.to_vec2();
                self.draw_connections(&painter, colors, origin);
                for idx in 0..self.state.nodes.len() {
                    let (node_id, node_rect) = {
                        let node = &self.state.nodes[idx];
                        let shape = egui::Rect::from_min_size(
                            rect.min + node.position.to_vec2(),
                            node.size,
                        );
                        (node.id.clone(), shape)
                    };
                    let response_id = egui::Id::new(("builder_node", node_id.clone()));
                    let node_response =
                        ui.interact(node_rect, response_id, egui::Sense::click_and_drag());
                    if node_response.dragged() {
                        if let Some(node) = self.state.node_mut(&node_id) {
                            node.position += node_response.drag_delta();
                        }
                        self.state.dirty = true;
                    }
                    if node_response.clicked() {
                        pending_selection = Some(node_id.clone());
                    }
                    if let Some(node) = self.state.node(&node_id) {
                        self.draw_node(&painter, node_rect, node, colors);
                    }
                }
                if let Some(id) = pending_selection {
                    self.state.select_node(Some(id));
                }
            });
    }

    /// 연결 선을 그린다.
    fn draw_connections(&self, painter: &egui::Painter, colors: BuilderColors, origin: egui::Vec2) {
        for conn in &self.state.connections {
            if let (Some(from), Some(to)) =
                (self.state.node(&conn.from_id), self.state.node(&conn.to_id))
            {
                let start = from.position + egui::vec2(from.size.x / 2.0, from.size.y);
                let end = to.position + egui::vec2(to.size.x / 2.0, 0.0);
                let start = egui::pos2(start.x + origin.x, start.y + origin.y);
                let end = egui::pos2(end.x + origin.x, end.y + origin.y);
                painter.add(CubicBezierShape::from_points_stroke(
                    [
                        start,
                        start + egui::vec2(0.0, 60.0),
                        end - egui::vec2(0.0, 60.0),
                        end,
                    ],
                    false,                      // closed
                    egui::Color32::TRANSPARENT, // fill 없음
                    Stroke::new(2.0, colors.connection_stroke),
                ));
            }
        }
    }

    /// 개별 노드를 드로잉한다.
    fn draw_node(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        node: &super::model::EditorStepNode,
        colors: BuilderColors,
    ) {
        let bg = if node.selected {
            colors.node_selected
        } else {
            colors.node_fill
        };
        painter.rect_filled(rect, 10.0, bg);
        painter.rect_stroke(rect, 10.0, egui::Stroke::new(1.6, colors.node_border));
        let title_pos = rect.min + egui::vec2(10.0, 8.0);
        painter.text(
            title_pos,
            egui::Align2::LEFT_TOP,
            &node.name,
            egui::FontId::proportional(16.0),
            colors.text_primary,
        );
        let id_pos = rect.min + egui::vec2(10.0, 30.0);
        painter.text(
            id_pos,
            egui::Align2::LEFT_TOP,
            format!("ID: {}", node.id),
            egui::FontId::proportional(12.0),
            colors.text_secondary,
        );
        let visual = self.theme.step_visual(Self::visual_kind_for(node.kind));
        let mut subtitle = visual.label.to_string();
        if let EditorStepConfig::Extract { config } = &node.config {
            if config.var_name.is_empty() {
                subtitle = format!("{} → 변수 미지정", visual.label);
            } else {
                subtitle = format!("{} → ${}", visual.label, config.var_name);
            }
        } else if let EditorStepConfig::Loop { config } = &node.config {
            subtitle = format!("{} · {} steps", visual.label, config.nodes.len());
        }
        let type_pos = rect.min + egui::vec2(10.0, 48.0);
        painter.text(
            type_pos,
            egui::Align2::LEFT_TOP,
            format!("{} {}", visual.icon, subtitle),
            egui::FontId::proportional(14.0),
            visual.color,
        );
        let input_center = rect.center_top() - egui::vec2(0.0, 6.0);
        let output_center = rect.center_bottom() + egui::vec2(0.0, 6.0);
        painter.circle_filled(input_center, 5.0, colors.handle_fill);
        painter.circle_filled(output_center, 5.0, colors.handle_fill);
    }

    /// StepKind를 시각 스타일 분류로 매핑한다.
    fn visual_kind_for(kind: StepKind) -> StepVisualKind {
        match kind {
            StepKind::Sql => StepVisualKind::Sql,
            StepKind::SqlFile => StepVisualKind::SqlFile,
            StepKind::SqlLoaderPar => StepVisualKind::SqlLoader,
            StepKind::Shell => StepVisualKind::Shell,
            StepKind::Extract => StepVisualKind::Extract,
            StepKind::Loop => StepVisualKind::Loop,
        }
    }
}
