use crate::domain::error::AppError;
use crate::domain::ports::ProjectRepository;
use crate::domain::project::Project;
use rusqlite::Connection;

/// Adaptador SQLite del puerto ProjectRepository.
pub struct SqliteProjectRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteProjectRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

fn map_err(e: rusqlite::Error) -> AppError {
    AppError::Repository(e.to_string())
}

fn row_to_project(row: &rusqlite::Row) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get("id")?,
        name: row.get("name")?,
        color: row.get("color")?,
        created_at: row.get("created_at")?,
        archived: row.get::<_, i64>("archived")? != 0,
    })
}

impl<'a> ProjectRepository for SqliteProjectRepository<'a> {
    fn add(&mut self, project: &Project) -> Result<(), AppError> {
        self.conn
            .execute(
                "INSERT INTO project (id, name, color, created_at, archived)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    project.id,
                    project.name,
                    project.color,
                    project.created_at,
                    project.archived as i64,
                ],
            )
            .map_err(map_err)?;
        Ok(())
    }

    fn list(&self) -> Result<Vec<Project>, AppError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, color, created_at, archived FROM project ORDER BY created_at")
            .map_err(map_err)?;
        let rows = stmt.query_map([], row_to_project).map_err(map_err)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(map_err)?);
        }
        Ok(out)
    }

    fn find_by_id(&self, id: &str) -> Result<Option<Project>, AppError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, color, created_at, archived FROM project WHERE id = ?1")
            .map_err(map_err)?;
        let mut rows = stmt.query_map([id], row_to_project).map_err(map_err)?;
        match rows.next() {
            Some(r) => Ok(Some(r.map_err(map_err)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ports::ProjectRepository;
    use crate::infrastructure::db;

    fn migrated() -> Connection {
        let mut conn = db::open_in_memory().unwrap();
        db::apply(&mut conn).unwrap();
        conn
    }

    #[test]
    fn add_then_list_and_find_roundtrip() {
        let conn = migrated();
        let mut repo = SqliteProjectRepository::new(&conn);
        let p = Project::new("p1".into(), "Cliente A".into(), Some("#f00".into()), 1000).unwrap();
        repo.add(&p).unwrap();

        let all = repo.list().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0], p);

        let found = repo.find_by_id("p1").unwrap();
        assert_eq!(found, Some(p));
        assert_eq!(repo.find_by_id("nope").unwrap(), None);
    }

    #[test]
    fn empty_name_is_rejected_by_db_check() {
        let conn = migrated();
        // Insert directo saltando el dominio: el CHECK de la BD debe rechazarlo.
        let r = conn.execute(
            "INSERT INTO project (id, name, created_at) VALUES ('p1', '   ', 0)",
            [],
        );
        assert!(r.is_err(), "el CHECK length(trim(name))>0 debe rechazar nombre vacío");
    }
}
