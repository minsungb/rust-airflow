use super::super::*;
use super::*;
use std::collections::HashMap;

/// Step 구성 UI를 노출한다.
pub(super) fn render_step_config_ui(
    ui: &mut egui::Ui,
    config: &mut EditorStepConfig,
    mark_dirty: &mut bool,
    db_keys: &[String],
    id_hint: &str,
) {
    match config {
        EditorStepConfig::Sql { sql, target_db } => {
            render_target_db_picker(ui, target_db, db_keys, mark_dirty, id_hint);
            ui.label("SQL");
            if ui.text_edit_multiline(sql).changed() {
                *mark_dirty = true;
            }
        }
        EditorStepConfig::SqlFile { path, target_db } => {
            render_target_db_picker(ui, target_db, db_keys, mark_dirty, id_hint);
            ui.label("SQL 파일 경로");
            let mut path_buf = path.display().to_string();
            if ui.text_edit_singleline(&mut path_buf).changed() {
                *path = std::path::PathBuf::from(path_buf);
                *mark_dirty = true;
            }
        }
        EditorStepConfig::SqlLoaderPar { config } => {
            render_sqlldr(ui, config, mark_dirty);
        }
        EditorStepConfig::Shell { config } => {
            render_shell(ui, config, mark_dirty);
        }
        EditorStepConfig::Extract { config } => {
            render_extract(ui, config, mark_dirty);
        }
        EditorStepConfig::Loop { .. } => {}
    }
}

/// target_db를 선택할 수 있는 공용 콤보박스를 렌더링한다.
fn render_target_db_picker(
    ui: &mut egui::Ui,
    target_db: &mut Option<String>,
    db_keys: &[String],
    mark_dirty: &mut bool,
    id_hint: &str,
) {
    ui.label("DB 타겟(target_db)");
    let selected_text = target_db
        .as_deref()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "(기본 DB 사용)".to_string());
    ui.push_id(format!("target_db_{id_hint}"), |ui| {
        egui::ComboBox::from_id_source("target_db_combo")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(target_db.is_none(), "(기본 DB 사용)")
                    .clicked()
                {
                    if target_db.is_some() {
                        *target_db = None;
                        *mark_dirty = true;
                    }
                }
                for key in db_keys {
                    let selected = target_db.as_deref() == Some(key.as_str());
                    if ui.selectable_label(selected, key).clicked() && !selected {
                        *target_db = Some(key.clone());
                        *mark_dirty = true;
                    }
                }
            });
    });
}

/// SQL*Loader 속성 UI를 렌더링한다.
fn render_sqlldr(
    ui: &mut egui::Ui,
    config: &mut crate::scenario::SqlLoaderParConfig,
    mark_dirty: &mut bool,
) {
    let mut control = config.control_file.display().to_string();
    ui.label("control 파일");
    if ui.text_edit_singleline(&mut control).changed() {
        config.control_file = control.into();
        *mark_dirty = true;
    }

    optional_path_field_ui(ui, "data 파일", &mut config.data_file, mark_dirty);
    optional_path_field_ui(ui, "log 파일", &mut config.log_file, mark_dirty);
    optional_path_field_ui(ui, "bad 파일", &mut config.bad_file, mark_dirty);
    optional_path_field_ui(ui, "discard 파일", &mut config.discard_file, mark_dirty);

    let mut conn = config.conn.clone().unwrap_or_default();
    ui.label("접속 문자열");
    if ui.text_edit_singleline(&mut conn).changed() {
        config.conn = if conn.is_empty() { None } else { Some(conn) };
        *mark_dirty = true;
    }
    ui.small("비워두면 SQLLDR_CONN 환경 변수를 사용합니다.");
}

/// 선택적 경로 필드를 렌더링한다.
fn optional_path_field_ui(
    ui: &mut egui::Ui,
    label: &str,
    path: &mut Option<std::path::PathBuf>,
    mark_dirty: &mut bool,
) {
    ui.label(label);
    let mut buf = path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    if ui.text_edit_singleline(&mut buf).changed() {
        let trimmed = buf.trim();
        if trimmed.is_empty() {
            *path = None;
        } else {
            *path = Some(std::path::PathBuf::from(trimmed));
        }
        *mark_dirty = true;
    }
}

/// Shell 속성 UI를 렌더링한다.
fn render_shell(
    ui: &mut egui::Ui,
    config: &mut crate::scenario::ShellConfig,
    mark_dirty: &mut bool,
) {
    ui.label("스크립트");
    if ui.text_edit_multiline(&mut config.script).changed() {
        *mark_dirty = true;
    }

    let mut program = config.shell_program.clone().unwrap_or_default();
    ui.label("셸 프로그램");
    if ui.text_edit_singleline(&mut program).changed() {
        config.shell_program = if program.is_empty() {
            None
        } else {
            Some(program)
        };
        *mark_dirty = true;
    }

    let mut args = config.shell_args.join(", ");
    ui.label("인자 목록(쉼표 구분)");
    if ui.text_edit_singleline(&mut args).changed() {
        config.shell_args = args
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        *mark_dirty = true;
    }

    let mut work_dir = config
        .working_dir
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    ui.label("작업 디렉터리");
    if ui.text_edit_singleline(&mut work_dir).changed() {
        config.working_dir = if work_dir.is_empty() {
            None
        } else {
            Some(work_dir.into())
        };
        *mark_dirty = true;
    }

    let mut run_as = config.run_as.clone().unwrap_or_default();
    ui.label("실행 사용자");
    if ui.text_edit_singleline(&mut run_as).changed() {
        config.run_as = if run_as.is_empty() {
            None
        } else {
            Some(run_as)
        };
        *mark_dirty = true;
    }

    ui.label("환경 변수 (KEY=VALUE 한 줄씩)");
    let mut env_text = config
        .env
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("\n");
    if ui.text_edit_multiline(&mut env_text).changed() {
        config.env = parse_env(&env_text);
        *mark_dirty = true;
    }
}

/// Extract Step 속성 UI를 렌더링한다.
fn render_extract(ui: &mut egui::Ui, config: &mut ExtractVarFromFileConfig, mark_dirty: &mut bool) {
    ui.label("파일 경로");
    if ui.text_edit_singleline(&mut config.file_path).changed() {
        *mark_dirty = true;
    }
    let mut line = config.line as i32;
    if ui
        .add(egui::DragValue::new(&mut line).prefix("라인 "))
        .changed()
    {
        config.line = line.max(1) as usize;
        *mark_dirty = true;
    }
    ui.label("정규식 패턴");
    if ui.text_edit_singleline(&mut config.pattern).changed() {
        *mark_dirty = true;
    }
    let mut group = config.group as i32;
    if ui
        .add(egui::DragValue::new(&mut group).prefix("캡처 그룹 "))
        .changed()
    {
        config.group = group.max(0) as usize;
        *mark_dirty = true;
    }
    ui.label("저장할 변수명");
    if ui.text_edit_singleline(&mut config.var_name).changed() {
        *mark_dirty = true;
    }
}

/// Shell env 문자열을 파싱한다.
fn parse_env(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            trimmed
                .split_once('=')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        })
        .collect()
}
