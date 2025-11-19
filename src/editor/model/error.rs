/// 에디터 작업 중 발생 가능한 오류를 표현한다.
#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    /// Step ID가 중복된 경우이다.
    #[error("중복된 Step ID가 존재합니다: {0}")]
    DuplicateStepId(String),
    /// 존재하지 않는 노드를 참조하는 연결이다.
    #[error("존재하지 않는 노드를 참조하는 연결입니다: {from_id} -> {to_id}")]
    MissingNode { from_id: String, to_id: String },
    /// 순환 의존성이 감지된 경우이다.
    #[error("순환 의존성이 감지되었습니다. 연결 구성을 확인하세요.")]
    CyclicDependency,
}
