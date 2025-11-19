use eframe::egui;
use std::collections::HashSet;

use super::connection::EditorConnection;
use super::step::{EditorStepNode, StepKind};

/// 시나리오 에디터 전체 상태를 저장한다.
#[derive(Debug, Clone)]
pub struct ScenarioEditorState {
    /// 노드 목록.
    pub nodes: Vec<EditorStepNode>,
    /// 연결 목록.
    pub connections: Vec<EditorConnection>,
    /// 선택된 노드 ID.
    pub selected_node_id: Option<String>,
    /// 현재 파일 경로.
    pub current_file: Option<std::path::PathBuf>,
    /// 캔버스 오프셋.
    pub canvas_offset: egui::Vec2,
    /// 캔버스 줌 비율.
    pub canvas_zoom: f32,
    /// 미완성 연결 시작 노드.
    pub pending_connection: Option<String>,
    /// 저장되지 않은 변경 여부.
    pub dirty: bool,
}

impl ScenarioEditorState {
    /// 빈 에디터 상태를 생성한다.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            connections: Vec::new(),
            selected_node_id: None,
            current_file: None,
            canvas_offset: egui::vec2(0.0, 0.0),
            canvas_zoom: 1.0,
            pending_connection: None,
            dirty: false,
        }
    }

    /// 고유한 Step ID를 생성한다.
    pub fn generate_id(&self, prefix: &str) -> String {
        let mut idx = 1;
        let ids: HashSet<&str> = self.nodes.iter().map(|n| n.id.as_str()).collect();
        loop {
            let candidate = format!("{prefix}_{idx}");
            if !ids.contains(candidate.as_str()) {
                return candidate;
            }
            idx += 1;
        }
    }

    /// 새 노드를 추가하고 선택한다.
    pub fn add_node(&mut self, kind: StepKind) -> String {
        let id = self.generate_id("step");
        let mut node = EditorStepNode::new(id.clone(), format!("새 Step {id}"), kind);
        node.position = egui::pos2(80.0, 80.0);
        self.nodes.push(node);
        self.select_node(Some(id.clone()));
        self.dirty = true;
        id
    }

    /// 지정된 노드를 제거한다.
    pub fn remove_node(&mut self, id: &str) {
        self.nodes.retain(|node| node.id != id);
        self.connections
            .retain(|conn| conn.from_id != id && conn.to_id != id);
        if self.selected_node_id.as_deref() == Some(id) {
            self.selected_node_id = None;
        }
        self.dirty = true;
    }

    /// 노드 선택 상태를 갱신한다.
    pub fn select_node(&mut self, id: Option<String>) {
        self.selected_node_id = id.clone();
        for node in &mut self.nodes {
            node.selected = Some(node.id.as_str()) == id.as_deref();
        }
    }

    /// ID로 노드를 조회한다.
    pub fn node_mut(&mut self, id: &str) -> Option<&mut EditorStepNode> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    /// ID로 노드를 조회한다.
    pub fn node(&self, id: &str) -> Option<&EditorStepNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    /// depends_on 목록을 반환한다.
    pub fn dependencies_of(&self, id: &str) -> Vec<String> {
        self.connections
            .iter()
            .filter(|conn| conn.to_id == id)
            .map(|conn| conn.from_id.clone())
            .collect()
    }

    /// 연결을 추가한다.
    pub fn add_connection(&mut self, from_id: &str, to_id: &str) {
        if from_id == to_id {
            return;
        }
        let conn = EditorConnection {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
        };
        if !self.connections.contains(&conn) {
            self.connections.push(conn);
            self.dirty = true;
        }
    }

    /// 연결을 제거한다.
    pub fn remove_connection(&mut self, from_id: &str, to_id: &str) {
        self.connections
            .retain(|conn| !(conn.from_id == from_id && conn.to_id == to_id));
        self.dirty = true;
    }
}

impl Default for ScenarioEditorState {
    /// 기본 상태를 반환한다.
    fn default() -> Self {
        Self::new()
    }
}
