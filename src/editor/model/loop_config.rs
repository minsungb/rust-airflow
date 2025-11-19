use crate::scenario::{LoopIterationFailure, LoopStepConfig};
use eframe::egui;
use std::collections::HashSet;

use super::connection::EditorConnection;
use super::error::EditorError;
use super::step::EditorStepNode;

/// Loop 하위 흐름을 표현하는 구성체이다.
#[derive(Debug, Clone)]
pub struct LoopEditorConfig {
    /// 반복 대상 glob 패턴.
    pub for_each_glob: String,
    /// 변수명.
    pub as_var: String,
    /// 실패 정책.
    pub on_iteration_failure: LoopIterationFailure,
    /// 하위 노드 목록.
    pub nodes: Vec<EditorStepNode>,
    /// 하위 연결 목록.
    pub connections: Vec<EditorConnection>,
    /// 선택된 하위 노드.
    pub selected_node_id: Option<String>,
}

impl LoopEditorConfig {
    /// 기본 Loop 구성을 생성한다.
    pub fn new() -> Self {
        Self {
            for_each_glob: String::new(),
            as_var: "ITEM".into(),
            on_iteration_failure: LoopIterationFailure::StopAll,
            nodes: Vec::new(),
            connections: Vec::new(),
            selected_node_id: None,
        }
    }

    /// 시나리오 Loop 설정을 에디터 구성으로 변환한다.
    pub fn from_scenario_config(config: &LoopStepConfig) -> Self {
        let mut nodes = Vec::new();
        let mut connections = Vec::new();
        for step in &config.steps {
            nodes.push(EditorStepNode::from_scenario_step(step));
            for dep in &step.depends_on {
                connections.push(EditorConnection {
                    from_id: dep.clone(),
                    to_id: step.id.clone(),
                });
            }
        }
        Self {
            for_each_glob: config.for_each_glob.clone(),
            as_var: config.as_var.clone(),
            on_iteration_failure: config.on_iteration_failure.clone(),
            nodes,
            connections,
            selected_node_id: None,
        }
    }

    /// 하위 노드용 고유 ID를 생성한다.
    pub fn generate_child_id(&self) -> String {
        let mut idx = 1;
        let ids: HashSet<&str> = self.nodes.iter().map(|n| n.id.as_str()).collect();
        loop {
            let candidate = format!("loop_step_{idx}");
            if !ids.contains(candidate.as_str()) {
                return candidate;
            }
            idx += 1;
        }
    }

    /// 하위 노드를 조회한다.
    pub fn node_mut(&mut self, id: &str) -> Option<&mut EditorStepNode> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    /// 하위 노드를 조회한다.
    pub fn node(&self, id: &str) -> Option<&EditorStepNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    /// 특정 노드의 의존성 목록을 반환한다.
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
        }
    }

    /// 연결을 제거한다.
    pub fn remove_connection(&mut self, from_id: &str, to_id: &str) {
        self.connections
            .retain(|conn| !(conn.from_id == from_id && conn.to_id == to_id));
    }

    /// 노드를 제거하고 연결을 정리한다.
    pub fn remove_node(&mut self, id: &str) {
        self.nodes.retain(|node| node.id != id);
        self.connections
            .retain(|conn| conn.from_id != id && conn.to_id != id);
        if self.selected_node_id.as_deref() == Some(id) {
            self.selected_node_id = None;
        }
    }

    /// Loop 구성을 Scenario 구조로 변환한다.
    pub fn to_loop_step_config(&self) -> Result<LoopStepConfig, EditorError> {
        let mut steps = Vec::new();
        for node in &self.nodes {
            let deps = self.dependencies_of(&node.id);
            steps.push(node.to_scenario_step(deps)?);
        }
        Ok(LoopStepConfig {
            for_each_glob: self.for_each_glob.clone(),
            as_var: self.as_var.clone(),
            steps,
            on_iteration_failure: self.on_iteration_failure.clone(),
        })
    }
}
