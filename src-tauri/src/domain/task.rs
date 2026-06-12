use crate::domain::error::AppError;

/// Entidad de dominio. Pertenece a un `Project` (`project_id`). Invariante: `name`
/// no vacío. Id = ULID (texto), generado fuera. `completed` arranca en false.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub created_at: i64, // epoch millis UTC
    pub completed: bool,
}

impl Task {
    /// Crea una tarea válida. Rechaza nombre vacío o sólo espacios.
    pub fn new(
        id: String,
        project_id: String,
        name: String,
        created_at: i64,
    ) -> Result<Self, AppError> {
        if name.trim().is_empty() {
            return Err(AppError::TaskNameEmpty);
        }
        Ok(Self { id, project_id, name, created_at, completed: false })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_empty_name() {
        let r = Task::new("t1".into(), "p1".into(), "  ".into(), 0);
        assert!(matches!(r, Err(AppError::TaskNameEmpty)));
    }

    #[test]
    fn new_accepts_valid_name_not_completed() {
        let t = Task::new("t1".into(), "p1".into(), "Disenar API".into(), 42).unwrap();
        assert_eq!(t.name, "Disenar API");
        assert_eq!(t.project_id, "p1");
        assert_eq!(t.created_at, 42);
        assert!(!t.completed);
    }
}
