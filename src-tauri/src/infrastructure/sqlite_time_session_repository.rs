use crate::domain::error::AppError;
use crate::domain::ports::TimeSessionRepository;
use crate::domain::time_session::TimeSession;
use rusqlite::Connection;

/// Adaptador SQLite del puerto `TimeSessionRepository`.
pub struct SqliteTimeSessionRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteTimeSessionRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

fn map_err(e: rusqlite::Error) -> AppError {
    AppError::Repository(e.to_string())
}

fn row_to_session(row: &rusqlite::Row) -> rusqlite::Result<TimeSession> {
    Ok(TimeSession {
        id: row.get("id")?,
        task_id: row.get("task_id")?,
        started_at: row.get("started_at")?,
        ended_at: row.get("ended_at")?,
        last_heartbeat_at: row.get("last_heartbeat_at")?,
        is_suspect: row.get::<_, i64>("is_suspect")? != 0,
    })
}

impl TimeSessionRepository for SqliteTimeSessionRepository<'_> {
    fn running(&self) -> Result<Option<TimeSession>, AppError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, task_id, started_at, ended_at, last_heartbeat_at, is_suspect
                 FROM time_session WHERE ended_at IS NULL
                 ORDER BY started_at LIMIT 1",
            )
            .map_err(map_err)?;
        let mut rows = stmt.query_map([], row_to_session).map_err(map_err)?;
        match rows.next() {
            Some(r) => Ok(Some(r.map_err(map_err)?)),
            None => Ok(None),
        }
    }

    fn add(&mut self, session: &TimeSession) -> Result<(), AppError> {
        self.conn
            .execute(
                "INSERT INTO time_session
                   (id, task_id, started_at, ended_at, last_heartbeat_at, is_suspect)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    session.id,
                    session.task_id,
                    session.started_at,
                    session.ended_at,
                    session.last_heartbeat_at,
                    i64::from(session.is_suspect),
                ],
            )
            .map_err(map_err)?;
        Ok(())
    }

    fn update(&mut self, session: &TimeSession) -> Result<(), AppError> {
        self.conn
            .execute(
                "UPDATE time_session
                 SET ended_at = ?2, last_heartbeat_at = ?3, is_suspect = ?4
                 WHERE id = ?1",
                rusqlite::params![
                    session.id,
                    session.ended_at,
                    session.last_heartbeat_at,
                    i64::from(session.is_suspect),
                ],
            )
            .map_err(map_err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ports::{ProjectRepository, TaskRepository, TimeSessionRepository};
    use crate::domain::project::Project;
    use crate::domain::task::Task;
    use crate::infrastructure::db;
    use crate::infrastructure::sqlite_project_repository::SqliteProjectRepository;
    use crate::infrastructure::sqlite_task_repository::SqliteTaskRepository;

    fn migrated_with_task() -> Connection {
        let mut conn = db::open_in_memory().unwrap();
        db::apply(&mut conn).unwrap();
        {
            let mut pr = SqliteProjectRepository::new(&conn);
            pr.add(&Project::new("p1".into(), "P".into(), None, 1).unwrap()).unwrap();
            let mut tr = SqliteTaskRepository::new(&conn);
            tr.add(&Task::new("t1".into(), "p1".into(), "T".into(), 1).unwrap()).unwrap();
        }
        conn
    }

    #[test]
    fn add_then_running_roundtrip() {
        let conn = migrated_with_task();
        let mut repo = SqliteTimeSessionRepository::new(&conn);
        let s = TimeSession::start("s1".into(), "t1".into(), 1000);
        repo.add(&s).unwrap();

        let running = repo.running().unwrap();
        assert_eq!(running, Some(s));
    }

    #[test]
    fn update_closes_the_session() {
        let conn = migrated_with_task();
        let mut repo = SqliteTimeSessionRepository::new(&conn);
        let mut s = TimeSession::start("s1".into(), "t1".into(), 1000);
        repo.add(&s).unwrap();

        s.stop(5000).unwrap();
        repo.update(&s).unwrap();

        assert!(repo.running().unwrap().is_none(), "tras cerrar no debe haber sesión activa");
    }

    #[test]
    fn partial_index_rejects_second_open_session() {
        let conn = migrated_with_task();
        let mut repo = SqliteTimeSessionRepository::new(&conn);
        repo.add(&TimeSession::start("s1".into(), "t1".into(), 1000)).unwrap();
        let r = repo.add(&TimeSession::start("s2".into(), "t1".into(), 2000));
        assert!(
            matches!(r, Err(AppError::Repository(_))),
            "el índice único parcial debe impedir 2 sesiones abiertas"
        );
    }
}
