# CLAUDE.md — LaboralTracker

Herramienta de *time tracking* de escritorio (estilo Time Doctor), construida de
forma **incremental** (un sub-proyecto a la vez). Sub-proyecto actual: **MVP A —
núcleo local de time tracking**.

## Gobernanza

- Rige el núcleo universal `~/.claude/CLAUDE.md` + `~/.claude/rules/swebok.md`.
- **No** aplica `~/.claude/rules/python.md`: este proyecto es **Rust + TypeScript**.
- Stack: **Tauri 2.x** · núcleo **Rust** · UI **Svelte + TS** · **SQLite** (`rusqlite`).
- Diseño completo: [docs/superpowers/specs/2026-06-10-laboraltracker-mvp-a-design.md](docs/superpowers/specs/2026-06-10-laboraltracker-mvp-a-design.md).

## Invariantes (siempre en contexto — el detalle está enlazado)

- **Tiempo:** todo instante persistido = **epoch-millis UTC**; duración derivada,
  nunca persistida. Zona horaria es presentación. → [time-policy](docs/conventions/02-time-policy.md)
- **Una sola sesión activa a la vez**; dueño = `StartTimerUseCase`; la BD lo
  garantiza con índice único parcial. → [architecture](docs/conventions/01-architecture-solid.md), [schema](docs/conventions/05-data-schema.md)
- **Reloj de pared salta:** `CHECK(ended_at >= started_at)` y `stop()` rechazan
  retroceso; sesión >12 h o huérfana → `is_suspect`. → [time-policy](docs/conventions/02-time-policy.md)
- **Concurrencia:** `Mutex<Connection>` en el `State`, **comandos síncronos**,
  nunca un `Mutex` a través de `.await`; pragmas por conexión. → [concurrency](docs/conventions/03-concurrency.md)
- **Errores:** enums `thiserror`, sin `unwrap()`/`panic!` en flujo normal; loguear
  completo + devolver controlado; DTOs/errores TS **generados desde Rust**. → [errors](docs/conventions/04-error-handling.md)
- **Dominio puro:** `domain` no importa `rusqlite` ni Tauri. Rust **idiomático**,
  no Python transliterado.

## Convenciones (cargar bajo demanda)

Índice: [docs/conventions/00-index.md](docs/conventions/00-index.md). Carga solo
el tema que tocas; no todo el conjunto.

## Proceso

- Git desde el commit 0. **Conventional commits** (`feat:`, `fix:`, `test:`,
  `refactor:`, `docs:`, `chore:`). Incrementos pequeños y revisables.
  Ramas: `main` + `feature/…`.
- TDD por capas (`cargo test` + `proptest` para tiempo; `vitest` en front).
- Una sola fuente de verdad por concepto; enlazar, no duplicar.

## Comandos

> Pendientes hasta el scaffold de Tauri. Se documentan aquí cuando existan
> (`cargo test`, `cargo clippy`, `cargo fmt`, `npm run …`, `cargo tauri dev`).
