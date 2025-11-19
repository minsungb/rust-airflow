#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod editor;
mod engine;
mod executor;
mod scenario;
mod theme;

use app::BatchOrchestratorApp;
use eframe::egui;
use std::io::Cursor;

/// egui 애플리케이션을 초기화하고 실행하는 진입점입니다.
fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(load_icon_from_ico())
            .with_app_id("Rust Airflow")
            .with_inner_size([1200.0, 780.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "Rust Batch Orchestrator",
        native_options,
        Box::new(|cc| Box::new(BatchOrchestratorApp::new(cc))),
    )
}

/// 애플리케이션 아이콘을 ICO 파일에서 읽어 egui가 요구하는 포맷으로 변환합니다.
fn load_icon_from_ico() -> egui::IconData {
    // 가장 큰 엔트리를 골라 RGBA로 변환
    let bytes = include_bytes!("../icons/icon.ico");
    let dir = ico::IconDir::read(Cursor::new(bytes.as_slice())).expect("read ico");
    let entry = dir
        .entries()
        .iter()
        .max_by_key(|e| (e.width(), e.height()))
        .expect("ico empty");
    let img = entry.decode().expect("decode ico entry");
    let (w, h) = (img.width() as u32, img.height() as u32);
    let rgba = img.rgba_data().to_vec();
    egui::IconData {
        rgba,
        width: w,
        height: h,
    }
}
