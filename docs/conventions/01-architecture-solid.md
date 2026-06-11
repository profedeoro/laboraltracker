# 01 — Arquitectura (Clean / Hexagonal) y SOLID

> Carga este doc cuando diseñes o muevas piezas entre capas.
> Fuente de verdad del spec: §4. Aquí vive el detalle.

## Capas (en `src-tauri/`, Rust)

Regla de dependencia: las flechas apuntan al **dominio** (lo abstracto) → DIP.

```txt
Frontend (Svelte/TS)                 ← Presentación (UI)
   │  invoke()  ── comandos Tauri (DTOs tipados generados desde Rust)
src-tauri/ (Rust)
 ├─ presentation/   handlers de comandos Tauri (finos) + DTOs
 ├─ application/    casos de uso (StartTimer, StopTimer, CreateProject…)
 ├─ domain/         entidades, value objects, invariantes, PUERTOS (traits)
 └─ infrastructure/ adaptadores: repositorios SQLite (implementan los traits)

presentation ──► application ──► domain (traits/puertos) ◄── infrastructure
   composition root (lib.rs) inyecta los adaptadores concretos
```

- **`domain`**: sin dependencias externas (ni SQLite ni Tauri). 100% testeable.
- **`application`**: depende solo de *traits* del dominio (puertos).
- **`infrastructure`**: implementa esos traits con `rusqlite`.
- **`presentation`**: traduce comando Tauri ↔ caso de uso. Sin lógica de negocio.
- **`lib.rs`** (*composition root*): el **único** lugar de la app donde se
  construyen e inyectan las implementaciones concretas (abrir BD, crear repos,
  registrarlos en el estado de Tauri). "Composition root" = el punto de entrada
  donde se "componen" todas las dependencias una sola vez; el resto del código solo
  recibe abstracciones. Aquí también se registran los comandos.

## Puertos (traits del dominio)

```txt
ProjectRepository   add / list / find_by_id / archive
TaskRepository      add / list_by_project / find_by_id
SessionRepository   add(new) / save(existing) / find_running / list_overlapping(range)
Clock               now() -> epoch_millis_utc   (inyectable para tests)
```

Semántica: **`add` = INSERT** de sesión nueva; **`save` = UPDATE** de una
existente (`ended_at`/`is_suspect`). **Decisión a cerrar en Plan 2** (no dejarla
abierta al implementar): elegir entre mantener `add`/`save` separados o colapsar en
un único `save`/`upsert`. Recomendación: mantener `add`/`save` separados — son
operaciones de dominio distintas (crear vs. cerrar) y el índice único parcial
protege el INSERT.

## Dueño único del invariante "una sola sesión activa"

Vive **solo en `StartTimerUseCase`** (orquestación), no en un servicio de
dominio paralelo. Defensa en profundidad: la BD lo garantiza con un índice único
parcial (ver [05-data-schema.md](05-data-schema.md)). El `Mutex<Connection>`
(ver [03-concurrency.md](03-concurrency.md)) serializa la ejecución, así que no
hay carrera viva; el índice es la red de seguridad.

## Tabla SOLID (guía viva)

- **SRP:** una razón de cambio por pieza. Cada caso de uso = struct con un método.
- **OCP:** nuevo adaptador de persistencia no toca dominio ni casos de uso.
  *Límite honesto:* aísla del motor de persistencia, **no** de los problemas de
  sistemas distribuidos de la nube (ver spec §11).
- **LSP:** `SqliteSessionRepository` e `InMemorySessionRepository` intercambiables.
- **ISP:** puertos pequeños y específicos, no un repositorio-Dios.
- **DIP:** `application` depende de traits; solo el composition root ve lo concreto.

## Nota de proporción (decisión consciente)

Para la funcionalidad cruda del MVP esta arquitectura está **sobredimensionada**.
Se acepta a propósito como ejercicio didáctico. En cada sub-proyecto se revisa si
una abstracción gana su sitio o sobra: **podar**, no solo sumar.
