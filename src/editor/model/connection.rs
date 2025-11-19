/// 노드 간의 방향성 연결을 표현한다.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EditorConnection {
    /// 의존성을 제공하는 노드 ID.
    pub from_id: String,
    /// 의존성을 갖는 노드 ID.
    pub to_id: String,
}
