use super::model::{EditorStepConfig, EditorStepNode, ScenarioEditorState, StepKind};
use crate::scenario::{ConfirmDefault, ExtractVarFromFileConfig, LoopIterationFailure};
use crate::theme::{BuilderColors, StepVisualKind, Theme, ThemeDecorations, ThemePalette};
use eframe::egui;
use eframe::epaint::{CubicBezierShape, Stroke};
use std::collections::HashMap;

mod canvas;
mod layout;
mod palette;
mod properties;

pub use layout::ScenarioBuilderUi;
