# Cronómetro — diseño y descomposición (MVP A)

> Diseño del cronómetro (time tracking) sobre la base de Plans 1-3 ya mergeados.
> Las **reglas vivas** no se duplican aquí: viven en
> [02-time-policy.md](../../conventions/02-time-policy.md) (semántica de tiempo,
> huérfanas, heartbeat, "hoy" por solapamiento) y
> [05-data-schema.md](../../conventions/05-data-schema.md) (tabla `time_session`,
> índice único parcial). Este doc registra **decomposición y alcance**.

## Contexto

`02-time-policy.md` y el esquema de Plan 1 ya especifican casi toda la semántica
del cronómetro (sesión única global, epoch-millis UTC, duración derivada, salto de
reloj, huérfanas, heartbeat, vista "hoy" por solapamiento). La tabla
`time_session` (`id, task_id, started_at, ended_at, last_heartbeat_at, is_suspect`)
y el índice único parcial `ux_one_running_session` existen desde Plan 1.

> Nota de numeración: `02-time-policy.md` menciona "Plan 3 (cronómetro)" porque se
> escribió antes de insertar el slice de Tareas. El cronómetro es ahora **Plan 4+**.

## Decisión: descomponer el cronómetro en 3 incrementos

El cronómetro completo (start/stop + vista "hoy" + robustez) es demasiado para un
solo incremento revisable. Se parte en tres planes, cada uno con su PR:

- **Plan 4 — Cronómetro core (start/stop):** iniciar/parar el tiempo de una tarea,
  una sola sesión activa global, con cronómetro en vivo en la UI. *(este doc)*
- **Plan 5 — Vista "hoy":** agregación por solapamiento (totales por tarea →
  por proyecto), excluyendo `is_suspect`. Query ya definida en `02-time-policy.md`.
- **Plan 6 — Robustez de sesiones:** recuperación de sesiones huérfanas al arrancar
  + escritor de `heartbeat` (front cada ~30 s). Mecanismo en `02-time-policy.md`.

Razón: incrementos pequeños y revisables; cada uno entrega valor por separado
(Plan 4 ya permite cronometrar; Plan 5 reporta; Plan 6 endurece).

## Alcance de Plan 4 (start/stop core)

Calca el patrón hexagonal de Plans 2-3.

### Dominio
- Entidad `TimeSession { id, task_id, started_at, ended_at: Option<i64>,
  last_heartbeat_at: Option<i64>, is_suspect: bool }`.
  - `TimeSession::start(id, task_id, now)` → sesión abierta (`ended_at = None`,
    `is_suspect = false`, `last_heartbeat_at = None`).
  - `stop(now) -> Result<(), AppError>`: si `now < started_at` →
    `Err(ClockWentBackwards)`; setea `ended_at = now`; si
    `now - started_at > 12 h` (cap configurable, default 12 h) → `is_suspect = true`.
- Nuevas variantes de `AppError`: `ClockWentBackwards`, `NoRunningSession`.
- Puerto `TimeSessionRepository`: `running() -> Result<Option<TimeSession>>`,
  `add(&TimeSession)`, `update(&TimeSession)`.

### Aplicación
- `StartTimerUseCase`: verifica que la tarea exista (`NotFound` si no), **cierra la
  sesión activa si existe** (invariante de sesión única, dueño del invariante),
  abre y persiste la nueva. Devuelve la nueva sesión.
- `StopTimerUseCase`: toma la sesión activa (`NoRunningSession` si no hay), la para
  (`stop(now)`), persiste. Devuelve la sesión parada.

### Infraestructura
- `SqliteTimeSessionRepository`: `running` = `SELECT ... WHERE ended_at IS NULL`;
  `add` = INSERT; `update` = UPDATE de `ended_at`/`is_suspect`/`last_heartbeat_at`
  por `id`. El índice único parcial es defensa en profundidad.

### Presentación
- `TimeSessionDto` (serde camelCase + `ts-rs`; instantes `i64` → TS `number`).
- Comandos Tauri: `start_timer(taskId)`, `stop_timer()`, `running_timer()`
  (este último restaura el estado del cronómetro al abrir la app).

### Frontend
- Store del timer (sesión activa + transcurrido en vivo vía `setInterval` desde
  `startedAt`, **solo presentación**; el dato canónico lo fija el backend al parar).
- En la lista de tareas: botón **Iniciar** por tarea; la tarea activa muestra el
  transcurrido en vivo + botón **Parar**. Al montar, `running_timer()` restaura.

## Decisiones (aprobadas)

1. Iniciar en una tarea con otra corriendo → **auto-para la anterior** (sesión única).
2. Parar sin sesión activa → **error `NoRunningSession`**; la UI oculta "Parar" si no hay timer.
3. Iniciar sobre una tarea inexistente → **`NotFound`** (espejo de `CreateTask` + FK).
4. La UI tickea en vivo (presentación); el backend fija el tiempo canónico al parar.

## Fuera de alcance de Plan 4

- Vista "hoy" / agregación → Plan 5.
- Recuperación de huérfanas + heartbeat → Plan 6.
- Editar/eliminar sesiones, marcar tareas completadas → incrementos posteriores (YAGNI).

## Verificación

- Dominio: `stop()` (reloj atrás → error; cap 12 h → `is_suspect`).
- Aplicación: `StartTimer` auto-cierra la anterior; `StopTimer` sin sesión → error;
  `StartTimer` sobre tarea inexistente → `NotFound`.
- Infra: `running`/`add`/`update` roundtrip; el índice único parcial rechaza una 2ª
  sesión abierta.
- Contrato: `TimeSessionDto.ts` generado con instantes como `number`.
- `cargo test` + `cargo build` 0 warnings + `cargo clippy -D warnings` + `npm run check`.
