# 05 — Esquema de datos (SQLite)

> **Canónico aquí hasta el scaffold.** Cuando se cree `src-tauri/`, este DDL pasa
> a `src-tauri/migrations/0001_init.sql`, que se vuelve la fuente de verdad; este
> doc quedará como comentario + enlace. (Evita duplicar: una sola fuente.)
> Política de tiempo asociada: [02-time-policy.md](02-time-policy.md).

## `0001_init.sql`

> El archivo real **no** incluye `PRAGMA foreign_keys = ON;`: los pragmas se
> aplican **por conexión** en `infrastructure::db::open` (ver
> [03-concurrency.md](03-concurrency.md)). Razón: `rusqlite_migration` corre cada
> migración dentro de una transacción, y SQLite ignora `foreign_keys` en
> transacción. Poner el PRAGMA aquí sería un no-op silencioso.

> **IDs = ULID en `TEXT`** (ADR [0005](../decisions/0005-id-strategy-local-to-cloud.md)).
> PK/FK son `TEXT`; el ULID se genera en el **dominio Rust** (crate `ulid`), nunca
> por la BD. Globalmente único → sin colisión en la sync futura. Por eso **no** se
> usa `INTEGER PRIMARY KEY` (rowid autoincremental local).

```sql
CREATE TABLE project (
    id          TEXT    PRIMARY KEY,            -- ULID generado en dominio
    name        TEXT    NOT NULL CHECK (length(trim(name)) > 0),
    color       TEXT,
    created_at  INTEGER NOT NULL,            -- epoch millis UTC
    archived    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE task (
    id          TEXT    PRIMARY KEY,            -- ULID
    project_id  TEXT    NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    name        TEXT    NOT NULL CHECK (length(trim(name)) > 0),
    created_at  INTEGER NOT NULL,
    completed   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX ix_task_project ON task(project_id);

CREATE TABLE time_session (
    id                TEXT    PRIMARY KEY,       -- ULID
    task_id           TEXT    NOT NULL REFERENCES task(id) ON DELETE CASCADE,
    started_at        INTEGER NOT NULL,           -- epoch millis UTC
    ended_at          INTEGER,                    -- NULL = corriendo
    last_heartbeat_at INTEGER,                    -- ancla de recuperación
    is_suspect        INTEGER NOT NULL DEFAULT 0,
    CHECK (ended_at IS NULL OR ended_at >= started_at)  -- sin duraciones negativas
);
CREATE INDEX ix_session_task    ON time_session(task_id);
CREATE INDEX ix_session_started ON time_session(started_at);

-- INVARIANTE GARANTIZADO POR LA BD: como máximo UNA sesión corriendo.
-- La expresión vale 1 para toda fila con ended_at IS NULL; UNIQUE ⇒ una sola.
CREATE UNIQUE INDEX ux_one_running_session
    ON time_session( (ended_at IS NULL) )
    WHERE ended_at IS NULL;
```

## Notas

- `CHECK (ended_at >= started_at)`: red contra el salto de reloj de pared (ver
  [02-time-policy.md](02-time-policy.md)).
- `ux_one_running_session`: defensa en profundidad. **Cómo funciona** (no es por
  distinción de NULL): es un **índice de expresión parcial**. El `WHERE ended_at
  IS NULL` limita el índice a las filas *corriendo*; para todas ellas la expresión
  `(ended_at IS NULL)` vale `1`; `UNIQUE` sobre ese `1` ⇒ como máximo una fila
  corriendo. Verificado contra SQLite live: la 2ª inserción abierta falla con
  `UNIQUE constraint failed`.
  > Ojo: `... ON time_session(task_id) WHERE ended_at IS NULL` sería **incorrecto**
  > (permitiría una sesión activa *por tarea*). El índice debe ser global.
- `ON DELETE CASCADE` solo funciona con `PRAGMA foreign_keys = ON` por conexión
  (ver [03-concurrency.md](03-concurrency.md)).

## Migraciones

Herramienta: `rusqlite_migration` (o `PRAGMA user_version` manual). Disciplina de
migración versionada **desde el commit 0**; cada migración con intención clara,
sin editar una migración ya aplicada.
