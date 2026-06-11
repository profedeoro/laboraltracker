# 03 — Concurrencia y conexión SQLite/Tauri

> Carga este doc cuando toques la conexión, los repos o un comando Tauri.

## Reglas

- `rusqlite::Connection` es `Send` pero `!Sync`. Vive tras un
  `Mutex<Connection>` en el `State` de Tauri. Suficiente para mono-usuario; un
  pool (`r2d2_sqlite`) solo se justifica con lecturas concurrentes reales, que
  aquí no hay.
- **Comandos síncronos:** el I/O de SQLite local es sub-milisegundo; no justifica
  `async`. **Nunca** mantener un `std::sync::Mutex` a través de un `.await`.
  Si un comando futuro hiciera trabajo largo, envolver el bloqueante en
  `tauri::async_runtime::spawn_blocking`.
- **Pragmas por conexión** (`foreign_keys`, `busy_timeout` **no** persisten en el
  archivo): centralizados en `infrastructure::db::open`. Sin
  `PRAGMA foreign_keys = ON` por conexión, los `ON DELETE CASCADE` del esquema
  **no se aplican**.

## `infrastructure/db.rs`

```rust
use rusqlite::Connection;
use std::{path::Path, sync::Mutex};

/// Estado compartido que Tauri inyecta vía `.manage(...)`.
pub struct Db(pub Mutex<Connection>);

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;      -- mejor concurrencia lectura/escritura
         PRAGMA foreign_keys = ON;       -- imprescindible: SQLite las apaga por defecto
         PRAGMA synchronous = NORMAL;    -- seguro y rápido bajo WAL
         PRAGMA busy_timeout = 5000;",   -- reintenta 5s ante lock en vez de fallar
    )?;
    Ok(conn)
}
```

## Composition root y handler

```rust
// lib.rs
let conn = infrastructure::db::open(&db_path).expect("no se pudo abrir la BD");
// migraciones aquí, antes de manage(...)
builder
    .manage(Db(Mutex::new(conn)))
    .invoke_handler(tauri::generate_handler![start_timer, stop_timer /* … */]);

// presentation/commands.rs
#[tauri::command]
fn start_timer(task_id: i64, db: tauri::State<Db>) -> Result<SessionDto, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::Internal)?;
    // construir repos sobre &conn, invocar StartTimerUseCase, mapear a DTO
}
```
