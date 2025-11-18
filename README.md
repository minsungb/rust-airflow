# Rust Batch Orchestrator

Tokio + egui 기반의 경량 배치 오케스트레이터입니다. YAML로 정의된 시나리오를 로드하여 DAG 형태로 실행하고, Step별 상태/로그를 실시간으로 모니터링할 수 있습니다.

## 주요 기능

- SQL / SQL 파일 / sqlldr / Shell Step 지원
- depends_on을 이용한 DAG 실행 및 allow_parallel 기반 병렬 처리
- Step별 재시도/타임아웃/지수 백오프 관리
- EngineEvent 채널을 통해 CLI 엔진과 egui UI 분리
- build.rs가 시스템 한글 폰트를 탐색하여 egui에 적용
- Windows 빌드 시 아이콘 자동 임베드 (사용자 제공 `icons/icon.ico` 필요)

## 실행 방법

```bash
cargo run --release
```

애플리케이션 실행 후 좌측 상단 `시나리오 열기` 버튼으로 YAML 파일을 선택합니다. 예시 시나리오는 `scenarios/sample_finance_job.yaml`에 포함되어 있습니다.

## 시나리오 규칙

```yaml
name: "sample_finance_job"
steps:
  - id: "01_extract"
    name: "연도 추출 SQL 실행"
    kind:
      sql_file: "sql/01_extract_year.sql"
    depends_on: []
    allow_parallel: false
    retry: 1
    timeout_sec: 600
```

`kind`는 `sql`, `sql_file`, `sqlldr_par`, `shell` 중 하나를 선택하며, Kind별 설정은 동일 레벨에 추가 필드로 작성합니다.

```yaml
  - id: load_customer
    name: 고객 마스터 적재
    kind: sqlldr_par
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
