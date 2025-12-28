use egui::Color32;
use serde::{Deserialize, Serialize};

/// Tool selection enum
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DrawingTool {
    FreeDraw,
    Rectangle,
    Line,
    Arrow,
    Text,
}

/// Shape representation - stored in image coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DrawingShape {
    FreeDraw {
        points: Vec<(f32, f32)>,
        color: [u8; 4],
        thickness: f32,
    },
    Rectangle {
        start: (f32, f32),
        end: (f32, f32),
        color: [u8; 4],
        thickness: f32,
        filled: bool,
    },
    Line {
        start: (f32, f32),
        end: (f32, f32),
        color: [u8; 4],
        thickness: f32,
    },
    Arrow {
        start: (f32, f32),
        end: (f32, f32),
        color: [u8; 4],
        thickness: f32,
    },
    Text {
        position: (f32, f32),
        content: String,
        color: [u8; 4],
        font_size: f32,
    },
}

/// Drawing state for active stroke
#[derive(Debug, Clone)]
pub enum ActiveDrawing {
    FreeDraw { points: Vec<(f32, f32)> },
    Shape { start_pos: (f32, f32) },
    Text { position: (f32, f32), content: String },
    None,
}

/// Drawing settings
#[derive(Debug, Clone)]
pub struct DrawingSettings {
    pub color: Color32,
    pub thickness: f32,
    pub filled: bool,
    pub font_size: f32,
}

impl Default for DrawingSettings {
    fn default() -> Self {
        Self {
            color: Color32::RED,
            thickness: 2.0,
            filled: false,
            font_size: 16.0,
        }
    }
}

/// Undo/redo stack for managing drawing history
#[derive(Debug, Clone)]
pub struct UndoStack {
    past_states: Vec<Vec<DrawingShape>>,
    future_states: Vec<Vec<DrawingShape>>,
    max_history: usize,
}

impl UndoStack {
    pub fn new(max_history: usize) -> Self {
        Self {
            past_states: Vec::new(),
            future_states: Vec::new(),
            max_history,
        }
    }

    pub fn push(&mut self, state: Vec<DrawingShape>) {
        self.past_states.push(state);
        if self.past_states.len() > self.max_history {
            self.past_states.remove(0);
        }
        self.future_states.clear(); // Clear redo stack on new action
    }

    pub fn undo(&mut self, current: Vec<DrawingShape>) -> Option<Vec<DrawingShape>> {
        if let Some(prev) = self.past_states.pop() {
            self.future_states.push(current);
            Some(prev)
        } else {
            None
        }
    }

    pub fn redo(&mut self, current: Vec<DrawingShape>) -> Option<Vec<DrawingShape>> {
        if let Some(next) = self.future_states.pop() {
            self.past_states.push(current);
            Some(next)
        } else {
            None
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.past_states.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future_states.is_empty()
    }

    pub fn clear(&mut self) {
        self.past_states.clear();
        self.future_states.clear();
    }
}

/// Annotation layer for a single image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationLayer {
    pub image_path: String,
    pub shapes: Vec<DrawingShape>,
}

impl AnnotationLayer {
    pub fn new(image_path: String) -> Self {
        Self {
            image_path,
            shapes: Vec::new(),
        }
    }
}
