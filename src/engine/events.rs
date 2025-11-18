/// 엔진에서 UI로 전달되는 주요 이벤트 모델이다.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    /// Step 시작 알림.
    StepStarted { step_id: String },
    /// Step별 로그 라인.
    StepLog { step_id: String, line: String },
    /// Step 종료 알림.
    StepFinished { step_id: String, success: bool },
    /// 전체 시나리오 종료.
    ScenarioFinished,
}
