use crate::scenario::Scenario;
use std::collections::HashMap;
use std::time::Instant;

/// Step의 런타임 상태를 표현한다.
#[derive(Debug, Clone)]
pub enum StepStatus {
    /// 아직 실행 대기 상태.
    Pending,
    /// 실행 중.
    Running,
    /// 정상 종료.
    Success,
    /// 실패와 함께 오류 메시지를 포함한다.
    Failed(String),
}

/// Step의 시간 및 로그 정보를 담는다.
#[derive(Debug, Clone)]
pub struct StepRuntimeState {
    /// 현재 상태 값.
    pub status: StepStatus,
    /// 시작 시각.
    pub started_at: Option<Instant>,
    /// 종료 시각.
    pub finished_at: Option<Instant>,
    /// 메모리에 적재된 로그 버퍼.
    pub logs: Vec<String>,
}

impl StepRuntimeState {
    /// 초기 상태를 생성한다.
    pub fn new() -> Self {
        Self {
            status: StepStatus::Pending,
            started_at: None,
            finished_at: None,
            logs: Vec::new(),
        }
    }
}

impl Default for StepRuntimeState {
    /// StepRuntimeState의 기본값을 정의한다.
    fn default() -> Self {
        Self::new()
    }
}

/// Scenario 실행 중 Step 상태 맵을 관리한다.
#[derive(Debug, Clone)]
pub struct ScenarioRuntime {
    /// 원본 시나리오 정의.
    pub scenario: Scenario,
    /// Step별 상태 맵.
    pub steps_state: HashMap<String, StepRuntimeState>,
}

impl ScenarioRuntime {
    /// Scenario를 받아 초기 상태를 생성한다.
    pub fn new(scenario: Scenario) -> Self {
        let steps_state = scenario
            .steps
            .iter()
            .map(|step| (step.id.clone(), StepRuntimeState::new()))
            .collect();
        Self {
            scenario,
            steps_state,
        }
    }
}
