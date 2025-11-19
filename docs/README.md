# Docs 사용 안내

이 디렉터리는 사용자 매뉴얼, 스크린샷, 예제 시나리오 등 **실행 가이드 자산**을 보관합니다. 기본적으로 Markdown 문서와 이미지를 함께 배치하며, 새 문서를 추가할 때는 `docs/DOCUMENTATION_GUIDE.md`의 규칙을 따라 주세요.

## Scenario Builder 사용 팁

1. `docs/examples/sample_flow.yaml`을 열어 Scenario Builder에 드래그 앤 드롭하면 노드 구성이 자동 복원됩니다.
2. 의존성이 없는 Step이 여러 개라면 `allow_parallel` 값이 `false`인 Step부터 순차 실행되고, 해당 Step들이 모두 끝나야 `true`인 Step이 병렬로 시작됩니다.
3. 특정 Step이 실패하면 `mark_blocked_steps`가 하위 Step을 즉시 실패 처리하므로, 필수 전처리 Step의 `allow_parallel` 값을 신중히 설정합니다.

## 컨텍스트 변수 전달 예시

1. 상위 Step에서 값을 기록하려면 `ExtractVarFromFile` Step을 추가하고 `var_name`을 지정합니다. Regex 그룹에서 읽어온 값이 실행 컨텍스트에 저장됩니다.
2. Loop Step을 사용할 경우 `for_each_glob` 패턴에 매칭되는 파일마다 `as_var`에 지정한 이름으로 경로가 저장됩니다.
3. 하위 Step에서는 SQL, Shell 스크립트, 파일 경로 등 문자열 필드에 `${VAR_NAME}` 형태의 플레이스홀더를 적으면 실행 시점에 컨텍스트 값으로 치환됩니다. 값이 없으면 Step이 실패하므로, Scenario Builder 속성 패널에서 변수명을 정확히 일치시킵니다.

필요한 스크린샷이나 다이어그램이 있다면 `docs/` 하위에 PNG/JPEG로 저장하고, 관련 Markdown에서 상대 경로로 링크하세요.
