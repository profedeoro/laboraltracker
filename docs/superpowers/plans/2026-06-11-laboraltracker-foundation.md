# LaboralTracker — Plan 1: Fundación (scaffold + persistencia)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tener una app Tauri 2 + Svelte/TS que arranca, abre SQLite, aplica la migración `0001_init.sql` y expone un comando `health`; con el invariante "una sola sesión activa" garantizado y **probado** a nivel de base de datos.

**Architecture:** Capas hexagonales en `src-tauri/`. En este plan solo se crea el esqueleto de carpetas y la capa `infrastructure` (conexión + migraciones). Sin dominio ni casos de uso todavía (Plan 2). La conexión vive tras `Mutex<Connection>` en el `State` de Tauri; comandos síncronos.

**Tech Stack:** Tauri 2.x, Rust (`rusqlite` con feature `bundled`, `rusqlite_migration`, `thiserror`, `serde`), Svelte + TypeScript + Vite (SvelteKit), SQLite.

**Convenciones que aplican (cargar al implementar):**
[03-concurrency.md](../../conventions/03-concurrency.md) ·
[05-data-schema.md](../../conventions/05-data-schema.md) ·
[CLAUDE.md](../../../CLAUDE.md). Commits: *conventional* y **sin** trailer
`Co-Authored-By` ni atribución de IA (regla de `~/.claude/CLAUDE.md`).

---

## File structure (lo que este plan crea)

```txt
laboraltracker/                         # raíz del proyecto scaffold (dentro de Exe/)
├── package.json, vite.config.ts, …     # SvelteKit + Vite (generado)
├── src/routes/+page.svelte             # UI mínima: botón que llama health
├── src-tauri/
│   ├── Cargo.toml                      # deps Rust
│   ├── migrations/0001_init.sql        # esquema canónico
│   ├── src/
│   │   ├── main.rs                     # entry (generado)
│   │   ├── lib.rs                      # composition root: open+migrate+manage+health
│   │   └── infrastructure/
│   │       ├── mod.rs
│   │       └── db.rs                   # open(), open_in_memory(), apply(), Db state
│   └── tauri.conf.json
└── .gitignore                          # node_modules, target, *.db
```

> **Nota:** el scaffold de Tauri crea su propio proyecto. Lo generamos **dentro de
> `Exe/`** (la raíz git actual), de modo que `docs/`, `CLAUDE.md` y `CHANGELOG.md`
> ya existentes queden junto al código.

---

## Prerrequisitos (verificar antes de Task 1)

- [ ] **Verificar toolchain**

Run:
```bash
rustc --version && cargo --version && node --version && npm --version
```
Expected: versiones impresas (Rust ≥ 1.80 por `LazyLock`; Node ≥ 18).
Si falta Rust → instalar `rustup`; si falta Node → instalar Node LTS.
En Windows: además se requieren **Microsoft C++ Build Tools** (MSVC) y **WebView2**
(preinstalado en Win11). Si `cargo build` falla por el linker, instalar
"Desktop development with C++" desde Visual Studio Build Tools.

---

## Task 1: Scaffold Tauri 2 + Svelte/TS

**Files:**
- Create: todo el árbol del proyecto (generado por `create-tauri-app`).

- [ ] **Step 1: Generar el proyecto**

Desde `c:/Users/elrug/OneDrive/Escritorio/Python for Science/Exe`:
```bash
npm create tauri-app@latest laboraltracker -- --template svelte-ts --manager npm --yes
```
Expected: crea la carpeta `laboraltracker/` con frontend SvelteKit + `src-tauri/`.
Si el flag `--template svelte-ts` no existe en tu versión, ejecuta
`npm create tauri-app@latest` de forma interactiva y elige: **Svelte**, **TypeScript**,
gestor **npm**.

- [ ] **Step 2: Mover el contenido scaffold a la raíz `Exe/`**

El generador crea `Exe/laboraltracker/...`. Llevamos su contenido a `Exe/` para
convivir con `docs/`, `CLAUDE.md`, `CHANGELOG.md`:
```bash
cd "c:/Users/elrug/OneDrive/Escritorio/Python for Science/Exe"
# PowerShell:
Move-Item -Path .\laboraltracker\* -Destination . -Force
Move-Item -Path .\laboraltracker\.gitignore -Destination . -Force
Remove-Item .\laboraltracker -Recurse -Force
```
Expected: en `Exe/` ahora hay `package.json`, `src/`, `src-tauri/` junto a `docs/`.

- [ ] **Step 3: Instalar dependencias del frontend**

Run: `npm install`
Expected: `node_modules/` creado, sin errores.

- [ ] **Step 4: Verificar que compila el backend Rust**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: `Finished` sin errores (la primera vez baja y compila dependencias).

- [ ] **Step 5: Asegurar `.gitignore` y commit baseline**

Verifica que `.gitignore` incluye `node_modules`, `/src-tauri/target`, `*.db`,
`*.db-wal`, `*.db-shm`. Si falta alguno, añádelo.
```bash
git add -A
git commit -m "feat: scaffold Tauri 2 + Svelte/TS baseline"
```
Expected: commit creado; `git status` limpio salvo artefactos ignorados.

---

## Task 2: Esqueleto de capas + dependencias Rust

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/infrastructure/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Añadir dependencias**

En `src-tauri/Cargo.toml`, en `[dependencies]` añade (junto a las de Tauri ya
presentes):
```toml
rusqlite = { version = "0.32.1", features = ["bundled"] }
rusqlite_migration = "1.3"
thiserror = "1"
ulid = "1"
```
> `ulid` genera los IDs de texto en el dominio (ADR 0005). En Plan 1 aún no hay
> dominio, así que la dependencia se declara aquí pero se usa desde Plan 2.
Y en una sección nueva `[dev-dependencies]`:
```toml
[dev-dependencies]
tempfile = "3"
```
> `features = ["bundled"]` compila SQLite con el binario: cero dependencias del
> sistema. `serde`/`serde_json` ya vienen con el scaffold de Tauri.
>
> **⚠️ Compatibilidad de versiones (crítico):** `rusqlite` y `rusqlite_migration`
> deben compartir la **misma** versión de `rusqlite` o Cargo enlaza dos crates
> `rusqlite` distintas y `to_latest(&mut Connection)` no compila (E0308: dos tipos
> `Connection` incompatibles). El par `rusqlite 0.32.x` + `rusqlite_migration 1.3`
> comparten `rusqlite ^0.32`. **No** mezclar con `rusqlite_migration 1.2` (pide
> `rusqlite ^0.31`).

- [ ] **Step 1b: Verificar que NO hay `rusqlite` duplicado**

Run: `cargo tree -d --manifest-path src-tauri/Cargo.toml`
Expected: **no** aparece `rusqlite` listado dos veces. Si aparece, alinea las
versiones (sube/baja `rusqlite_migration` hasta que su requisito de `rusqlite`
coincida con el tuyo) y repite.

- [ ] **Step 2: Crear el módulo `infrastructure`**

Create `src-tauri/src/infrastructure/mod.rs`:
```rust
pub mod db;
```

- [ ] **Step 3: Declarar el módulo en `lib.rs`**

En `src-tauri/src/lib.rs`, añade al inicio del archivo:
```rust
mod infrastructure;
```

- [ ] **Step 4: Verificar que compila (aún sin `db.rs` → debe fallar claro)**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: FALLA con "file not found for module `db`" o similar. Es esperado;
lo creamos en Task 3. (No commit aquí.)

---

## Task 3: Conexión SQLite con pragmas (TDD)

**Files:**
- Create: `src-tauri/src/infrastructure/db.rs`
- Test: dentro de `db.rs` (`#[cfg(test)]`)

- [ ] **Step 1: Escribir el test que falla**

Create `src-tauri/src/infrastructure/db.rs`:
```rust
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

/// Estado compartido que Tauri inyecta vía `.manage(...)`.
pub struct Db(pub Mutex<Connection>);

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
}
```

- [ ] **Step 2: Ejecutar el test → debe fallar a compilar**

Run: `cargo test --manifest-path src-tauri/Cargo.toml open_`
Expected: FALLA de compilación ("cannot find function `open`/`open_in_memory`").

- [ ] **Step 3: Implementar `open` y `open_in_memory`**

Añade en `db.rs` **encima** del bloque `#[cfg(test)]`:
```rust
/// Abre una conexión a archivo con los pragmas de producción.
/// Los pragmas se aplican POR CONEXIÓN (no persisten en el archivo).
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

/// Conexión en memoria para tests. WAL no aplica en memoria; solo FK.
pub fn open_in_memory() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(conn)
}
```

- [ ] **Step 4: Ejecutar los tests → deben pasar**

Run: `cargo test --manifest-path src-tauri/Cargo.toml open_`
Expected: `test result: ok. 2 passed`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/
git commit -m "feat(infra): SQLite connection with per-connection pragmas"
```

---

## Task 4: Migración `0001_init.sql` y garantías de BD (TDD)

**Files:**
- Create: `src-tauri/migrations/0001_init.sql`
- Modify: `src-tauri/src/infrastructure/db.rs` (añade `apply()` + tests)

> El DDL viene de [05-data-schema.md](../../conventions/05-data-schema.md). **Cambio
> respecto al doc:** la línea `PRAGMA foreign_keys = ON;` NO va en la migración (se
> aplica por conexión en `open()`). `rusqlite_migration` corre cada migración en
> transacción, donde el PRAGMA sería ignorado. Tras este plan, actualiza
> `05-data-schema.md` para reflejar que el archivo real omite esa línea.

- [ ] **Step 1: Crear el archivo de migración (sin el PRAGMA)**

Create `src-tauri/migrations/0001_init.sql`:
```sql
CREATE TABLE project (
    id          TEXT    PRIMARY KEY,
    name        TEXT    NOT NULL CHECK (length(trim(name)) > 0),
    color       TEXT,
    created_at  INTEGER NOT NULL,
    archived    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE task (
    id          TEXT    PRIMARY KEY,
    project_id  TEXT    NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    name        TEXT    NOT NULL CHECK (length(trim(name)) > 0),
    created_at  INTEGER NOT NULL,
    completed   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX ix_task_project ON task(project_id);

CREATE TABLE time_session (
    id                TEXT    PRIMARY KEY,
    task_id           TEXT    NOT NULL REFERENCES task(id) ON DELETE CASCADE,
    started_at        INTEGER NOT NULL,
    ended_at          INTEGER,
    last_heartbeat_at INTEGER,
    is_suspect        INTEGER NOT NULL DEFAULT 0,
    CHECK (ended_at IS NULL OR ended_at >= started_at)
);
CREATE INDEX ix_session_task    ON time_session(task_id);
CREATE INDEX ix_session_started ON time_session(started_at);

CREATE UNIQUE INDEX ux_one_running_session
    ON time_session( (ended_at IS NULL) )
    WHERE ended_at IS NULL;
```

- [ ] **Step 2: Escribir los tests que fallan**

En `db.rs`, dentro de `mod tests`, añade:
```rust
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
```

- [ ] **Step 3: Ejecutar → falla a compilar (no existe `apply`)**

Run: `cargo test --manifest-path src-tauri/Cargo.toml migration_ db_rejects`
Expected: FALLA de compilación ("cannot find function `apply`").

- [ ] **Step 4: Implementar `apply()` con migraciones embebidas**

En `db.rs`, junto a `open`, añade:
```rust
use rusqlite_migration::{Migrations, M};
use std::sync::LazyLock;

static MIGRATIONS: LazyLock<Migrations<'static>> = LazyLock::new(|| {
    Migrations::new(vec![M::up(include_str!("../../migrations/0001_init.sql"))])
});

/// Aplica las migraciones pendientes hasta la última versión.
pub fn apply(conn: &mut Connection) -> rusqlite_migration::Result<()> {
    MIGRATIONS.to_latest(conn)
}
```

- [ ] **Step 5: Ejecutar → deben pasar**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: todos los tests `ok` (5 en total: 2 de Task 3 + 3 nuevos).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/
git commit -m "feat(infra): 0001 schema migration with DB-enforced invariants

- indice unico parcial garantiza una sola sesion activa
- CHECK impide duraciones negativas"
```

---

## Task 5: Wiring en Tauri + comando `health`

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Componer en `lib.rs` (open + migrate + manage + health)**

Reemplaza el cuerpo de `src-tauri/src/lib.rs` por (conservando `mod infrastructure;`
del Task 2):
```rust
mod infrastructure;

use infrastructure::db::{self, Db};
use std::sync::Mutex;
use tauri::Manager;

#[tauri::command]
fn health() -> String {
    "ok".to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir().expect("no app data dir");
            std::fs::create_dir_all(&dir).ok();
            let db_path = dir.join("laboraltracker.db");
            let mut conn = db::open(&db_path).expect("open db");
            db::apply(&mut conn).expect("apply migrations");
            app.manage(Db(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![health])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```
> **Verifica el `.plugin(tauri_plugin_opener::init())`**: el scaffold de Tauri 2 lo
> incluye por defecto. Si tu `lib.rs` generado NO lo tenía, elimina esa línea. No
> inventes plugins que no estén en `Cargo.toml`.

- [ ] **Step 2: Verificar que compila**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: `Finished` sin errores.

- [ ] **Step 3: UI mínima que llama `health`**

Reemplaza `src/routes/+page.svelte` por:
```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  let status = $state('—');

  async function checkHealth() {
    status = await invoke<string>('health');
  }
</script>

<main style="padding: 2rem; font-family: system-ui;">
  <h1>LaboralTracker</h1>
  <button onclick={checkHealth}>Comprobar backend</button>
  <p>Estado: <strong>{status}</strong></p>
</main>
```
> Sintaxis runes de Svelte 5 (`$state`, `onclick`). Si el scaffold trajo Svelte 4,
> usa `let status = '—'` y `on:click={checkHealth}` en su lugar.

- [ ] **Step 3b: Confirmar la configuración SSR/prerender de SvelteKit**

Tauri usa `@sveltejs/adapter-static`. Verifica que existe `src/routes/+layout.ts`
(o `.js`) con SSR desactivado, para que llamar a `@tauri-apps/api` nunca corra en
un entorno de servidor/prerender:
```ts
export const ssr = false;
export const prerender = true;
```
Expected: el archivo del scaffold ya trae estas líneas. Si no existe, créalo. En
este plan `invoke` solo se llama al hacer clic (no en carga de módulo ni en
`load()`), pero esta config evita que el build rompa si una página futura lo hace.

- [ ] **Step 4: Arrancar la app y verificar de extremo a extremo**

Run: `npm run tauri dev`
Expected: abre una ventana de escritorio con el título "LaboralTracker". Al pulsar
**Comprobar backend**, "Estado:" cambia a **ok**. Se crea el archivo
`laboraltracker.db` en el directorio de datos de la app.
Cierra la ventana para terminar (Ctrl+C en la terminal).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: wire SQLite into Tauri state and add health command + UI"
```

---

## Task 6: Documentar el incremento

**Files:**
- Modify: `CHANGELOG.md`
- Modify: `docs/conventions/05-data-schema.md`

- [ ] **Step 1: Actualizar CHANGELOG (bilingüe)**

En `CHANGELOG.md`, bajo `## [Unreleased]` → `### 🇬🇧 Added / 🇪🇸 Añadido`, añade:
```md
- **Project scaffold** (Tauri 2 + Svelte/TS) with SQLite wired into Tauri state,
  migration `0001` applied on startup, and a `health` command. /
  **Scaffold del proyecto** (Tauri 2 + Svelte/TS) con SQLite en el state de Tauri,
  migración `0001` aplicada al arrancar, y un comando `health`.
```

- [ ] **Step 2: Verificar que `0001_init.sql` coincide con el doc canónico**

`docs/conventions/05-data-schema.md` ya documenta que el archivo real **no** lleva
`PRAGMA foreign_keys = ON;` (pragmas por conexión). Verifica que tu
`src-tauri/migrations/0001_init.sql` coincide carácter a carácter con el bloque DDL
de ese doc (sin el PRAGMA). Si difieren, alinéalos. No debe quedar divergencia.

- [ ] **Step 3: Commit**

```bash
git add CHANGELOG.md docs/
git commit -m "docs: record foundation increment in changelog and schema note"
```

---

## Definition of Done (Plan 1)

- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` → 5 tests en verde.
- [ ] `npm run tauri dev` abre la ventana; el botón devuelve `ok`.
- [ ] Existe `laboraltracker.db` con las 3 tablas tras el primer arranque.
- [ ] La BD rechaza una segunda sesión abierta y duraciones negativas (probado).
- [ ] CHANGELOG y nota de esquema actualizados. Todos los commits son convencionales.
- [ ] Sin `unwrap()`/`panic!` en código de runtime salvo `expect` de arranque en el
      composition root (permitido por [04-error-handling.md](../../conventions/04-error-handling.md)).

## Qué NO entra (Plan 2 y 3)

- Dominio (`Project`, `Task`, `TimeSession`), puertos, casos de uso, repos SQLite
  reales, comandos de negocio, generación de tipos TS (`ts-rs`), UI de proyectos/
  tareas y cronómetro. Cada uno con su propio spec→plan.
```