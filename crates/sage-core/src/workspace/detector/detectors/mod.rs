//! Language-specific detector modules

mod go;
mod jvm;
mod node;
mod other;
mod python;
mod rust;

use crate::workspace::detector::types::ProjectType;
use std::path::Path;

/// Run all detectors on a project
pub(super) fn detect_all(root: &Path, project: &mut ProjectType) {
    rust::detect(root, project);
    node::detect(root, project);
    python::detect(root, project);
    go::detect(root, project);
    jvm::detect(root, project);
    other::detect(root, project);
}
