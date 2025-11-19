use super::model::{
    EditorConnection, EditorError, EditorStepConfig, EditorStepNode, LoopEditorConfig,
    ScenarioEditorState,
};
use crate::scenario::Scenario;
use eframe::egui;
use std::collections::{HashMap, HashSet, VecDeque};

/// Scenario를 에디터 상태로 변환한다.
pub fn scenario_to_editor_state(scenario: &Scenario) -> ScenarioEditorState {
    let mut state = ScenarioEditorState::new();
    let mut levels: HashMap<String, usize> = HashMap::new();
    for step in &scenario.steps {
        let node = EditorStepNode::from_scenario_step(step);
        state.nodes.push(node);
    }
    for step in &scenario.steps {
        for dep in &step.depends_on {
            state.connections.push(EditorConnection {
                from_id: dep.clone(),
                to_id: step.id.clone(),
            });
        }
    }
    for step in &scenario.steps {
        assign_level(step.id.as_str(), scenario, &mut levels);
    }
    let mut per_level: HashMap<usize, Vec<String>> = HashMap::new();
    for (id, level) in &levels {
        per_level.entry(*level).or_default().push(id.clone());
    }
    let mut level_keys: Vec<usize> = per_level.keys().cloned().collect();
    level_keys.sort_unstable();
    let spacing_x = 260.0;
    let spacing_y = 200.0;
    for level in level_keys {
        if let Some(ids) = per_level.get(&level) {
            for (idx, node_id) in ids.iter().enumerate() {
                if let Some(node) = state.node_mut(node_id) {
                    node.position = egui::pos2(
                        80.0 + idx as f32 * spacing_x,
                        80.0 + level as f32 * spacing_y,
                    );
                }
            }
        }
    }
    state
}

/// 재귀적으로 노드 레벨을 계산한다.
fn assign_level(step_id: &str, scenario: &Scenario, memo: &mut HashMap<String, usize>) -> usize {
    if let Some(level) = memo.get(step_id) {
        return *level;
    }
    let step = scenario
        .steps
        .iter()
        .find(|s| s.id == step_id)
        .expect("유효하지 않은 Step ID");
    if step.depends_on.is_empty() {
        memo.insert(step_id.to_string(), 0);
        return 0;
    }
    let mut max_dep = 0;
    for dep in &step.depends_on {
        let level = assign_level(dep, scenario, memo);
        max_dep = max_dep.max(level + 1);
    }
    memo.insert(step_id.to_string(), max_dep);
    max_dep
}

/// 에디터 상태를 Scenario로 변환한다.
pub fn editor_state_to_scenario(state: &ScenarioEditorState) -> Result<Scenario, EditorError> {
    let mut ids = HashSet::new();
    for node in &state.nodes {
        collect_ids_from_node(node, &mut ids)?;
        if let crate::editor::model::EditorStepConfig::Loop { config } = &node.config {
            validate_loop_connections(config)?;
        }
    }
    for conn in &state.connections {
        if state.node(&conn.from_id).is_none() || state.node(&conn.to_id).is_none() {
            return Err(EditorError::MissingNode {
                from_id: conn.from_id.clone(),
                to_id: conn.to_id.clone(),
            });
        }
    }
    if has_cycle(state) {
        return Err(EditorError::CyclicDependency);
    }
    let scenario_name = state
        .current_file
        .as_ref()
        .and_then(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "Scenario Builder".into());
    let mut scenario = Scenario {
        name: scenario_name,
        db: Default::default(),
        steps: Vec::new(),
    };
    for node in &state.nodes {
        let deps = state.dependencies_of(&node.id);
        scenario.steps.push(node.to_scenario_step(deps)?);
    }
    Ok(scenario)
}

/// 위상 정렬을 사용해 사이클 여부를 판별한다.
fn has_cycle(state: &ScenarioEditorState) -> bool {
    let mut indegree: HashMap<&str, usize> = HashMap::new();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in &state.nodes {
        indegree.insert(node.id.as_str(), 0);
        adj.insert(node.id.as_str(), Vec::new());
    }
    for conn in &state.connections {
        if let Some(entry) = indegree.get_mut(conn.to_id.as_str()) {
            *entry += 1;
        }
        if let Some(list) = adj.get_mut(conn.from_id.as_str()) {
            list.push(conn.to_id.as_str());
        }
    }
    let mut queue: VecDeque<&str> = indegree
        .iter()
        .filter_map(|(id, &deg)| if deg == 0 { Some(*id) } else { None })
        .collect();
    let mut visited = 0;
    while let Some(id) = queue.pop_front() {
        visited += 1;
        if let Some(children) = adj.get(id) {
            for child in children {
                if let Some(entry) = indegree.get_mut(child) {
                    *entry -= 1;
                    if *entry == 0 {
                        queue.push_back(child);
                    }
                }
            }
        }
    }
    visited != state.nodes.len()
}

/// Loop 내부에서 중복 ID를 검사한다.
fn collect_ids_from_node(
    node: &EditorStepNode,
    ids: &mut HashSet<String>,
) -> Result<(), EditorError> {
    if !ids.insert(node.id.clone()) {
        return Err(EditorError::DuplicateStepId(node.id.clone()));
    }
    if let EditorStepConfig::Loop { config } = &node.config {
        for child in &config.nodes {
            collect_ids_from_node(child, ids)?;
        }
    }
    Ok(())
}

/// Loop 하위 흐름의 연결/사이클을 검증한다.
fn validate_loop_connections(config: &LoopEditorConfig) -> Result<(), EditorError> {
    for conn in &config.connections {
        if config.node(&conn.from_id).is_none() || config.node(&conn.to_id).is_none() {
            return Err(EditorError::MissingNode {
                from_id: conn.from_id.clone(),
                to_id: conn.to_id.clone(),
            });
        }
    }
    if has_cycle_in_loop(config) {
        return Err(EditorError::CyclicDependency);
    }
    for node in &config.nodes {
        if let EditorStepConfig::Loop { config: inner } = &node.config {
            validate_loop_connections(inner)?;
        }
    }
    Ok(())
}

/// Loop 내부에 사이클이 있는지 검사한다.
fn has_cycle_in_loop(config: &LoopEditorConfig) -> bool {
    let mut indegree: HashMap<&str, usize> = HashMap::new();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in &config.nodes {
        indegree.insert(node.id.as_str(), 0);
        adj.insert(node.id.as_str(), Vec::new());
    }
    for conn in &config.connections {
        if let Some(entry) = indegree.get_mut(conn.to_id.as_str()) {
            *entry += 1;
        }
        if let Some(list) = adj.get_mut(conn.from_id.as_str()) {
            list.push(conn.to_id.as_str());
        }
    }
    let mut queue: VecDeque<&str> = indegree
        .iter()
        .filter_map(|(id, &deg)| if deg == 0 { Some(*id) } else { None })
        .collect();
    let mut visited = 0;
    while let Some(id) = queue.pop_front() {
        visited += 1;
        if let Some(children) = adj.get(id) {
            for child in children {
                if let Some(entry) = indegree.get_mut(child) {
                    *entry -= 1;
                    if *entry == 0 {
                        queue.push_back(child);
                    }
                }
            }
        }
    }
    if visited != config.nodes.len() {
        return true;
    }
    for node in &config.nodes {
        if let EditorStepConfig::Loop { config: inner } = &node.config {
            if has_cycle_in_loop(inner) {
                return true;
            }
        }
    }
    false
}
