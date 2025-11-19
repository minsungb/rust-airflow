use crate::editor::ScenarioBuilderUi;
use crate::engine::{ConfirmPhase, StepStatus};
use crate::scenario::ConfirmDefault;
use eframe::egui::{self, RichText};

use super::state::{AppTab, BatchOrchestratorApp};
use super::widgets::{PrimaryButton, StepCard, solid_section_header};

mod layout;
mod modal;
mod panels;
mod status;
mod toolbar;
