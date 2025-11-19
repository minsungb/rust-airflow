use crate::scenario::ConfirmDefault;

/// 컨펌 요청이 어느 시점인지 나타내는 값이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmPhase {
    /// 실행 전 확인 단계이다.
    Before,
    /// 실행 후 확인 단계이다.
    After,
}

/// 엔진에서 UI로 전달되는 주요 이벤트 모델이다.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    /// Step 시작 알림이다.
    StepStarted { step_id: String },
    /// Step별 로그 라인이다.
    StepLog { step_id: String, line: String },
    /// Step 종료 알림이다.
    StepFinished { step_id: String, success: bool },
    /// 컨펌을 위해 사용자 입력이 필요한 경우 발생한다.
    RequestConfirm {
        /// 컨펌 요청 ID이다.
        request_id: u64,
        /// 대상 Step ID이다.
        step_id: String,
        /// Step 이름이다.
        step_name: String,
        /// Step 종류 문자열이다.
        step_kind: String,
        /// Step 요약 정보이다.
        summary: Option<String>,
        /// 사용자에게 보여줄 메시지이다.
        message: Option<String>,
        /// 기본 응답이다.
        default_answer: ConfirmDefault,
        /// 컨펌 단계이다.
        phase: ConfirmPhase,
    },
    /// 컨펌 요청에 응답이 완료되면 전달된다.
    ConfirmResponse {
        /// 요청 ID이다.
        request_id: u64,
        /// 대상 Step ID이다.
        step_id: String,
        /// 수락 여부이다.
        accepted: bool,
    },
    /// 전체 시나리오 종료이다.
    ScenarioFinished,
}
