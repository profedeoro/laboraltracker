use crate::domain::error::AppError;

/// Entidad de dominio. Invariante: `name` no vacío. Id = ULID (texto), generado fuera.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub created_at: i64, // epoch millis UTC
    pub archived: bool,
}

impl Project {
    /// Crea un proyecto válido. Rechaza nombre vacío o sólo espacios.
    pub fn new(
        id: String,
        name: String,
        color: Option<String>,
        created_at: i64,
    ) -> Result<Self, AppError> {
        if name.trim().is_empty() {
            return Err(AppError::ProjectNameEmpty);
        }
        Ok(Self { id, name, color, created_at, archived: false })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_empty_name() {
        let r = Project::new("p1".into(), "   ".into(), None, 0);
        assert!(matches!(r, Err(AppError::ProjectNameEmpty)));
    }

    #[test]
    fn new_accepts_valid_name_not_archived() {
        let p = Project::new("p1".into(), "Cliente A".into(), None, 123).unwrap();
        assert_eq!(p.name, "Cliente A");
        assert_eq!(p.created_at, 123);
        assert!(!p.archived);
    }
}
