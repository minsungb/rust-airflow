use super::model::{EditorStepConfig, ScenarioEditorState, StepKind};
use crate::theme::{BuilderColors, Theme};
use eframe::egui;
use std::collections::HashMap;
use eframe::epaint::{CubicBezierShape, Stroke};

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
        .show(ui, |ui| {
                    
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

                    match &mut selected.config {
                        EditorStepConfig::Sql { sql, target_db } => {
                            ui.label("대상 DB");
                            let mut db_buf = target_db.clone().unwrap_or_default();
                            if ui.text_edit_singleline(&mut db_buf).changed() {
                                *target_db = if db_buf.is_empty() {
                                    None
                                } else {
                                    Some(db_buf)
                                };
                                mark_dirty = true;
                            }
                            ui.label("SQL");
                            if ui.text_edit_multiline(sql).changed() {
                                mark_dirty = true;
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
                                mark_dirty = true;
                            }
                            ui.label("SQL 파일 경로");
                            let mut path_buf = path.display().to_string();
                            if ui.text_edit_singleline(&mut path_buf).changed() {
                                *path = std::path::PathBuf::from(path_buf);
                                mark_dirty = true;
                            }
                        }
                        EditorStepConfig::SqlLoaderPar { config } => {
                            Self::render_sqlldr(ui, config, &mut mark_dirty);
                        }
                        EditorStepConfig::Shell { config } => {
                            Self::render_shell(ui, config, &mut mark_dirty);
                        }
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
                painter.add(
                    CubicBezierShape::from_points_stroke(
                        [
                            start,
                            start + egui::vec2(0.0, 60.0),
                            end - egui::vec2(0.0, 60.0),
                            end,
                        ],
                        false,                        // closed
                        egui::Color32::TRANSPARENT,   // fill 없음
                        Stroke::new(2.0, colors.connection_stroke),
                    ),
                );
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
        let input_center = rect.center_top() - egui::vec2(0.0, 6.0);
        let output_center = rect.center_bottom() + egui::vec2(0.0, 6.0);
        painter.circle_filled(input_center, 5.0, colors.handle_fill);
        painter.circle_filled(output_center, 5.0, colors.handle_fill);
    }
}
