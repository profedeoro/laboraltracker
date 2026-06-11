# 0003 — Fundaciones de tiempo y esquema SQLite

- **Status:** Accepted
- **Date:** 2026-06-11
- **Nota (rev. 2026-06-11):** esta ADR fija la **política** de tiempo y
  recuperación. El **mecanismo** concreto (quién escribe `last_heartbeat_at` y el
  caso de uso `RecoverOrphanSessionsOnStartup`) se diseña e implementa en **Plan 3**;
  ver [02-time-policy.md](../conventions/02-time-policy.md) → "Mecanismo". El
  esquema (columnas) llega en Plan 1; la lógica en Plan 3.

## 🇬🇧 Context
In a time tracker, time **is** the domain. The first spec left `Clock::now()`
vague (no UTC, no wall-clock-jump handling) and enforced the "one active session"
invariant only in code (TOCTOU-prone, and contradictory about its owner). These
are the most expensive things to change later (they touch the schema).

## 🇪🇸 Contexto
En un *time tracker*, el tiempo **es** el dominio. El primer spec dejó
`Clock::now()` vago (sin UTC, sin manejo de saltos de reloj) y garantizaba el
invariante de "una sola sesión activa" solo en código (propenso a TOCTOU y
contradictorio sobre su dueño). Esto es lo más caro de cambiar después (toca el
esquema).

## 🇬🇧 Decision
Lock the foundations now:
- All instants stored as **epoch-millis UTC**; duration derived, never stored.
- **Timezone is presentation**; "today" computed by **overlap** across midnight.
- **Orphan session** recovery on startup via `last_heartbeat_at`, marked
  `is_suspect`; duration cap (12 h) also marks suspect.
- Invariant owner = **`StartTimerUseCase`** (single owner). DB guarantees it with a
  **partial unique index**; `CHECK(ended_at >= started_at)` rejects backward clock.
- Concurrency: **`Mutex<Connection>`** in Tauri state, **synchronous commands**,
  per-connection pragmas (WAL, `foreign_keys`, `busy_timeout`).
- FE↔BE DTOs/errors **generated to TS from Rust** (`ts-rs`).

## 🇪🇸 Decisión
Cerrar las fundaciones ahora:
- Todo instante se guarda como **epoch-millis UTC**; duración derivada, nunca persistida.
- **Zona horaria es presentación**; "hoy" por **solapamiento** cruzando medianoche.
- Recuperación de **sesión huérfana** al arrancar vía `last_heartbeat_at`, marcada
  `is_suspect`; cap de duración (12 h) también marca sospechosa.
- Dueño del invariante = **`StartTimerUseCase`** (único). La BD lo garantiza con un
  **índice único parcial**; `CHECK(ended_at >= started_at)` rechaza reloj retrocedido.
- Concurrencia: **`Mutex<Connection>`** en el state de Tauri, **comandos síncronos**,
  pragmas por conexión (WAL, `foreign_keys`, `busy_timeout`).
- DTOs/errores FE↔BE **generados a TS desde Rust** (`ts-rs`).

## 🇬🇧 Consequences
- (+) Trustworthy time data from day 1; corruption blocked at the DB.
- (+) Schema-level guarantees survive code bugs (defense in depth).
- (−) Extra columns (`last_heartbeat_at`, `is_suspect`) and a heartbeat loop.
- Detail: [`02-time-policy.md`](../conventions/02-time-policy.md),
  [`03-concurrency.md`](../conventions/03-concurrency.md),
  [`05-data-schema.md`](../conventions/05-data-schema.md).

## 🇪🇸 Consecuencias
- (+) Datos de tiempo confiables desde el día 1; corrupción bloqueada en la BD.
- (+) Garantías a nivel de esquema sobreviven a bugs del código (defensa en profundidad).
- (−) Columnas extra (`last_heartbeat_at`, `is_suspect`) y un bucle de heartbeat.
- Detalle: [`02-time-policy.md`](../conventions/02-time-policy.md),
  [`03-concurrency.md`](../conventions/03-concurrency.md),
  [`05-data-schema.md`](../conventions/05-data-schema.md).
