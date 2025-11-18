mod app;
mod engine;
mod executor;
mod scenario;
mod theme;

use eframe::egui;
use app::BatchOrchestratorApp;

/// 메인 진입점으로 egui 애플리케이션을 실행한다.
fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("rust-airflow")
            .with_inner_size([1200.0, 720.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Rust Batch Orchestrator",
        native_options,
        Box::new(|cc| Box::new(BatchOrchestratorApp::new(cc))),
    )
}
