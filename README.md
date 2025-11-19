# Rust Batch Orchestrator

Tokio + egui 기반의 경량 배치 오케스트레이터입니다. YAML로 정의된 시나리오를 로드하여 DAG 형태로 실행하고, Step별 상태/로그를 실시간으로 모니터링할 수 있습니다.

## 주요 기능

- SQL / SQL 파일 / sqlldr / Shell Step 지원
- depends_on을 이용한 DAG 실행 및 allow_parallel 기반 병렬 처리
- Step별 재시도/타임아웃/지수 백오프 관리
- EngineEvent 채널을 통해 CLI 엔진과 egui UI 분리
- build.rs가 시스템 한글 폰트를 탐색하여 egui에 적용
- Windows 빌드 시 아이콘 자동 임베드 (사용자 제공 `icons/icon.ico` 필요)
- Power Automate 스타일의 Scenario Builder에서 드래그 앤 드롭으로 DAG 설계

## 실행 방법

```bash
cargo run --release
```

애플리케이션 실행 후 좌측 상단 `시나리오 열기` 버튼으로 YAML 파일을 선택합니다. 예시 시나리오는 `scenarios/sample_finance_job.yaml`에 포함되어 있습니다.

## Scenario Builder UI

- 상단 탭에서 **Scenario Builder**를 선택하면 좌측 팔레트/중앙 플로우 캔버스/우측 속성 패널이 나타납니다.
- 팔레트에서 Step 유형(SQL, SQL 파일, SQL*Loader, Shell)을 클릭하면 캔버스에 새 노드가 추가됩니다.
- 노드를 드래그해 위치를 조정하고, 우측 패널에서 ID/이름/SQL/셸 스크립트 등을 편집합니다.
- `의존성 추가` 콤보박스로 노드 간 연결을 지정하면 `depends_on` 관계가 자동 생성됩니다.
- 상단 빌더 툴바에서 `저장`/`다른 이름으로`를 클릭하면 YAML로 내보낼 수 있고, `실행` 버튼으로 즉시 엔진을 구동할 수 있습니다.
- 새로 작성하거나 수정한 플로우는 `docs/examples/sample_flow.yaml`을 참고하여 테스트할 수 있습니다.

### Step 실행 순서

- `collect_ready_steps` 로직은 의존성(`depends_on`)이 모두 충족된 Step을 먼저 모읍니다.
- 준비된 Step들 가운데 `allow_parallel: false`인 Step이 하나라도 있다면, 해당 Step들은 모두 `sequential` 큐로 보내져 **먼저 순차 실행**되고 완료되어야 합니다.
- 순차 실행이 끝난 뒤에야 `allow_parallel: true`인 Step들이 `tokio::spawn`으로 동시에 실행됩니다. 따라서 동일 시점에 준비되었더라도 `allow_parallel` 값을 통해 실제 병렬 여부를 제어합니다.
- 선행 Step이 실패하면 `mark_blocked_steps`가 해당 Step을 건너뛰고 실패 처리하므로, 의존 관계를 설계할 때 실패 전파를 감안해야 합니다.

### 상위 Step 값 전달 방법

- 엔진은 `src/engine/context.rs`의 `ExecutionContext`를 통해 실행 중 변수를 공유합니다. Step에서 `${VAR_NAME}` 형태의 플레이스홀더를 사용하면 컨텍스트 값 또는 OS 환경 변수를 치환합니다.
- **Extract Step**: `ExtractVarFromFile` 유형을 사용하면 파일에서 정규식으로 값을 추출해 `var_name`으로 저장합니다. 이후 SQL/Shell Step의 `sql`, `sql_file`, `shell.script` 등에 `${var_name}`을 삽입하면 치환됩니다.
- **Loop Step**: `LoopStepConfig`의 `as_var`에 지정한 변수에 현재 파일/엔트리 경로가 저장되며, 하위 Step에서 `${as_var}`로 접근할 수 있습니다.
- 컨텍스트에 값이 없거나 정규식이 매칭되지 않으면 해당 Step이 즉시 실패하므로, 파일 경로와 그룹 번호를 정확히 설정해야 합니다.

## 시나리오 규칙

```yaml
name: "sample_finance_job"
steps:
  - id: "01_extract"
    name: "연도 추출 SQL 실행"
    kind: sql_file
    sql_file: "sql/01_extract_year.sql"
    depends_on: []
    allow_parallel: false
    retry: 1
    timeout_sec: 600
```

`kind`는 `sql`, `sql_file`, `sql_loader_par`, `shell` 중 하나를 선택하며, Kind별 설정은 동일 레벨에 추가 필드로 작성합니다.

```yaml
  - id: load_customer
    name: 고객 마스터 적재
    kind: sql_loader_par
    sqlldr:
      conn: "APP_USER/secret@ORCLPDB1"
      control_file: "/app/batch/ctl/customer.ctl"
      data_file: "/app/batch/data/customer.dat"
      log_file: "/app/batch/logs/customer.log"
```

Shell Step은 다음과 같이 실행 스크립트, 계정, 환경 변수를 지정할 수 있습니다.

```yaml
  - id: notify
    name: 알림 전송
    kind: shell
    shell:
      script: "./scripts/notify.sh"
      shell_program: "/bin/bash"
      env:
        SERVICE: prod
      run_as: batchuser
      error_policy: ignore
```

## 프로젝트 구조

- `src/scenario.rs` – Step/Scenario 도메인 및 YAML 로더
- `src/engine.rs` – DAG 실행기, Step 상태 관리, 이벤트 송신
- `src/executor.rs` – DB 실행 추상화 및 Dummy 실행기, sqlldr 실행 도우미
- `src/app.rs` – egui UI 및 이벤트 수신 로직
- `src/theme.rs` – 테마/폰트 관리
- `build.rs` – 시스템 폰트 탐색 및 Windows 아이콘 임베딩
- `docs/` – 사용자 제공 스크린샷 등 문서 자산 디렉터리 (기본 파일 없음)
- `icons/` – 사용자 제공 `icon.ico` 배치 위치

## 주의사항

- sqlldr, shell Step은 실제 환경에 맞게 명령어/경로를 수정해야 합니다.
- DummyExecutor는 SQL을 실제 DB에 전달하지 않으므로, 실제 환경에서는 `DbExecutor`를 구현하세요.
- UI 로그는 Step별 500줄까지 보존되며 초과 시 오래된 로그부터 삭제됩니다.
- UI 스크린샷이나 Windows 아이콘과 같은 바이너리 자산은 사용자가 직접 추가해야 합니다.

## 문서 사용 안내

- 저장소 전반의 개요, 실행법, 폴더 구조는 본 `README.md`를 업데이트하여 유지합니다.
- 세부 사용법이나 팀 내 위키성 자료는 `docs/` 디렉터리 아래 별도 Markdown 파일로 작성하고, 본문에 링크를 추가합니다.
- `docs/DOCUMENTATION_GUIDE.md`에는 문서 작성 규칙과 샘플 목차가 정리되어 있으니 새 문서를 만들기 전에 반드시 확인하세요.
- 문서나 예제 파일에 등장하는 경로나 시나리오 이름은 실제 제공되는 자산과 동기화하여 독자가 그대로 따라 할 수 있도록 유지합니다.
