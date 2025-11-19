use super::*;

/// StepStatus를 기반으로 직관적인 아이콘과 텍스트를 반환한다.
pub(super) fn status_indicator(status: &StepStatus) -> (&'static str, &'static str) {
    match status {
        StepStatus::Pending => ("⏳", "대기 중"),
        StepStatus::Running => ("⚙️", "실행 중"),
        StepStatus::Success => ("✅", "성공"),
        StepStatus::Failed(_) => ("❌", "실패"),
    }
}
