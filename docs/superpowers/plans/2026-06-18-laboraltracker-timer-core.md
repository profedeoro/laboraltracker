# LaboralTracker — Plan 4: Cronómetro core (start/stop)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Iniciar y parar el cronómetro de una tarea de punta a punta, con **una sola sesión activa global**, persistida en SQLite, y un cronómetro **en vivo** en la UI. Aquí se empiezan a usar la tabla `time_session` y el índice único parcial creados en Plan 1.

**Architecture:** Hexagonal (igual que Plans 2-3). Diseño: [2026-06-18-cronometro-design.md](../specs/2026-06-18-cronometro-design.md). Reglas de tiempo: [02-time-policy.md](../../conventions/02-time-policy.md). El dominio no conoce `rusqlite` ni Tauri. `StartTimerUseCase` es dueño del invariante de sesión única: cierra la activa antes de abrir otra; el índice único parcial es defensa en profundidad.

**Tech Stack:** Rust (`rusqlite`, `ulid`, `thiserror`, `ts-rs`), SQLite, Svelte 5 + TS. Sin dependencias nuevas.

**Reglas de entorno (Windows):** en PowerShell, antes de cualquier `cargo`, ejecutar `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`. Tests: `cargo test --manifest-path src-tauri\Cargo.toml`. Gate de calidad en cada tarea: `cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets` sin lints `clippy::*` (el `dead_code` de ítems aún no cableados es aceptable hasta la tarea de wiring, donde sí debe pasar `-- -D warnings`). Commits **conventional**, **sin** `Co-Authored-By`.

**Fuera de alcance (Plan 5 y 6):** vista "hoy"/agregación → Plan 5. Recuperación de huérfanas + heartbeat → Plan 6. (El campo `last_heartbeat_at` existe en el esquema y en la entidad, pero en Plan 4 nadie lo escribe todavía.)

---

## File structure (lo que este plan crea/modifica)

```txt
src-tauri/
├── src/
│   ├── domain/
│   │   ├── mod.rs                            # + pub mod time_session
│   │   ├── error.rs                          # + ClockWentBackwards, NoRunningSession
│   │   ├── time_session.rs                   # TimeSession (start, stop)              [NUEVO]
│   │   └── ports.rs                          # + TimeSessionRepository; + TaskRepository::find_by_id
│   ├── application/
│   │   ├── mod.rs                            # + pub mod start_timer/stop_timer
│   │   ├── testing.rs                        # + InMemoryTimeSessionRepository; TaskRepo find_by_id
│   │   ├── start_timer.rs                    # StartTimerUseCase                       [NUEVO]
│   │   └── stop_timer.rs                     # StopTimerUseCase                        [NUEVO]
│   ├── infrastructure/
│   │   ├── mod.rs                            # + pub mod sqlite_time_session_repository
│   │   ├── sqlite_task_repository.rs         # + impl find_by_id
│   │   └── sqlite_time_session_repository.rs # SqliteTimeSessionRepository            [NUEVO]
│   ├── presentation/
│   │   ├── dto.rs                            # + TimeSessionDto
│   │   └── commands.rs                       # + start_timer, stop_timer, running_timer
│   └── lib.rs                                # registra los 3 comandos nuevos
└── src/lib/                                  # (frontend)
    ├── bindings/TimeSessionDto.ts            # generado por ts-rs
    ├── api/timer.ts                          # wrappers invoke tipados                 [NUEVO]
    └── stores/timer.ts                       # store del cronómetro                    [NUEVO]
src/routes/+page.svelte                       # botones Iniciar/Parar + cronómetro en vivo
CHANGELOG.md
```

---

## Task 1: Dominio — `TimeSession` + variantes de error

**Files:**
- Modify: `src-tauri/src/domain/error.rs`
- Create: `src-tauri/src/domain/time_session.rs`
- Modify: `src-tauri/src/domain/mod.rs`

- [ ] **Step 1: Añadir variantes a `AppError`**

En `src-tauri/src/domain/error.rs`, dentro de `enum AppError`, añadir estas dos variantes (después de `Repository(String)` o donde encajen):
```rust
    #[error("clock went backwards: now is before the session start")]
    ClockWentBackwards,
    #[error("no running session")]
    NoRunningSession,
```

- [ ] **Step 2: Declarar el módulo**

En `src-tauri/src/domain/mod.rs`, añadir al final:
```rust
pub mod time_session;
```

- [ ] **Step 3: Escribir la entidad con tests (TDD)**

Create `src-tauri/src/domain/time_session.rs`:
```rust
use crate::domain::error::AppError;

/// Cap de duración (configurable a futuro). Una sesión más larga se marca sospechosa.
const MAX_SESSION_MS: i64 = 12 * 60 * 60 * 1000; // 12 h

/// Entidad de dominio: una sesión de cronómetro sobre una tarea.
/// Instantes en epoch-millis UTC. Duración derivada, nunca persistida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeSession {
    pub id: String,
    pub task_id: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub last_heartbeat_at: Option<i64>,
    pub is_suspect: bool,
}

impl TimeSession {
    /// Abre una sesión nueva (en curso). Id = ULID (texto), generado fuera.
    pub fn start(id: String, task_id: String, now: i64) -> Self {
        Self {
            id,
            task_id,
            started_at: now,
            ended_at: None,
            last_heartbeat_at: None,
            is_suspect: false,
        }
    }

    /// Cierra la sesión en `now`. Rechaza reloj retrocedido (`now < started_at`).
    /// Si la duración supera el cap, marca la sesión como sospechosa.
    pub fn stop(&mut self, now: i64) -> Result<(), AppError> {
        if now < self.started_at {
            return Err(AppError::ClockWentBackwards);
        }
        self.ended_at = Some(now);
        if now - self.started_at > MAX_SESSION_MS {
            self.is_suspect = true;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_is_open_and_not_suspect() {
        let s = TimeSession::start("s1".into(), "t1".into(), 1000);
        assert_eq!(s.started_at, 1000);
        assert_eq!(s.ended_at, None);
        assert!(!s.is_suspect);
        assert_eq!(s.last_heartbeat_at, None);
    }

    #[test]
    fn stop_sets_ended_at() {
        let mut s = TimeSession::start("s1".into(), "t1".into(), 1000);
        s.stop(5000).unwrap();
        assert_eq!(s.ended_at, Some(5000));
        assert!(!s.is_suspect);
    }

    #[test]
    fn stop_rejects_clock_backwards_and_leaves_session_open() {
        let mut s = TimeSession::start("s1".into(), "t1".into(), 1000);
        let r = s.stop(999);
        assert!(matches!(r, Err(AppError::ClockWentBackwards)));
        assert_eq!(s.ended_at, None, "no debe cerrar si el reloj retrocedió");
    }

    #[test]
    fn stop_marks_suspect_when_over_cap() {
        let mut s = TimeSession::start("s1".into(), "t1".into(), 0);
        let over = MAX_SESSION_MS + 1;
        s.stop(over).unwrap();
        assert_eq!(s.ended_at, Some(over));
        assert!(s.is_suspect, "una sesión > 12 h debe marcarse sospechosa");
    }
}
```

- [ ] **Step 4: Ejecutar los tests → deben pasar**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo test --manifest-path src-tauri\Cargo.toml domain::time_session`
Expected: 4 passed.

- [ ] **Step 5: Clippy (plain) + commit**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets`
Expected: sin lints `clippy::*` en el código nuevo (el `dead_code`/variantes sin usar es aceptable hasta el wiring). Luego:
```bash
git add src-tauri/
git commit -m "feat(domain): TimeSession entity with start/stop and clock-safety invariants"
```

---

## Task 2: Puertos — `TimeSessionRepository` + `TaskRepository::find_by_id`

> `StartTimerUseCase` (Task 3) necesita verificar que la tarea exista. `TaskRepository`
> hoy sólo tiene `add` y `list_by_project`; le agregamos `find_by_id`. Como es un
> método de trait, hay que implementarlo en TODAS las impls en el mismo commit
> (in-memory + SQLite) para que compile.

**Files:**
- Modify: `src-tauri/src/domain/ports.rs`
- Modify: `src-tauri/src/application/testing.rs`
- Modify: `src-tauri/src/infrastructure/sqlite_task_repository.rs`

- [ ] **Step 1: Extender los puertos en `src-tauri/src/domain/ports.rs`**

1. Añadir el método `find_by_id` al trait `TaskRepository` existente:
```rust
    fn find_by_id(&self, id: &str) -> Result<Option<Task>, AppError>;
```
(dentro de `pub trait TaskRepository { ... }`, junto a `add` y `list_by_project`.)

2. Al final del archivo, añadir el nuevo puerto:
```rust
use crate::domain::time_session::TimeSession;

/// Puerto de persistencia de sesiones de cronómetro.
pub trait TimeSessionRepository {
    /// La sesión actualmente en curso (`ended_at IS NULL`), si existe.
    fn running(&self) -> Result<Option<TimeSession>, AppError>;
    fn add(&mut self, session: &TimeSession) -> Result<(), AppError>;
    fn update(&mut self, session: &TimeSession) -> Result<(), AppError>;
}
```

- [ ] **Step 2: Implementar `find_by_id` en el doble en memoria**

En `src-tauri/src/application/testing.rs`, dentro de `impl TaskRepository for InMemoryTaskRepository`, añadir:
```rust
    fn find_by_id(&self, id: &str) -> Result<Option<Task>, AppError> {
        Ok(self.items.iter().find(|t| t.id == id).cloned())
    }
```

- [ ] **Step 3: Implementar `find_by_id` en el adaptador SQLite**

En `src-tauri/src/infrastructure/sqlite_task_repository.rs`, dentro de `impl TaskRepository for SqliteTaskRepository<'_>`, añadir (reutiliza el `row_to_task` y `map_err` ya existentes en el archivo):
```rust
    fn find_by_id(&self, id: &str) -> Result<Option<Task>, AppError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, project_id, name, created_at, completed
                 FROM task WHERE id = ?1",
            )
            .map_err(map_err)?;
        let mut rows = stmt.query_map([id], row_to_task).map_err(map_err)?;
        match rows.next() {
            Some(r) => Ok(Some(r.map_err(map_err)?)),
            None => Ok(None),
        }
    }
```

- [ ] **Step 4: Añadir un test del `find_by_id` SQLite**

En el `mod tests` de `src-tauri/src/infrastructure/sqlite_task_repository.rs`, añadir:
```rust
    #[test]
    fn find_by_id_returns_task_or_none() {
        let conn = migrated();
        seed_project(&conn, "p1");
        let mut repo = SqliteTaskRepository::new(&conn);
        repo.add(&Task::new("t1".into(), "p1".into(), "A".into(), 1).unwrap())
            .unwrap();
        assert_eq!(repo.find_by_id("t1").unwrap().map(|t| t.name), Some("A".to_string()));
        assert_eq!(repo.find_by_id("nope").unwrap(), None);
    }
```

- [ ] **Step 5: Ejecutar tests + verificar build**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo test --manifest-path src-tauri\Cargo.toml sqlite_task_repository`
Expected: tests del repo de tareas en verde (incluido `find_by_id_returns_task_or_none`).
Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo build --manifest-path src-tauri\Cargo.toml`
Expected: `Finished` (warnings `dead_code` de `TimeSessionRepository`/`find_by_id` aún sin cablear son aceptables).

- [ ] **Step 6: Clippy (plain) + commit**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets`
Expected: sin lints `clippy::*`. Luego:
```bash
git add src-tauri/
git commit -m "feat(domain): TimeSessionRepository port and TaskRepository::find_by_id"
```

---

## Task 3: Caso de uso `StartTimer` (+ doble en memoria de sesiones)

**Files:**
- Modify: `src-tauri/src/application/mod.rs`
- Modify: `src-tauri/src/application/testing.rs`
- Create: `src-tauri/src/application/start_timer.rs`

- [ ] **Step 1: Declarar los módulos**

En `src-tauri/src/application/mod.rs`, añadir:
```rust
pub mod start_timer;
pub mod stop_timer;
```
NOTA: `stop_timer` se implementa en Task 4. Crear placeholder `src-tauri/src/application/stop_timer.rs` con solo `// implemented in Task 4` para que compile; se sobreescribe luego.

- [ ] **Step 2: Añadir `InMemoryTimeSessionRepository` a `testing.rs`**

En `src-tauri/src/application/testing.rs`:
1. Asegurar que los imports incluyan el puerto y la entidad. El bloque de imports debe contener:
```rust
use crate::domain::error::AppError;
use crate::domain::ports::{Clock, ProjectRepository, TaskRepository, TimeSessionRepository};
use crate::domain::project::Project;
use crate::domain::task::Task;
use crate::domain::time_session::TimeSession;
```
2. Al final del archivo, añadir:
```rust
/// Repositorio de sesiones en memoria para tests de casos de uso (sin SQLite).
#[derive(Default)]
pub struct InMemoryTimeSessionRepository {
    pub items: Vec<TimeSession>,
}

impl TimeSessionRepository for InMemoryTimeSessionRepository {
    fn running(&self) -> Result<Option<TimeSession>, AppError> {
        Ok(self.items.iter().find(|s| s.ended_at.is_none()).cloned())
    }
    fn add(&mut self, session: &TimeSession) -> Result<(), AppError> {
        self.items.push(session.clone());
        Ok(())
    }
    fn update(&mut self, session: &TimeSession) -> Result<(), AppError> {
        match self.items.iter_mut().find(|s| s.id == session.id) {
            Some(slot) => {
                *slot = session.clone();
                Ok(())
            }
            None => Err(AppError::NotFound(session.id.clone())),
        }
    }
}
```

- [ ] **Step 3: Escribir el caso de uso con tests (TDD)**

Create `src-tauri/src/application/start_timer.rs`:
```rust
use crate::domain::error::AppError;
use crate::domain::ports::{Clock, TaskRepository, TimeSessionRepository};
use crate::domain::time_session::TimeSession;

/// Caso de uso: iniciar el cronómetro de una tarea. Dueño del invariante de sesión
/// única: si hay una sesión corriendo, la cierra antes de abrir la nueva.
pub struct StartTimerUseCase;

impl StartTimerUseCase {
    pub fn execute(
        sessions: &mut impl TimeSessionRepository,
        tasks: &impl TaskRepository,
        clock: &impl Clock,
        task_id: String,
    ) -> Result<TimeSession, AppError> {
        if tasks.find_by_id(&task_id)?.is_none() {
            return Err(AppError::NotFound(task_id));
        }
        let now = clock.now();
        // Cerrar la sesión activa (si la hay) antes de abrir otra.
        if let Some(mut running) = sessions.running()? {
            running.stop(now)?;
            sessions.update(&running)?;
        }
        let session = TimeSession::start(ulid::Ulid::new().to_string(), task_id, now);
        sessions.add(&session)?;
        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::testing::{
        FixedClock, InMemoryTaskRepository, InMemoryTimeSessionRepository,
    };
    use crate::domain::task::Task;

    fn tasks_with_t1() -> InMemoryTaskRepository {
        let mut r = InMemoryTaskRepository::default();
        r.items
            .push(Task::new("t1".into(), "p1".into(), "Tarea 1".into(), 1).unwrap());
        r.items
            .push(Task::new("t2".into(), "p1".into(), "Tarea 2".into(), 2).unwrap());
        r
    }

    #[test]
    fn starts_a_session_when_none_running() {
        let mut sessions = InMemoryTimeSessionRepository::default();
        let tasks = tasks_with_t1();
        let clock = FixedClock(1000);
        let s = StartTimerUseCase::execute(&mut sessions, &tasks, &clock, "t1".into()).unwrap();
        assert_eq!(s.task_id, "t1");
        assert_eq!(s.started_at, 1000);
        assert_eq!(s.ended_at, None);
        assert_eq!(sessions.items.len(), 1);
    }

    #[test]
    fn starting_another_closes_the_running_one() {
        let mut sessions = InMemoryTimeSessionRepository::default();
        let tasks = tasks_with_t1();

        let first = StartTimerUseCase::execute(&mut sessions, &tasks, &FixedClock(1000), "t1".into()).unwrap();
        let second = StartTimerUseCase::execute(&mut sessions, &tasks, &FixedClock(2000), "t2".into()).unwrap();

        assert_eq!(sessions.items.len(), 2);
        // exactamente una sesión corriendo, y es la segunda
        let running: Vec<_> = sessions.items.iter().filter(|s| s.ended_at.is_none()).collect();
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].id, second.id);
        // la primera quedó cerrada en now=2000
        let closed = sessions.items.iter().find(|s| s.id == first.id).unwrap();
        assert_eq!(closed.ended_at, Some(2000));
    }

    #[test]
    fn rejects_start_on_missing_task() {
        let mut sessions = InMemoryTimeSessionRepository::default();
        let tasks = tasks_with_t1();
        let r = StartTimerUseCase::execute(&mut sessions, &tasks, &FixedClock(0), "ghost".into());
        assert!(matches!(r, Err(AppError::NotFound(id)) if id == "ghost"));
        assert_eq!(sessions.items.len(), 0);
    }
}
```

- [ ] **Step 4: Ejecutar → deben pasar**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo test --manifest-path src-tauri\Cargo.toml application::start_timer`
Expected: 3 passed.

- [ ] **Step 5: Clippy (plain) + commit**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets`
Expected: sin lints `clippy::*`. Luego:
```bash
git add src-tauri/
git commit -m "feat(application): StartTimerUseCase enforcing the single-active-session invariant"
```

---

## Task 4: Caso de uso `StopTimer`

**Files:**
- Overwrite: `src-tauri/src/application/stop_timer.rs` (placeholder de Task 3)

- [ ] **Step 1: Escribir el caso de uso con tests (TDD)**

Overwrite `src-tauri/src/application/stop_timer.rs`:
```rust
use crate::domain::error::AppError;
use crate::domain::ports::{Clock, TimeSessionRepository};
use crate::domain::time_session::TimeSession;

/// Caso de uso: parar la sesión activa. Error si no hay ninguna.
pub struct StopTimerUseCase;

impl StopTimerUseCase {
    pub fn execute(
        sessions: &mut impl TimeSessionRepository,
        clock: &impl Clock,
    ) -> Result<TimeSession, AppError> {
        let mut running = sessions.running()?.ok_or(AppError::NoRunningSession)?;
        running.stop(clock.now())?;
        sessions.update(&running)?;
        Ok(running)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::testing::{FixedClock, InMemoryTimeSessionRepository};
    use crate::domain::time_session::TimeSession;

    #[test]
    fn stops_the_running_session() {
        let mut sessions = InMemoryTimeSessionRepository::default();
        sessions.items.push(TimeSession::start("s1".into(), "t1".into(), 1000));
        let stopped = StopTimerUseCase::execute(&mut sessions, &FixedClock(4000)).unwrap();
        assert_eq!(stopped.ended_at, Some(4000));
        assert!(sessions.running().unwrap().is_none(), "no debe quedar sesión activa");
    }

    #[test]
    fn errors_when_no_running_session() {
        let mut sessions = InMemoryTimeSessionRepository::default();
        let r = StopTimerUseCase::execute(&mut sessions, &FixedClock(0));
        assert!(matches!(r, Err(AppError::NoRunningSession)));
    }
}
```

- [ ] **Step 2: Ejecutar → deben pasar**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo test --manifest-path src-tauri\Cargo.toml application::stop_timer`
Expected: 2 passed.

- [ ] **Step 3: Clippy (plain) + commit**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets`
Expected: sin lints `clippy::*`. Luego:
```bash
git add src-tauri/
git commit -m "feat(application): StopTimerUseCase"
```

---

## Task 5: Repositorio SQLite de sesiones

**Files:**
- Modify: `src-tauri/src/infrastructure/mod.rs`
- Create: `src-tauri/src/infrastructure/sqlite_time_session_repository.rs`

- [ ] **Step 1: Declarar el módulo**

En `src-tauri/src/infrastructure/mod.rs`, añadir:
```rust
pub mod sqlite_time_session_repository;
```

- [ ] **Step 2: Escribir el repositorio con tests (TDD)**

Create `src-tauri/src/infrastructure/sqlite_time_session_repository.rs`:
```rust
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
        // Segunda sesión abierta (otra id) → el índice único parcial debe rechazarla.
        let r = repo.add(&TimeSession::start("s2".into(), "t1".into(), 2000));
        assert!(
            matches!(r, Err(AppError::Repository(_))),
            "el índice único parcial debe impedir 2 sesiones abiertas"
        );
    }
}
```

- [ ] **Step 3: Ejecutar → deben pasar**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo test --manifest-path src-tauri\Cargo.toml infrastructure::sqlite_time_session_repository`
Expected: 3 passed.

- [ ] **Step 4: Clippy (plain) + commit**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets`
Expected: sin lints `clippy::*`. Luego:
```bash
git add src-tauri/
git commit -m "feat(infra): SqliteTimeSessionRepository with partial-index defense"
```

---

## Task 6: DTO + comandos Tauri + wiring (gate estricto)

**Files:**
- Modify: `src-tauri/src/presentation/dto.rs`
- Modify: `src-tauri/src/presentation/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Añadir `TimeSessionDto` a `src-tauri/src/presentation/dto.rs`**

1. Tras los `use` de dominio existentes, añadir:
```rust
use crate::domain::time_session::TimeSession;
```
2. Al final del archivo:
```rust
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct TimeSessionDto {
    pub id: String,
    pub task_id: String,
    #[ts(type = "number")]
    pub started_at: i64,
    #[ts(type = "number | null")]
    pub ended_at: Option<i64>,
    #[ts(type = "number | null")]
    pub last_heartbeat_at: Option<i64>,
    pub is_suspect: bool,
}

impl From<TimeSession> for TimeSessionDto {
    fn from(s: TimeSession) -> Self {
        Self {
            id: s.id,
            task_id: s.task_id,
            started_at: s.started_at,
            ended_at: s.ended_at,
            last_heartbeat_at: s.last_heartbeat_at,
            is_suspect: s.is_suspect,
        }
    }
}
```

- [ ] **Step 2: Añadir los comandos a `src-tauri/src/presentation/commands.rs`**

1. `use` nuevos (no dupliques `Db`, `AppError`, `SystemClock`, `SqliteTaskRepository` si ya están):
```rust
use crate::application::start_timer::StartTimerUseCase;
use crate::application::stop_timer::StopTimerUseCase;
use crate::infrastructure::sqlite_time_session_repository::SqliteTimeSessionRepository;
use crate::presentation::dto::TimeSessionDto;
```
2. Al final del archivo:
```rust
#[tauri::command]
pub fn start_timer(task_id: String, db: tauri::State<Db>) -> Result<TimeSessionDto, AppError> {
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::Repository("db mutex poisoned".into()))?;
    let tasks = SqliteTaskRepository::new(&conn);
    let mut sessions = SqliteTimeSessionRepository::new(&conn);
    let session = StartTimerUseCase::execute(&mut sessions, &tasks, &SystemClock, task_id)?;
    Ok(TimeSessionDto::from(session))
}

#[tauri::command]
pub fn stop_timer(db: tauri::State<Db>) -> Result<TimeSessionDto, AppError> {
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::Repository("db mutex poisoned".into()))?;
    let mut sessions = SqliteTimeSessionRepository::new(&conn);
    let session = StopTimerUseCase::execute(&mut sessions, &SystemClock)?;
    Ok(TimeSessionDto::from(session))
}

#[tauri::command]
pub fn running_timer(db: tauri::State<Db>) -> Result<Option<TimeSessionDto>, AppError> {
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::Repository("db mutex poisoned".into()))?;
    let sessions = SqliteTimeSessionRepository::new(&conn);
    Ok(sessions.running()?.map(TimeSessionDto::from))
}
```

- [ ] **Step 3: Registrar en `src-tauri/src/lib.rs`**

Reemplazar el `invoke_handler` por:
```rust
        .invoke_handler(tauri::generate_handler![
            health,
            presentation::commands::create_project,
            presentation::commands::list_projects,
            presentation::commands::create_task,
            presentation::commands::list_tasks,
            presentation::commands::start_timer,
            presentation::commands::stop_timer,
            presentation::commands::running_timer
        ])
```

- [ ] **Step 4: Verificar — build, todos los tests, y clippy ESTRICTO**

Run, en orden:
1. `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo test --manifest-path src-tauri\Cargo.toml` → todos en verde.
2. `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo build --manifest-path src-tauri\Cargo.toml` → **0 warnings**.
3. `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings` → **exit 0**.

Si queda algún `dead_code`, algo no quedó cableado (revisá el `invoke_handler` y los `use`). No tapar con `allow`.
> ts-rs puede regenerar `.ts` bajo `src/lib/bindings/` al testear. NO los commitees acá; van en Task 7. `git add src-tauri/` solamente.

- [ ] **Step 5: Commit**
```bash
git add src-tauri/
git commit -m "feat(presentation): TimeSessionDto and start/stop/running timer commands"
```

---

## Task 7: Exportar el tipo TS de `TimeSessionDto`

**Files:**
- Create (generado): `src/lib/bindings/TimeSessionDto.ts`

- [ ] **Step 1: Generar**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo test --manifest-path src-tauri\Cargo.toml export_bindings`
Expected: `export_bindings_*` PASS (incluido `export_bindings_timesessiondto`).

- [ ] **Step 2: Verificar contenido y ubicación**

Run (PowerShell): `Get-Content src\lib\bindings\TimeSessionDto.ts`
Expected: tipo con `id`, `taskId`, `startedAt: number`, `endedAt: number | null`, `lastHeartbeatAt: number | null`, `isSuspect: boolean`. En `<repo>/src/lib/bindings/` (no dentro de `src-tauri/`).

- [ ] **Step 3: Commit**
```bash
git add src/lib/bindings/TimeSessionDto.ts
git commit -m "chore(bindings): generate TimeSessionDto TS type via ts-rs"
```

---

## Task 8: Frontend — API, store y UI con cronómetro en vivo

**Files:**
- Create: `src/lib/api/timer.ts`
- Create: `src/lib/stores/timer.ts`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Wrapper `invoke` tipado**

Create `src/lib/api/timer.ts`:
```ts
import { invoke } from '@tauri-apps/api/core';
import type { TimeSessionDto } from '$lib/bindings/TimeSessionDto';

export function startTimer(taskId: string): Promise<TimeSessionDto> {
  return invoke<TimeSessionDto>('start_timer', { taskId });
}

export function stopTimer(): Promise<TimeSessionDto> {
  return invoke<TimeSessionDto>('stop_timer');
}

export function runningTimer(): Promise<TimeSessionDto | null> {
  return invoke<TimeSessionDto | null>('running_timer');
}
```

- [ ] **Step 2: Store del cronómetro**

Create `src/lib/stores/timer.ts`:
```ts
import { writable } from 'svelte/store';
import type { TimeSessionDto } from '$lib/bindings/TimeSessionDto';
import * as api from '$lib/api/timer';

/** Sesión actualmente en curso, o null si el cronómetro está parado. */
export const running = writable<TimeSessionDto | null>(null);

export async function refreshRunning(): Promise<void> {
  running.set(await api.runningTimer());
}

export async function start(taskId: string): Promise<void> {
  running.set(await api.startTimer(taskId));
}

export async function stop(): Promise<void> {
  await api.stopTimer();
  running.set(null);
}
```

- [ ] **Step 3: UI con botones y transcurrido en vivo**

Replace `src/routes/+page.svelte`:
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { projects, refreshProjects, addProject } from '$lib/stores/projects';
  import { tasks, selectedProjectId, loadTasks, addTask } from '$lib/stores/tasks';
  import { running, refreshRunning, start, stop } from '$lib/stores/timer';

  let projectName = $state('');
  let taskName = $state('');
  let error = $state('');
  // Reloj de presentación: tickea cada segundo para el transcurrido en vivo.
  let now = $state(Date.now());

  onMount(() => {
    refreshProjects();
    refreshRunning();
    const id = setInterval(() => {
      now = Date.now();
    }, 1000);
    return () => clearInterval(id);
  });

  function formatElapsed(ms: number): string {
    const total = Math.max(0, Math.floor(ms / 1000));
    const hh = String(Math.floor(total / 3600)).padStart(2, '0');
    const mm = String(Math.floor((total % 3600) / 60)).padStart(2, '0');
    const ss = String(total % 60).padStart(2, '0');
    return `${hh}:${mm}:${ss}`;
  }

  async function submitProject(e: Event) {
    e.preventDefault();
    error = '';
    try {
      await addProject(projectName, null);
      projectName = '';
    } catch (err) {
      error = JSON.stringify(err);
    }
  }

  async function selectProject(id: string) {
    error = '';
    try {
      await loadTasks(id);
    } catch (err) {
      error = JSON.stringify(err);
    }
  }

  async function submitTask(e: Event) {
    e.preventDefault();
    error = '';
    const pid = $selectedProjectId;
    if (!pid) return;
    try {
      await addTask(pid, taskName);
      taskName = '';
    } catch (err) {
      error = JSON.stringify(err);
    }
  }

  async function startTask(taskId: string) {
    error = '';
    try {
      await start(taskId);
    } catch (err) {
      error = JSON.stringify(err);
    }
  }

  async function stopTask() {
    error = '';
    try {
      await stop();
    } catch (err) {
      error = JSON.stringify(err);
    }
  }
</script>

<main style="padding: 2rem; font-family: system-ui; max-width: 40rem;">
  <h1>LaboralTracker — Proyectos y Tareas</h1>

  {#if $running}
    <p style="background:#ecfdf5; border:1px solid #16a34a; padding:.5rem .75rem; border-radius:.375rem;">
      ⏱ Cronómetro corriendo —
      <strong style="font-variant-numeric: tabular-nums;">{formatElapsed(now - $running.startedAt)}</strong>
      <button type="button" onclick={stopTask} style="margin-left:.5rem;">Parar</button>
    </p>
  {/if}

  <form onsubmit={submitProject} style="display: flex; gap: .5rem; margin: 1rem 0;">
    <input placeholder="Nombre del proyecto" bind:value={projectName} required />
    <button type="submit">Crear proyecto</button>
  </form>

  {#if error}<p style="color: crimson;">Error: {error}</p>{/if}

  <ul>
    {#each $projects as p (p.id)}
      <li>
        <button
          type="button"
          onclick={() => selectProject(p.id)}
          style="font-weight: {$selectedProjectId === p.id ? 'bold' : 'normal'};"
        >
          {p.name}
        </button>
        <small>{p.id}</small>
      </li>
    {/each}
  </ul>
  {#if $projects.length === 0}<p><em>Sin proyectos todavía.</em></p>{/if}

  {#if $selectedProjectId}
    {@const selected = $projects.find((p) => p.id === $selectedProjectId)}
    <hr style="margin: 1.5rem 0;" />
    <h2>Tareas de <span style="color: #2563eb;">{selected?.name ?? ''}</span></h2>
    <form onsubmit={submitTask} style="display: flex; gap: .5rem; margin: 1rem 0;">
      <input
        placeholder="Nueva tarea para {selected?.name ?? 'el proyecto'}"
        bind:value={taskName}
        required
      />
      <button type="submit">Crear tarea</button>
    </form>
    <ul>
      {#each $tasks as t (t.id)}
        <li style="display: flex; align-items: center; gap: .5rem;">
          <strong>{t.name}</strong>
          {#if $running && $running.taskId === t.id}
            <span style="font-variant-numeric: tabular-nums; color: #16a34a;">
              {formatElapsed(now - $running.startedAt)}
            </span>
            <button type="button" onclick={stopTask}>Parar</button>
          {:else}
            <button type="button" onclick={() => startTask(t.id)}>Iniciar</button>
          {/if}
        </li>
      {/each}
    </ul>
    {#if $tasks.length === 0}<p><em>Este proyecto no tiene tareas todavía.</em></p>{/if}
  {/if}
</main>
```

- [ ] **Step 4: Verificación estática (NO abrir la GUI)**

Run: `npm run check`
Expected: 0 errores nuevos. (El warning preexistente de `@types/node` es aceptable.)
> NO ejecutar `npm run tauri dev` (lo prueba el usuario al final).

- [ ] **Step 5: Commit**
```bash
git add src/
git commit -m "feat(ui): start/stop timer per task with live elapsed counter"
```

---

## Task 9: Documentar el incremento

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Entrada en CHANGELOG (bilingüe)**

Bajo `## [Unreleased]` → `### 🇬🇧 Added / 🇪🇸 Añadido`, añadir como último ítem:
```md
- **Timer core** (Plan 4): start/stop a task's timer end-to-end with a single
  global active session — `TimeSession` domain (clock-backwards guard, 12 h
  suspect cap), `TimeSessionRepository` port, `StartTimer` (auto-closes the
  running session; verifies the task exists) / `StopTimer` use cases,
  `SqliteTimeSessionRepository` (partial-index defense), `ts-rs`-generated
  `TimeSessionDto`, Tauri commands (`start_timer`/`stop_timer`/`running_timer`),
  and a Svelte UI with a live elapsed counter. Adds `TaskRepository::find_by_id`. /
  **Núcleo del cronómetro** (Plan 4): iniciar/parar el cronómetro de una tarea de
  punta a punta con una sola sesión activa global — dominio `TimeSession` (guarda
  contra reloj retrocedido, cap de 12 h como sospechosa), puerto
  `TimeSessionRepository`, casos de uso `StartTimer` (cierra la sesión activa;
  verifica que la tarea exista) / `StopTimer`, `SqliteTimeSessionRepository`
  (defensa por índice único parcial), `TimeSessionDto` generado con `ts-rs`,
  comandos Tauri (`start_timer`/`stop_timer`/`running_timer`) y UI Svelte con
  cronómetro en vivo. Agrega `TaskRepository::find_by_id`.
  → `docs/superpowers/plans/2026-06-18-laboraltracker-timer-core.md`
```

- [ ] **Step 2: Commit**
```bash
git add CHANGELOG.md
git commit -m "docs(changelog): record timer core slice (Plan 4)"
```

---

## Definition of Done (Plan 4)

- [ ] `cargo test` → todos en verde (Plans 1-3 + dominio/aplicación/infra de Plan 4).
- [ ] `cargo build` → 0 warnings; `cargo clippy --all-targets -- -D warnings` → exit 0.
- [ ] El dominio (`domain/`) no importa `rusqlite` ni `tauri`.
- [ ] `TimeSessionDto.ts` generado desde Rust en `src/lib/bindings/`, con instantes como `number`.
- [ ] Invariante de sesión única: iniciar una tarea con otra corriendo cierra la anterior; el índice único parcial rechaza una 2ª sesión abierta (test).
- [ ] `stop()` rechaza reloj retrocedido y marca sospechosa > 12 h (tests).
- [ ] `npm run tauri dev`: Iniciar arranca el cronómetro en vivo; Parar lo detiene y persiste; al reabrir, `running_timer` restaura una sesión en curso.
- [ ] Commits convencionales, sin `Co-Authored-By`.

## Qué NO entra (Plan 5 y 6)

- Vista "hoy" / agregación por solapamiento → Plan 5.
- Recuperación de huérfanas al arrancar + heartbeat (escritura de `last_heartbeat_at`) → Plan 6.
- Editar/eliminar sesiones, marcar tareas completadas → incrementos posteriores (YAGNI).
```