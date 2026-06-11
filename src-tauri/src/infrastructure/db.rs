use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

/// Shared state injected by Tauri via `.manage(...)`.
pub struct Db(pub Mutex<Connection>);

/// Opens a file-based connection with production pragmas.
/// Pragmas are applied PER CONNECTION (they do not persist in the file).
pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(conn)
}

/// In-memory connection for tests. WAL does not apply in memory; only FK.
pub fn open_in_memory() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(conn)
}

use rusqlite_migration::{Migrations, M};
use std::sync::LazyLock;

static MIGRATIONS: LazyLock<Migrations<'static>> = LazyLock::new(|| {
    Migrations::new(vec![M::up(include_str!("../../migrations/0001_init.sql"))])
});

/// Applies all pending migrations up to the latest version.
pub fn apply(conn: &mut Connection) -> rusqlite_migration::Result<()> {
    MIGRATIONS.to_latest(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_enables_foreign_keys() {
        let conn = open_in_memory().expect("open in memory");
        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1, "foreign_keys debe estar ON");
    }

    #[test]
    fn open_file_sets_wal_and_foreign_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        let conn = open(&path).expect("open file");
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
        assert_eq!(fk, 1);
    }

    fn migrated_in_memory() -> Connection {
        let mut conn = open_in_memory().unwrap();
        apply(&mut conn).expect("apply migrations");
        conn
    }

    #[test]
    fn migration_creates_core_tables() {
        let conn = migrated_in_memory();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type='table' AND name IN ('project','task','time_session')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn db_rejects_second_running_session() {
        let conn = migrated_in_memory();
        conn.execute("INSERT INTO project(id, name, created_at) VALUES ('p1', 'p', 0)", []).unwrap();
        conn.execute("INSERT INTO task(id, project_id, name, created_at) VALUES ('t1', 'p1', 't', 0)", []).unwrap();
        conn.execute("INSERT INTO time_session(id, task_id, started_at) VALUES ('s1', 't1', 100)", []).unwrap();
        let second = conn.execute("INSERT INTO time_session(id, task_id, started_at) VALUES ('s2', 't1', 200)", []);
        assert!(second.is_err(), "una 2ª sesión abierta debe ser rechazada por el índice único parcial");
    }

    #[test]
    fn db_rejects_negative_duration() {
        let conn = migrated_in_memory();
        conn.execute("INSERT INTO project(id, name, created_at) VALUES ('p1', 'p', 0)", []).unwrap();
        conn.execute("INSERT INTO task(id, project_id, name, created_at) VALUES ('t1', 'p1', 't', 0)", []).unwrap();
        let bad = conn.execute(
            "INSERT INTO time_session(id, task_id, started_at, ended_at) VALUES ('s1', 't1', 200, 100)",
            [],
        );
        assert!(bad.is_err(), "ended_at < started_at debe violar el CHECK");
    }
}
