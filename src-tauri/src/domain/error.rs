use serde::Serialize;
use ts_rs::TS;

/// Error único de aplicación: invariantes de dominio + fallos de repositorio.
/// Serializable para el frontend; tipo TS generado por ts-rs.
#[derive(Debug, thiserror::Error, Serialize, TS)]
#[ts(export, export_to = "../src/lib/bindings/")]
#[serde(tag = "kind", content = "detail")]
pub enum AppError {
    #[error("project name cannot be empty")]
    ProjectNameEmpty,
    #[error("task name cannot be empty")]
    TaskNameEmpty,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("repository error: {0}")]
    Repository(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages_are_stable() {
        assert_eq!(AppError::ProjectNameEmpty.to_string(), "project name cannot be empty");
        assert_eq!(AppError::NotFound("p1".into()).to_string(), "not found: p1");
    }
}
