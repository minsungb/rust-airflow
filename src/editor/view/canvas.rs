use super::*;

impl<'a> ScenarioBuilderUi<'a> {
    /// 캔버스를 렌더링하고 노드/연결 상호작용을 처리한다.
    pub(super) fn render_canvas(&mut self, ui: &mut egui::Ui, colors: BuilderColors) {
        let desired_size = egui::vec2(2400.0, 1600.0);
        egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
                let painter = ui.painter_at(rect);
                let mut pending_selection: Option<String> = None;
                if response.clicked() && !response.dragged() {
                    self.clear_selection();
                }
                let origin = rect.min.to_vec2();
                self.draw_connections(&painter, colors, origin);
                for idx in 0..self.get_state().nodes.len() {
                    let (node_id, node_rect) = {
                        let node = &self.get_state().nodes[idx];
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
                        if let Some(node) = self.get_state_mut().node_mut(&node_id) {
                            node.position += node_response.drag_delta();
                        }
                        self.get_state_mut().dirty = true;
                    }
                    if node_response.clicked() {
                        pending_selection = Some(node_id.clone());
                    }
                    if let Some(node) = self.get_state().node(&node_id) {
                        self.draw_node(&painter, node_rect, node, colors);
                    }
                }
                if let Some(id) = pending_selection {
                    self.get_state_mut().select_node(Some(id));
                }
            });
    }

    /// 연결 선을 그린다.
    fn draw_connections(
        &mut self,
        painter: &egui::Painter,
        colors: BuilderColors,
        origin: egui::Vec2,
    ) {
        for conn in &self.get_state().connections {
            if let (Some(from), Some(to)) = (
                self.get_state().node(&conn.from_id),
                self.get_state().node(&conn.to_id),
            ) {
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
                    false,
                    egui::Color32::TRANSPARENT,
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
        node: &EditorStepNode,
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
        let visual = self
            .get_theme()
            .step_visual(Self::visual_kind_for(node.kind));
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
