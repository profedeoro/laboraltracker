# LaboralTracker — Plan 3: Tareas (slice vertical, Crear + Listar)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Crear y listar **tareas** dentro de un proyecto, de punta a punta: dominio Rust puro → puerto `TaskRepository` → repositorio SQLite + doble en memoria → casos de uso → `TaskDto` con tipos TS generados (`ts-rs`) → comandos Tauri → UI Svelte maestro-detalle (elegir proyecto → ver/crear sus tareas).

**Architecture:** Hexagonal (ver [01-architecture-solid.md](../../conventions/01-architecture-solid.md)), idéntica a Plan 2. El dominio no conoce `rusqlite` ni Tauri. `CreateTaskUseCase` verifica la existencia del proyecto vía `ProjectRepository::find_by_id` (devolviendo `AppError::NotFound` si falta) y delega persistencia a `TaskRepository`. IDs = ULID `TEXT` (ADR 0005). Reutiliza `AppError`, `Clock`, `SystemClock`, `Db` y el patrón DTO/`ts-rs` ya existentes.

**Tech Stack:** Rust (`rusqlite`, `ulid`, `thiserror`, `ts-rs`), SQLite, Svelte 5 + TypeScript. (Sin dependencias nuevas: todo ya está desde Plan 2.)

**Convenciones (cargar al implementar):** [01-architecture-solid.md](../../conventions/01-architecture-solid.md) · [03-concurrency.md](../../conventions/03-concurrency.md) · [04-error-handling.md](../../conventions/04-error-handling.md) · [05-data-schema.md](../../conventions/05-data-schema.md).

**Reglas de entorno (Windows):** en PowerShell, antes de cualquier `cargo`, ejecutar `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`. Tests: `cargo test --manifest-path src-tauri\Cargo.toml`. Commits **conventional**, **sin** trailer `Co-Authored-By` (regla de `~/.claude/CLAUDE.md`).

**Cierre de contrato (importante):** Plan 2 dejó con `#[allow(dead_code)]` los ítems `ProjectRepository::find_by_id` (en `domain/ports.rs`) y el enum `AppError` (por `TaskNameEmpty`/`NotFound`, en `domain/error.rs`). Plan 3 los **usa todos**; la Task 5 **elimina ambos `#[allow(dead_code)]`** y exige `cargo build` con **0 warnings**.

---

## File structure (lo que este plan crea/modifica)

```txt
src-tauri/
├── src/
│   ├── domain/
│   │   ├── mod.rs                           # + pub mod task
│   │   ├── error.rs                         # - quitar #[allow(dead_code)] del enum (Task 5)
│   │   ├── ports.rs                         # + trait TaskRepository; quitar allow de find_by_id (Task 5)
│   │   └── task.rs                          # Task (entidad + invariante nombre)  [NUEVO]
│   ├── application/
│   │   ├── mod.rs                           # + pub mod create_task/list_tasks
│   │   ├── testing.rs                       # + InMemoryTaskRepository (cfg test)
│   │   ├── create_task.rs                   # CreateTaskUseCase  [NUEVO]
│   │   └── list_tasks.rs                    # ListTasksUseCase   [NUEVO]
│   ├── infrastructure/
│   │   ├── mod.rs                           # + pub mod sqlite_task_repository
│   │   └── sqlite_task_repository.rs        # SqliteTaskRepository  [NUEVO]
│   ├── presentation/
│   │   ├── dto.rs                           # + TaskDto
│   │   └── commands.rs                      # + create_task, list_tasks
│   └── lib.rs                               # registra los 2 comandos nuevos
└── src/lib/                                 # (frontend)
    ├── bindings/TaskDto.ts                  # generado por ts-rs
    ├── api/tasks.ts                         # wrappers invoke tipados  [NUEVO]
    └── stores/tasks.ts                      # store Svelte             [NUEVO]
src/routes/+page.svelte                      # UI maestro-detalle proyectos→tareas
CHANGELOG.md                                 # entrada del incremento
```

---

## Task 1: Entidad `Task` y puerto `TaskRepository`

**Files:**
- Create: `src-tauri/src/domain/task.rs`
- Modify: `src-tauri/src/domain/mod.rs` (declarar `pub mod task;`)
- Modify: `src-tauri/src/domain/ports.rs` (añadir trait `TaskRepository`)

- [ ] **Step 1: Declarar el módulo**

En `src-tauri/src/domain/mod.rs`, añadir al final:
```rust
pub mod task;
```

- [ ] **Step 2: Escribir la entidad con tests (TDD)**

Create `src-tauri/src/domain/task.rs`:
```rust
use crate::domain::error::AppError;

/// Entidad de dominio. Pertenece a un Project (`project_id`). Invariante: `name`
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
```

- [ ] **Step 3: Ejecutar los tests de la entidad → deben pasar**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo test --manifest-path src-tauri\Cargo.toml domain::task`
Expected: 2 passed.

- [ ] **Step 4: Añadir el puerto `TaskRepository`**

En `src-tauri/src/domain/ports.rs`, al final del archivo añadir:
```rust
use crate::domain::task::Task;

/// Puerto de persistencia de tareas. La infraestructura lo implementa.
pub trait TaskRepository {
    fn add(&mut self, task: &Task) -> Result<(), AppError>;
    fn list_by_project(&self, project_id: &str) -> Result<Vec<Task>, AppError>;
}
```
> Nota: `AppError` ya está importado al inicio de `ports.rs` (lo usa `ProjectRepository`). No dupliques el `use`.

- [ ] **Step 5: Verificar compilación**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo build --manifest-path src-tauri\Cargo.toml`
Expected: `Finished`. (Warnings `dead_code` sobre `TaskRepository`/`Task` son aceptables ahora; se usan en tareas siguientes.)

- [ ] **Step 6: Commit**
```bash
git add src-tauri/
git commit -m "feat(domain): Task entity with non-empty-name invariant and TaskRepository port"
```

---

## Task 2: Caso de uso `CreateTask` (verifica proyecto + dobles en memoria)

**Files:**
- Modify: `src-tauri/src/application/mod.rs` (declarar `create_task`/`list_tasks`)
- Modify: `src-tauri/src/application/testing.rs` (añadir `InMemoryTaskRepository`)
- Create: `src-tauri/src/application/create_task.rs`

- [ ] **Step 1: Declarar los módulos nuevos**

En `src-tauri/src/application/mod.rs`, añadir junto a los existentes:
```rust
pub mod create_task;
pub mod list_tasks;
```
> NOTA: `list_tasks` se implementa en la Task 3. Para que ESTA task compile, creá también un placeholder mínimo `src-tauri/src/application/list_tasks.rs` con solo la línea `// implemented in Task 3`. Se sobreescribe en la próxima task.

- [ ] **Step 2: Añadir el doble en memoria de tareas**

En `src-tauri/src/application/testing.rs`:
1. En el `use` de dominio del inicio, añadir `Task` y el trait `TaskRepository`. El bloque de imports debe quedar:
```rust
use crate::domain::error::AppError;
use crate::domain::ports::{Clock, ProjectRepository, TaskRepository};
use crate::domain::project::Project;
use crate::domain::task::Task;
```
2. Al final del archivo, añadir:
```rust
/// Repositorio de tareas en memoria para tests de casos de uso (sin SQLite).
#[derive(Default)]
pub struct InMemoryTaskRepository {
    pub items: Vec<Task>,
}

impl TaskRepository for InMemoryTaskRepository {
    fn add(&mut self, task: &Task) -> Result<(), AppError> {
        self.items.push(task.clone());
        Ok(())
    }
    fn list_by_project(&self, project_id: &str) -> Result<Vec<Task>, AppError> {
        Ok(self
            .items
            .iter()
            .filter(|t| t.project_id == project_id)
            .cloned()
            .collect())
    }
}
```

- [ ] **Step 3: Escribir el caso de uso con tests (TDD)**

Create `src-tauri/src/application/create_task.rs`:
```rust
use crate::domain::error::AppError;
use crate::domain::ports::{Clock, ProjectRepository, TaskRepository};
use crate::domain::task::Task;

/// Caso de uso: crear una tarea dentro de un proyecto EXISTENTE.
/// Verifica la existencia del proyecto (NotFound si falta), genera el id ULID y
/// sella created_at con el Clock. Defensa en profundidad: la FK de la BD también
/// lo rechaza, pero aquí damos un error de dominio claro.
pub struct CreateTaskUseCase;

impl CreateTaskUseCase {
    pub fn execute(
        tasks: &mut impl TaskRepository,
        projects: &impl ProjectRepository,
        clock: &impl Clock,
        project_id: String,
        name: String,
    ) -> Result<Task, AppError> {
        if projects.find_by_id(&project_id)?.is_none() {
            return Err(AppError::NotFound(project_id));
        }
        let id = ulid::Ulid::new().to_string();
        let task = Task::new(id, project_id, name, clock.now())?;
        tasks.add(&task)?;
        Ok(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::testing::{
        FixedClock, InMemoryProjectRepository, InMemoryTaskRepository,
    };
    use crate::domain::project::Project;

    fn projects_with_p1() -> InMemoryProjectRepository {
        let mut r = InMemoryProjectRepository::default();
        r.items
            .push(Project::new("p1".into(), "Cliente A".into(), None, 1).unwrap());
        r
    }

    #[test]
    fn creates_task_under_existing_project() {
        let mut tasks = InMemoryTaskRepository::default();
        let projects = projects_with_p1();
        let clock = FixedClock(1000);
        let t = CreateTaskUseCase::execute(
            &mut tasks,
            &projects,
            &clock,
            "p1".into(),
            "Disenar".into(),
        )
        .unwrap();
        assert_eq!(t.project_id, "p1");
        assert_eq!(t.created_at, 1000);
        assert!(!t.id.is_empty());
        assert_eq!(tasks.items.len(), 1);
    }

    #[test]
    fn rejects_when_project_missing() {
        let mut tasks = InMemoryTaskRepository::default();
        let projects = InMemoryProjectRepository::default(); // vacío
        let clock = FixedClock(0);
        let r = CreateTaskUseCase::execute(
            &mut tasks,
            &projects,
            &clock,
            "ghost".into(),
            "X".into(),
        );
        assert!(matches!(r, Err(AppError::NotFound(id)) if id == "ghost"));
        assert_eq!(tasks.items.len(), 0);
    }

    #[test]
    fn rejects_empty_name_and_persists_nothing() {
        let mut tasks = InMemoryTaskRepository::default();
        let projects = projects_with_p1();
        let clock = FixedClock(0);
        let r = CreateTaskUseCase::execute(
            &mut tasks,
            &projects,
            &clock,
            "p1".into(),
            "  ".into(),
        );
        assert!(matches!(r, Err(AppError::TaskNameEmpty)));
        assert_eq!(tasks.items.len(), 0);
    }
}
```

- [ ] **Step 4: Ejecutar → deben pasar**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo test --manifest-path src-tauri\Cargo.toml application::create_task`
Expected: 3 passed.

- [ ] **Step 5: Commit**
```bash
git add src-tauri/
git commit -m "feat(application): CreateTaskUseCase with project-existence check and in-memory double"
```

---

## Task 3: Caso de uso `ListTasks`

**Files:**
- Overwrite: `src-tauri/src/application/list_tasks.rs` (placeholder de la Task 2)

- [ ] **Step 1: Escribir el caso de uso con test (TDD)**

Overwrite `src-tauri/src/application/list_tasks.rs`:
```rust
use crate::domain::error::AppError;
use crate::domain::ports::TaskRepository;
use crate::domain::task::Task;

/// Caso de uso: listar las tareas de un proyecto.
pub struct ListTasksUseCase;

impl ListTasksUseCase {
    pub fn execute(
        repo: &impl TaskRepository,
        project_id: &str,
    ) -> Result<Vec<Task>, AppError> {
        repo.list_by_project(project_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::testing::InMemoryTaskRepository;

    #[test]
    fn lists_only_tasks_of_the_given_project() {
        let mut repo = InMemoryTaskRepository::default();
        repo.items.push(Task::new("t1".into(), "p1".into(), "A".into(), 1).unwrap());
        repo.items.push(Task::new("t2".into(), "p2".into(), "B".into(), 2).unwrap());
        repo.items.push(Task::new("t3".into(), "p1".into(), "C".into(), 3).unwrap());
        let out = ListTasksUseCase::execute(&repo, "p1").unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "A");
        assert_eq!(out[1].name, "C");
    }
}
```

- [ ] **Step 2: Ejecutar → debe pasar**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo test --manifest-path src-tauri\Cargo.toml application::list_tasks`
Expected: 1 passed.

- [ ] **Step 3: Commit**
```bash
git add src-tauri/
git commit -m "feat(application): ListTasksUseCase"
```

---

## Task 4: Repositorio SQLite de tareas

**Files:**
- Modify: `src-tauri/src/infrastructure/mod.rs`
- Create: `src-tauri/src/infrastructure/sqlite_task_repository.rs`

- [ ] **Step 1: Declarar el módulo**

En `src-tauri/src/infrastructure/mod.rs`, añadir junto a los existentes:
```rust
pub mod sqlite_task_repository;
```

- [ ] **Step 2: Escribir el repositorio con tests (TDD)**

Create `src-tauri/src/infrastructure/sqlite_task_repository.rs`:
```rust
use crate::domain::error::AppError;
use crate::domain::ports::TaskRepository;
use crate::domain::task::Task;
use rusqlite::Connection;

/// Adaptador SQLite del puerto TaskRepository.
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

impl<'a> TaskRepository for SqliteTaskRepository<'a> {
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
                    task.completed as i64,
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
```
> `db::open_in_memory()` activa `PRAGMA foreign_keys = ON` (verificado en Plan 1), por eso el test de la FK es válido.

- [ ] **Step 3: Ejecutar → deben pasar**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo test --manifest-path src-tauri\Cargo.toml infrastructure::sqlite_task_repository`
Expected: 2 passed.

- [ ] **Step 4: Commit**
```bash
git add src-tauri/
git commit -m "feat(infra): SqliteTaskRepository"
```

---

## Task 5: `TaskDto` + comandos Tauri + wiring + cierre de `dead_code`

**Files:**
- Modify: `src-tauri/src/presentation/dto.rs`
- Modify: `src-tauri/src/presentation/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/domain/error.rs` (quitar `#[allow(dead_code)]`)
- Modify: `src-tauri/src/domain/ports.rs` (quitar `#[allow(dead_code)]` de `find_by_id`)

- [ ] **Step 1: Añadir `TaskDto`**

En `src-tauri/src/presentation/dto.rs`:
1. Tras el `use crate::domain::project::Project;` existente, añadir:
```rust
use crate::domain::task::Task;
```
2. Al final del archivo, añadir:
```rust
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct TaskDto {
    pub id: String,
    pub project_id: String,
    pub name: String,
    // i64 -> number (epoch-millis, seguro < Number.MAX_SAFE_INTEGER); ver ProjectDto.
    #[ts(type = "number")]
    pub created_at: i64,
    pub completed: bool,
}

impl From<Task> for TaskDto {
    fn from(t: Task) -> Self {
        Self {
            id: t.id,
            project_id: t.project_id,
            name: t.name,
            created_at: t.created_at,
            completed: t.completed,
        }
    }
}
```

- [ ] **Step 2: Añadir los comandos**

En `src-tauri/src/presentation/commands.rs`:
1. Añadir estos `use` junto a los existentes:
```rust
use crate::application::create_task::CreateTaskUseCase;
use crate::application::list_tasks::ListTasksUseCase;
use crate::infrastructure::sqlite_task_repository::SqliteTaskRepository;
use crate::presentation::dto::TaskDto;
```
> `SqliteProjectRepository`, `SystemClock`, `Db` y `AppError` ya están importados (los usa el comando de proyectos). No dupliques.
2. Al final del archivo, añadir:
```rust
#[tauri::command]
pub fn create_task(
    project_id: String,
    name: String,
    db: tauri::State<Db>,
) -> Result<TaskDto, AppError> {
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::Repository("db mutex poisoned".into()))?;
    let projects = SqliteProjectRepository::new(&conn);
    let mut tasks = SqliteTaskRepository::new(&conn);
    let task = CreateTaskUseCase::execute(&mut tasks, &projects, &SystemClock, project_id, name)?;
    Ok(TaskDto::from(task))
}

#[tauri::command]
pub fn list_tasks(
    project_id: String,
    db: tauri::State<Db>,
) -> Result<Vec<TaskDto>, AppError> {
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::Repository("db mutex poisoned".into()))?;
    let repo = SqliteTaskRepository::new(&conn);
    let tasks = ListTasksUseCase::execute(&repo, &project_id)?;
    Ok(tasks.into_iter().map(TaskDto::from).collect())
}
```
> Sobre los argumentos: el front invoca con `projectId` (camelCase); Tauri lo mapea a `project_id` (snake_case) automáticamente. Dos repos (`projects` inmutable, `tasks` mutable) comparten el mismo `&conn`: son préstamos compartidos del `Connection`, válidos a la vez.

- [ ] **Step 3: Registrar los comandos en `lib.rs`**

En `src-tauri/src/lib.rs`, reemplazar el `invoke_handler` actual por:
```rust
        .invoke_handler(tauri::generate_handler![
            health,
            presentation::commands::create_project,
            presentation::commands::list_projects,
            presentation::commands::create_task,
            presentation::commands::list_tasks
        ])
```

- [ ] **Step 4: Cerrar los `#[allow(dead_code)]` forward-declared de Plan 2**

Ahora `find_by_id`, `AppError::NotFound` y `AppError::TaskNameEmpty` están en uso real (alcanzables desde los comandos registrados). Quitá los `allow`:
1. En `src-tauri/src/domain/error.rs`: eliminá la línea `#[allow(dead_code)] // ...` que está sobre `pub enum AppError` (dejá el `#[derive(...)]`, el `#[ts(...)]` y el `#[serde(...)]`).
2. En `src-tauri/src/domain/ports.rs`: eliminá el `#[allow(dead_code)] // ...` que está sobre `fn find_by_id(...)` dentro de `trait ProjectRepository`.

- [ ] **Step 5: Verificar build + TODOS los tests + 0 warnings**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo test --manifest-path src-tauri\Cargo.toml`
Expected: todos en verde (Plan 1 + Plan 2 + dominio/aplicación/infra de Plan 3).
Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo build --manifest-path src-tauri\Cargo.toml`
Expected: `Finished` con **0 warnings**. Si aparece algún `dead_code`, es señal de que algo no quedó cableado: revisá el `invoke_handler` y los `use`. No re-agregues `allow` para tapar: investigá la causa.
> ts-rs puede regenerar `.ts` bajo `src/lib/bindings/` al testear. NO los commitees en esta task; van en la Task 6. `git add src-tauri/` solamente.

- [ ] **Step 6: Commit**
```bash
git add src-tauri/
git commit -m "feat(presentation): TaskDto and create/list task commands; close Plan 2 dead_code"
```

---

## Task 6: Exportar el tipo TS de `TaskDto`

**Files:**
- Create (generado): `src/lib/bindings/TaskDto.ts`

- [ ] **Step 1: Generar los bindings**

Run: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo test --manifest-path src-tauri\Cargo.toml export_bindings`
Expected: los tests `export_bindings_*` PASS (incluido `export_bindings_taskdto`).

- [ ] **Step 2: Verificar la ubicación y el contenido**

Run (PowerShell): `Get-Content src\lib\bindings\TaskDto.ts`
Expected: un tipo con `id`, `projectId`, `name`, `createdAt: number`, `completed: boolean`. Confirmá que `createdAt` sea `number` (no `bigint`). El archivo debe estar en `<repo>/src/lib/bindings/`, NO dentro de `src-tauri/`.

- [ ] **Step 3: Commit**
```bash
git add src/lib/bindings/TaskDto.ts
git commit -m "chore(bindings): generate TaskDto TS type via ts-rs"
```

---

## Task 7: Frontend — API tipada, store y UI maestro-detalle

**Files:**
- Create: `src/lib/api/tasks.ts`
- Create: `src/lib/stores/tasks.ts`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Wrapper `invoke` tipado**

Create `src/lib/api/tasks.ts`:
```ts
import { invoke } from '@tauri-apps/api/core';
import type { TaskDto } from '$lib/bindings/TaskDto';

export function createTask(projectId: string, name: string): Promise<TaskDto> {
  return invoke<TaskDto>('create_task', { projectId, name });
}

export function listTasks(projectId: string): Promise<TaskDto[]> {
  return invoke<TaskDto[]>('list_tasks', { projectId });
}
```

- [ ] **Step 2: Store de tareas**

Create `src/lib/stores/tasks.ts`:
```ts
import { writable } from 'svelte/store';
import type { TaskDto } from '$lib/bindings/TaskDto';
import * as api from '$lib/api/tasks';

export const tasks = writable<TaskDto[]>([]);
export const selectedProjectId = writable<string | null>(null);

export async function loadTasks(projectId: string): Promise<void> {
  selectedProjectId.set(projectId);
  tasks.set(await api.listTasks(projectId));
}

export async function addTask(projectId: string, name: string): Promise<void> {
  await api.createTask(projectId, name);
  await loadTasks(projectId);
}
```

- [ ] **Step 3: UI maestro-detalle (proyecto seleccionado → sus tareas)**

Replace `src/routes/+page.svelte`:
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { projects, refreshProjects, addProject } from '$lib/stores/projects';
  import { tasks, selectedProjectId, loadTasks, addTask } from '$lib/stores/tasks';

  let projectName = $state('');
  let taskName = $state('');
  let error = $state('');

  onMount(refreshProjects);

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
</script>

<main style="padding: 2rem; font-family: system-ui; max-width: 40rem;">
  <h1>LaboralTracker — Proyectos y Tareas</h1>

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
    <hr style="margin: 1.5rem 0;" />
    <h2>Tareas</h2>
    <form onsubmit={submitTask} style="display: flex; gap: .5rem; margin: 1rem 0;">
      <input placeholder="Nombre de la tarea" bind:value={taskName} required />
      <button type="submit">Crear tarea</button>
    </form>
    <ul>
      {#each $tasks as t (t.id)}
        <li><strong>{t.name}</strong> {t.completed ? '✓' : ''}</li>
      {/each}
    </ul>
    {#if $tasks.length === 0}<p><em>Este proyecto no tiene tareas todavía.</em></p>{/if}
  {/if}
</main>
```

- [ ] **Step 4: Verificación estática (NO abrir la GUI)**

Run: `npm run check`
Expected: 0 errores nuevos en los archivos creados/modificados. (El warning preexistente de `@types/node` es ruido de infraestructura, ignorable.)
> NO ejecutes `npm run tauri dev` (abre ventana; la prueba GUI la observa el usuario al final).

- [ ] **Step 5: Commit**
```bash
git add src/
git commit -m "feat(ui): master-detail projects and tasks from Svelte"
```

---

## Task 8: Documentar el incremento

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Entrada en CHANGELOG (bilingüe)**

Bajo `## [Unreleased]` → `### 🇬🇧 Added / 🇪🇸 Añadido`, añadir como último ítem de la lista:
```md
- **Tasks slice** (Plan 3): create/list tasks within a project end-to-end — pure
  domain (`Task` + non-empty-name invariant), `TaskRepository` port,
  `CreateTask` (with project-existence check → `NotFound`) / `ListTasks` use
  cases, `SqliteTaskRepository`, `ts-rs`-generated `TaskDto`, Tauri commands, and
  a master-detail Svelte UI. Closes Plan 2's forward-declared `find_by_id` /
  `AppError` items (removed `#[allow(dead_code)]`). /
  **Slice de Tareas** (Plan 3): crear/listar tareas dentro de un proyecto de punta
  a punta — dominio puro (`Task` + invariante de nombre), puerto `TaskRepository`,
  casos de uso `CreateTask` (con verificación de proyecto → `NotFound`) /
  `ListTasks`, `SqliteTaskRepository`, `TaskDto` generado con `ts-rs`, comandos
  Tauri y UI Svelte maestro-detalle. Cierra los ítems forward-declared de Plan 2
  (`find_by_id` / `AppError`; se quitó `#[allow(dead_code)]`).
  → `docs/superpowers/plans/2026-06-12-laboraltracker-tasks.md`
```

- [ ] **Step 2: Commit**
```bash
git add CHANGELOG.md
git commit -m "docs(changelog): record tasks slice (Plan 3)"
```

---

## Definition of Done (Plan 3)

- [ ] `cargo test` → todos en verde (Plan 1 + Plan 2 + Plan 3).
- [ ] `cargo build` → **0 warnings** (incluye haber quitado los dos `#[allow(dead_code)]` de Plan 2).
- [ ] El dominio (`domain/`) no importa `rusqlite` ni `tauri` (verificable por imports).
- [ ] `TaskDto.ts` generado desde Rust en `src/lib/bindings/` (no escrito a mano), con `createdAt: number`.
- [ ] `CreateTaskUseCase` rechaza proyecto inexistente con `NotFound` y nombre vacío con `TaskNameEmpty`.
- [ ] `npm run tauri dev`: seleccionar un proyecto muestra sus tareas; crear una tarea la lista y persiste tras reabrir; las tareas de un proyecto no aparecen en otro.
- [ ] Commits convencionales, sin `Co-Authored-By`.

## Qué NO entra (incrementos posteriores)

- Marcar tarea como completada (toggle de `completed`) → próximo incremento.
- Editar/eliminar/archivar tareas o proyectos → YAGNI por ahora.
- Cronómetro (start/stop, sesión, vista "hoy", huérfana/heartbeat) → Plan 4.
```