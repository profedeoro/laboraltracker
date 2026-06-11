# LaboralTracker — Plan 2: Proyectos (slice vertical completo)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Crear y listar **proyectos** de punta a punta: dominio Rust puro → puertos (traits) → repositorio SQLite + doble en memoria → casos de uso → DTOs con tipos TS generados (`ts-rs`) → comandos Tauri → UI Svelte. Aquí el `Db` cableado en Plan 1 **empieza a leerse**.

**Architecture:** Hexagonal (ver [01-architecture-solid.md](../../conventions/01-architecture-solid.md)). El dominio no conoce `rusqlite` ni Tauri. Los casos de uso dependen de traits (puertos); la infraestructura los implementa; el *composition root* (`lib.rs`) inyecta lo concreto en cada comando. IDs = ULID `TEXT` (ADR 0005). Errores = un enum `AppError` con `thiserror` + `serde` + `ts_rs`.

**Tech Stack:** Rust (`rusqlite`, `ulid`, `thiserror`, `ts-rs`), SQLite, Svelte 5 + TypeScript.

**Convenciones (cargar al implementar):** [01-architecture-solid.md](../../conventions/01-architecture-solid.md) · [03-concurrency.md](../../conventions/03-concurrency.md) · [04-error-handling.md](../../conventions/04-error-handling.md) · [05-data-schema.md](../../conventions/05-data-schema.md).

**Reglas de entorno (Windows):** en PowerShell, antes de cualquier `cargo`, ejecutar `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`. Tests: `cargo test --manifest-path src-tauri\Cargo.toml`. Commits **conventional**, **sin** trailer `Co-Authored-By` (regla de `~/.claude/CLAUDE.md`).

---

## File structure (lo que este plan crea/modifica)

```txt
src-tauri/
├── Cargo.toml                              # + ts-rs
├── src/
│   ├── domain/
│   │   ├── mod.rs                           # pub mod error/project/ports
│   │   ├── error.rs                         # AppError (thiserror + serde + ts-rs)
│   │   ├── project.rs                       # Project (entidad + invariante nombre)
│   │   └── ports.rs                         # ProjectRepository, Clock (traits)
│   ├── application/
│   │   ├── mod.rs                           # pub mod create_project/list_projects + testing(cfg test)
│   │   ├── create_project.rs               # CreateProjectUseCase
│   │   ├── list_projects.rs                # ListProjectsUseCase
│   │   └── testing.rs                       # InMemoryProjectRepository, FixedClock (cfg test)
│   ├── infrastructure/
│   │   ├── mod.rs                           # + clock, sqlite_project_repository
│   │   ├── db.rs                            # (existe)
│   │   ├── clock.rs                         # SystemClock
│   │   └── sqlite_project_repository.rs     # SqliteProjectRepository
│   ├── presentation/
│   │   ├── mod.rs                           # pub mod commands, dto
│   │   ├── dto.rs                           # ProjectDto (serde camelCase + ts-rs)
│   │   └── commands.rs                      # create_project, list_projects
│   └── lib.rs                               # + mods, registra comandos
└── src/lib/                                 # (frontend)
    ├── bindings/                            # *.ts generados por ts-rs
    ├── api/projects.ts                      # wrappers invoke tipados
    └── stores/projects.ts                   # store Svelte
src/routes/+page.svelte                      # UI crear/listar proyectos
```

---

## Task 1: Dependencia `ts-rs` y error de dominio

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/domain/mod.rs`, `src-tauri/src/domain/error.rs`
- Modify: `src-tauri/src/lib.rs` (declarar `mod domain;`)

- [ ] **Step 1: Añadir `ts-rs`**

En `src-tauri/Cargo.toml` `[dependencies]` añadir:
```toml
ts-rs = "10"
```

- [ ] **Step 2: Crear `domain/mod.rs`**

Create `src-tauri/src/domain/mod.rs`:
```rust
pub mod error;
pub mod ports;
pub mod project;
```

- [ ] **Step 3: Escribir el test del error (en `error.rs`)**

Create `src-tauri/src/domain/error.rs`:
```rust
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
```

- [ ] **Step 4: Declarar el módulo en `lib.rs`**

En `src-tauri/src/lib.rs`, debajo de `mod infrastructure;`, añadir:
```rust
mod domain;
```

- [ ] **Step 5: Verificar compilación + test**

Run: `cargo test --manifest-path src-tauri\Cargo.toml display_messages_are_stable`
Expected: PASS (1 test). Si `ts-rs` falla al resolver, confirmá `ts-rs = "10"` y `cargo update`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/
git commit -m "feat(domain): add AppError enum and ts-rs dependency"
```

---

## Task 2: Entidad `Project` y puertos

**Files:**
- Create: `src-tauri/src/domain/project.rs`, `src-tauri/src/domain/ports.rs`

- [ ] **Step 1: Escribir los tests de `Project` (TDD)**

Create `src-tauri/src/domain/project.rs`:
```rust
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
```

- [ ] **Step 2: Ejecutar → deben pasar**

Run: `cargo test --manifest-path src-tauri\Cargo.toml domain::project`
Expected: 2 passed.

- [ ] **Step 3: Definir los puertos (traits)**

Create `src-tauri/src/domain/ports.rs`:
```rust
use crate::domain::error::AppError;
use crate::domain::project::Project;

/// Puerto de persistencia de proyectos. La infraestructura lo implementa.
pub trait ProjectRepository {
    fn add(&mut self, project: &Project) -> Result<(), AppError>;
    fn list(&self) -> Result<Vec<Project>, AppError>;
    fn find_by_id(&self, id: &str) -> Result<Option<Project>, AppError>;
}

/// Reloj inyectable. now() = epoch millis UTC (ver 02-time-policy.md).
pub trait Clock {
    fn now(&self) -> i64;
}
```

- [ ] **Step 4: Declarar `ports`/`project` ya está en `mod.rs` (Task 1). Verificar compilación**

Run: `cargo build --manifest-path src-tauri\Cargo.toml`
Expected: `Finished` sin errores.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/
git commit -m "feat(domain): Project entity with non-empty-name invariant and ports"
```

---

## Task 3: Caso de uso `CreateProject` (con dobles en memoria)

**Files:**
- Create: `src-tauri/src/application/mod.rs`, `src-tauri/src/application/testing.rs`, `src-tauri/src/application/create_project.rs`
- Modify: `src-tauri/src/lib.rs` (declarar `mod application;`)

- [ ] **Step 1: Crear el módulo `application` y los dobles de prueba**

Create `src-tauri/src/application/mod.rs`:
```rust
pub mod create_project;
pub mod list_projects;

#[cfg(test)]
pub mod testing;
```

Create `src-tauri/src/application/testing.rs`:
```rust
use crate::domain::error::AppError;
use crate::domain::ports::{Clock, ProjectRepository};
use crate::domain::project::Project;

/// Repositorio en memoria para tests de casos de uso (sin SQLite).
#[derive(Default)]
pub struct InMemoryProjectRepository {
    pub items: Vec<Project>,
}

impl ProjectRepository for InMemoryProjectRepository {
    fn add(&mut self, project: &Project) -> Result<(), AppError> {
        self.items.push(project.clone());
        Ok(())
    }
    fn list(&self) -> Result<Vec<Project>, AppError> {
        Ok(self.items.clone())
    }
    fn find_by_id(&self, id: &str) -> Result<Option<Project>, AppError> {
        Ok(self.items.iter().find(|p| p.id == id).cloned())
    }
}

/// Reloj fijo para tests deterministas.
pub struct FixedClock(pub i64);
impl Clock for FixedClock {
    fn now(&self) -> i64 {
        self.0
    }
}
```

- [ ] **Step 2: Escribir el test del caso de uso (TDD)**

Create `src-tauri/src/application/create_project.rs`:
```rust
use crate::domain::error::AppError;
use crate::domain::ports::{Clock, ProjectRepository};
use crate::domain::project::Project;

/// Caso de uso: crear un proyecto. Genera el id ULID y sella created_at con el Clock.
pub struct CreateProjectUseCase;

impl CreateProjectUseCase {
    pub fn execute(
        repo: &mut impl ProjectRepository,
        clock: &impl Clock,
        name: String,
        color: Option<String>,
    ) -> Result<Project, AppError> {
        let id = ulid::Ulid::new().to_string();
        let project = Project::new(id, name, color, clock.now())?;
        repo.add(&project)?;
        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::testing::{FixedClock, InMemoryProjectRepository};

    #[test]
    fn creates_and_persists_project() {
        let mut repo = InMemoryProjectRepository::default();
        let clock = FixedClock(1000);
        let p = CreateProjectUseCase::execute(&mut repo, &clock, "Cliente A".into(), None).unwrap();
        assert_eq!(p.name, "Cliente A");
        assert_eq!(p.created_at, 1000);
        assert!(!p.id.is_empty());
        assert_eq!(repo.list().unwrap().len(), 1);
    }

    #[test]
    fn rejects_empty_name_and_persists_nothing() {
        let mut repo = InMemoryProjectRepository::default();
        let clock = FixedClock(0);
        let r = CreateProjectUseCase::execute(&mut repo, &clock, "  ".into(), None);
        assert!(matches!(r, Err(AppError::ProjectNameEmpty)));
        assert_eq!(repo.list().unwrap().len(), 0);
    }
}
```

- [ ] **Step 3: Declarar `mod application;` en `lib.rs`**

En `src-tauri/src/lib.rs`, debajo de `mod domain;`, añadir:
```rust
mod application;
```

- [ ] **Step 4: Ejecutar → deben pasar**

Run: `cargo test --manifest-path src-tauri\Cargo.toml application::create_project`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/
git commit -m "feat(application): CreateProjectUseCase with in-memory test doubles"
```

---

## Task 4: Caso de uso `ListProjects`

**Files:**
- Create: `src-tauri/src/application/list_projects.rs`

- [ ] **Step 1: Escribir el test (TDD)**

Create `src-tauri/src/application/list_projects.rs`:
```rust
use crate::domain::error::AppError;
use crate::domain::ports::ProjectRepository;
use crate::domain::project::Project;

/// Caso de uso: listar todos los proyectos.
pub struct ListProjectsUseCase;

impl ListProjectsUseCase {
    pub fn execute(repo: &impl ProjectRepository) -> Result<Vec<Project>, AppError> {
        repo.list()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::testing::InMemoryProjectRepository;

    #[test]
    fn lists_all_projects() {
        let mut repo = InMemoryProjectRepository::default();
        repo.items.push(Project::new("p1".into(), "A".into(), None, 1).unwrap());
        repo.items.push(Project::new("p2".into(), "B".into(), None, 2).unwrap());
        let out = ListProjectsUseCase::execute(&repo).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "A");
    }
}
```

- [ ] **Step 2: Ejecutar → debe pasar**

Run: `cargo test --manifest-path src-tauri\Cargo.toml application::list_projects`
Expected: 1 passed.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/
git commit -m "feat(application): ListProjectsUseCase"
```

---

## Task 5: Repositorio SQLite + reloj de sistema

**Files:**
- Create: `src-tauri/src/infrastructure/clock.rs`, `src-tauri/src/infrastructure/sqlite_project_repository.rs`
- Modify: `src-tauri/src/infrastructure/mod.rs`

- [ ] **Step 1: Declarar los módulos nuevos**

En `src-tauri/src/infrastructure/mod.rs`, añadir:
```rust
pub mod clock;
pub mod sqlite_project_repository;
```

- [ ] **Step 2: Reloj de sistema**

Create `src-tauri/src/infrastructure/clock.rs`:
```rust
use crate::domain::ports::Clock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Reloj real de pared en epoch millis UTC.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before UNIX epoch")
            .as_millis() as i64
    }
}
```

- [ ] **Step 3: Escribir el test del repo SQLite (TDD, contra BD temporal)**

Create `src-tauri/src/infrastructure/sqlite_project_repository.rs`:
```rust
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
```

- [ ] **Step 4: Ejecutar → deben pasar**

Run: `cargo test --manifest-path src-tauri\Cargo.toml infrastructure::sqlite_project_repository`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/
git commit -m "feat(infra): SqliteProjectRepository and SystemClock"
```

---

## Task 6: DTO + comandos Tauri + wiring

**Files:**
- Create: `src-tauri/src/presentation/mod.rs`, `src-tauri/src/presentation/dto.rs`, `src-tauri/src/presentation/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Crear el módulo de presentación**

Create `src-tauri/src/presentation/mod.rs`:
```rust
pub mod commands;
pub mod dto;
```

- [ ] **Step 2: DTO con serde camelCase + ts-rs**

Create `src-tauri/src/presentation/dto.rs`:
```rust
use crate::domain::project::Project;
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../src/lib/bindings/")]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub created_at: i64,
    pub archived: bool,
}

impl From<Project> for ProjectDto {
    fn from(p: Project) -> Self {
        Self {
            id: p.id,
            name: p.name,
            color: p.color,
            created_at: p.created_at,
            archived: p.archived,
        }
    }
}
```

- [ ] **Step 3: Comandos**

Create `src-tauri/src/presentation/commands.rs`:
```rust
use crate::application::create_project::CreateProjectUseCase;
use crate::application::list_projects::ListProjectsUseCase;
use crate::domain::error::AppError;
use crate::infrastructure::clock::SystemClock;
use crate::infrastructure::db::Db;
use crate::infrastructure::sqlite_project_repository::SqliteProjectRepository;
use crate::presentation::dto::ProjectDto;

#[tauri::command]
pub fn create_project(
    name: String,
    color: Option<String>,
    db: tauri::State<Db>,
) -> Result<ProjectDto, AppError> {
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::Repository("db mutex poisoned".into()))?;
    let mut repo = SqliteProjectRepository::new(&conn);
    let project = CreateProjectUseCase::execute(&mut repo, &SystemClock, name, color)?;
    Ok(ProjectDto::from(project))
}

#[tauri::command]
pub fn list_projects(db: tauri::State<Db>) -> Result<Vec<ProjectDto>, AppError> {
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::Repository("db mutex poisoned".into()))?;
    let repo = SqliteProjectRepository::new(&conn);
    let projects = ListProjectsUseCase::execute(&repo)?;
    Ok(projects.into_iter().map(ProjectDto::from).collect())
}
```

- [ ] **Step 4: Declarar el módulo y registrar comandos en `lib.rs`**

En `src-tauri/src/lib.rs`:
1. Debajo de `mod application;` añadir: `mod presentation;`
2. Reemplazar la línea del `invoke_handler` por:
```rust
        .invoke_handler(tauri::generate_handler![
            health,
            presentation::commands::create_project,
            presentation::commands::list_projects
        ])
```
> El `Db` ya se gestiona en el `setup` (Plan 1). Ahora `db.0` deja de estar "never read" → quitá el `#[allow(dead_code)]` de `Db` en `infrastructure/db.rs` si ya no genera warning (verificá con `cargo build`).

- [ ] **Step 5: Verificar build + todos los tests**

Run: `cargo test --manifest-path src-tauri\Cargo.toml`
Expected: todos en verde (los de Plan 1 + los nuevos). 0 warnings en `cargo build`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/
git commit -m "feat(presentation): project DTO and create/list Tauri commands"
```

---

## Task 7: Exportar tipos TS con `ts-rs`

**Files:**
- Create (generados): `src/lib/bindings/ProjectDto.ts`, `src/lib/bindings/AppError.ts`

- [ ] **Step 1: Generar los bindings**

`ts-rs` exporta los tipos al correr los tests (cada `#[ts(export)]` crea un test que escribe el `.ts`).
Run: `cargo test --manifest-path src-tauri\Cargo.toml export_bindings`
Expected: tests `export_bindings_*` PASS y se crean archivos `.ts`.

- [ ] **Step 2: Verificar que aterrizaron en `src/lib/bindings/`**

Run (PowerShell): `Get-ChildItem src\lib\bindings`
Expected: `ProjectDto.ts` y `AppError.ts`.
Si aterrizaron en otra ruta, ajustá `export_to` en `dto.rs`/`error.rs` (es relativo a `src-tauri/`; `"../src/lib/bindings/"` apunta a `<repo>/src/lib/bindings/`) y volvé a correr.

- [ ] **Step 3: Commit**

```bash
git add src/lib/bindings/
git commit -m "chore(bindings): generate TS types from Rust via ts-rs"
```

---

## Task 8: Frontend — API tipada, store y UI

**Files:**
- Create: `src/lib/api/projects.ts`, `src/lib/stores/projects.ts`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Wrapper `invoke` tipado**

Create `src/lib/api/projects.ts`:
```ts
import { invoke } from '@tauri-apps/api/core';
import type { ProjectDto } from '$lib/bindings/ProjectDto';

export function createProject(name: string, color: string | null): Promise<ProjectDto> {
  return invoke<ProjectDto>('create_project', { name, color });
}

export function listProjects(): Promise<ProjectDto[]> {
  return invoke<ProjectDto[]>('list_projects');
}
```
> `$lib` es el alias de SvelteKit a `src/lib`. Si tu `tsconfig`/`svelte.config.js` no lo resuelve, usá la ruta relativa `../bindings/ProjectDto`.

- [ ] **Step 2: Store de Svelte**

Create `src/lib/stores/projects.ts`:
```ts
import { writable } from 'svelte/store';
import type { ProjectDto } from '$lib/bindings/ProjectDto';
import * as api from '$lib/api/projects';

export const projects = writable<ProjectDto[]>([]);

export async function refreshProjects(): Promise<void> {
  projects.set(await api.listProjects());
}

export async function addProject(name: string, color: string | null): Promise<void> {
  await api.createProject(name, color);
  await refreshProjects();
}
```

- [ ] **Step 3: UI crear/listar proyectos**

Replace `src/routes/+page.svelte`:
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { projects, refreshProjects, addProject } from '$lib/stores/projects';

  let name = $state('');
  let error = $state('');

  onMount(refreshProjects);

  async function submit(e: Event) {
    e.preventDefault();
    error = '';
    try {
      await addProject(name, null);
      name = '';
    } catch (err) {
      error = JSON.stringify(err);
    }
  }
</script>

<main style="padding: 2rem; font-family: system-ui; max-width: 32rem;">
  <h1>LaboralTracker — Proyectos</h1>

  <form onsubmit={submit} style="display: flex; gap: .5rem; margin: 1rem 0;">
    <input placeholder="Nombre del proyecto" bind:value={name} required />
    <button type="submit">Crear</button>
  </form>

  {#if error}<p style="color: crimson;">Error: {error}</p>{/if}

  <ul>
    {#each $projects as p (p.id)}
      <li><strong>{p.name}</strong> <small>{p.id}</small></li>
    {/each}
  </ul>
  {#if $projects.length === 0}<p><em>Sin proyectos todavía.</em></p>{/if}
</main>
```

- [ ] **Step 4: Verificar de extremo a extremo**

Run: `npm run tauri dev` (en PowerShell con `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"` antes).
Expected: la ventana muestra el formulario. Crear un proyecto lo agrega a la lista y **persiste** tras cerrar/reabrir. (Este paso abre GUI: ejecutalo cuando el usuario pueda observar; cerrá con Ctrl+C.)

- [ ] **Step 5: Commit**

```bash
git add src/
git commit -m "feat(ui): create and list projects from Svelte"
```

---

## Task 9: Documentar el incremento

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Entrada en CHANGELOG (bilingüe)**

Bajo `## [Unreleased]` → `### 🇬🇧 Added / 🇪🇸 Añadido`, añadir:
```md
- **Projects slice** (Plan 2): create/list projects end-to-end — pure domain
  (`Project` + non-empty-name invariant), ports, `CreateProject`/`ListProjects`
  use cases, `SqliteProjectRepository`, `ts-rs`-generated DTO/error types, Tauri
  commands, and a Svelte UI. /
  **Slice de Proyectos** (Plan 2): crear/listar proyectos de punta a punta —
  dominio puro (`Project` + invariante de nombre), puertos, casos de uso
  `CreateProject`/`ListProjects`, `SqliteProjectRepository`, tipos DTO/error
  generados con `ts-rs`, comandos Tauri y UI Svelte.
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): record projects slice (Plan 2)"
```

---

## Definition of Done (Plan 2)

- [ ] `cargo test` → todos en verde (Plan 1 + dominio + aplicación + infraestructura).
- [ ] `cargo build` → 0 warnings.
- [ ] El dominio (`domain/`) no importa `rusqlite` ni `tauri` (verificable por imports).
- [ ] Tipos TS generados desde Rust en `src/lib/bindings/` (no escritos a mano).
- [ ] `npm run tauri dev`: crear un proyecto lo lista y persiste tras reabrir.
- [ ] Commits convencionales, sin `Co-Authored-By`.

## Qué NO entra (Plan 3 y 4)

- Tareas (`Task`, su repo/casos de uso/UI) → Plan 3.
- Cronómetro (start/stop, vista "hoy", huérfana/heartbeat) → Plan 4.
- Archivar/editar/eliminar proyectos → incremento posterior (YAGNI por ahora).
```