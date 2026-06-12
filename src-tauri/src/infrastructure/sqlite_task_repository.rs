use crate::domain::error::AppError;
use crate::domain::ports::TaskRepository;
use crate::domain::task::Task;
use rusqlite::Connection;

/// Adaptador `SQLite` del puerto `TaskRepository`.
pub struct SqliteTaskRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteTaskRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

fn map_err(e: rusqlite::Error) -> AppError {
    AppError::Repository(e.to_string())
}

fn row_to_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get("id")?,
        project_id: row.get("project_id")?,
        name: row.get("name")?,
        created_at: row.get("created_at")?,
        completed: row.get::<_, i64>("completed")? != 0,
    })
}

impl TaskRepository for SqliteTaskRepository<'_> {
    fn add(&mut self, task: &Task) -> Result<(), AppError> {
        self.conn
            .execute(
                "INSERT INTO task (id, project_id, name, created_at, completed)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    task.id,
                    task.project_id,
                    task.name,
                    task.created_at,
                    i64::from(task.completed),
                ],
            )
            .map_err(map_err)?;
        Ok(())
    }

    fn list_by_project(&self, project_id: &str) -> Result<Vec<Task>, AppError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, project_id, name, created_at, completed
                 FROM task WHERE project_id = ?1 ORDER BY created_at",
            )
            .map_err(map_err)?;
        let rows = stmt.query_map([project_id], row_to_task).map_err(map_err)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(map_err)?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ports::{ProjectRepository, TaskRepository};
    use crate::domain::project::Project;
    use crate::infrastructure::db;
    use crate::infrastructure::sqlite_project_repository::SqliteProjectRepository;

    fn migrated() -> Connection {
        let mut conn = db::open_in_memory().unwrap();
        db::apply(&mut conn).unwrap();
        conn
    }

    fn seed_project(conn: &Connection, id: &str) {
        let mut pr = SqliteProjectRepository::new(conn);
        pr.add(&Project::new(id.into(), "P".into(), None, 1).unwrap())
            .unwrap();
    }

    #[test]
    fn add_then_list_by_project_roundtrip() {
        let conn = migrated();
        seed_project(&conn, "p1");
        let mut repo = SqliteTaskRepository::new(&conn);
        let t = Task::new("t1".into(), "p1".into(), "Disenar".into(), 1000).unwrap();
        repo.add(&t).unwrap();

        let all = repo.list_by_project("p1").unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0], t);

        assert!(repo.list_by_project("other").unwrap().is_empty());
    }

    #[test]
    fn add_fails_when_project_fk_missing() {
        let conn = migrated();
        let mut repo = SqliteTaskRepository::new(&conn);
        // project_id inexistente: la FK (foreign_keys ON) debe rechazar el insert.
        let t = Task::new("t1".into(), "ghost".into(), "X".into(), 0).unwrap();
        let r = repo.add(&t);
        assert!(
            matches!(r, Err(AppError::Repository(_))),
            "la FK debe rechazar un project_id inexistente"
        );
    }
}
