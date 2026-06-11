# 02 — Política de tiempo (no negociable)

> En una herramienta de *time tracking*, el tiempo **es** el dominio.
> Carga este doc cuando toques sesiones, duraciones o la vista "hoy".
> Esquema relacionado: [05-data-schema.md](05-data-schema.md).

## Almacenamiento y tipo

- Todos los instantes se guardan como `INTEGER` = **epoch en milisegundos UTC**.
  Nada de strings locales, nada de zona horaria en la columna. UTC elimina la
  ambigüedad de DST y cambios de zona.
- **Duración derivada** (`ended_at − started_at`); **nunca** se persiste.
- El `Clock` es el único origen del "ahora". Contrato: `now() -> epoch_millis_utc`.

## Zona horaria = presentación, no dominio

- El "día" (vista "hoy") se calcula en la zona local del usuario y se traduce a
  un rango `[day_start, day_end)` en epoch-millis UTC **antes** de tocar SQL.
- Si el usuario viaja, "hoy" sigue su zona actual.

## Reloj de pared y sus saltos

Los instantes persistidos vienen del reloj de pared (vía `Clock`). El reloj de
pared *salta* (NTP, DST, ajuste manual, suspensión). Salvaguardas:

- **Reloj retrocedido:** el `CHECK (ended_at >= started_at)` del esquema rechaza
  el UPDATE en vez de guardar una duración negativa silenciosa. En dominio, el
  método `stop(now)` devuelve `Err(ClockWentBackwards)` si `now < started_at`.
- **Cap de duración:** sesión que supere un máximo configurable (default **12 h**)
  se marca `is_suspect = 1`.

## Sesión que cruza medianoche

NO se atribuye al día de inicio. Se reparte por **solapamiento** del intervalo
con el rango de cada día (23:30→01:00 → 30 min a un día, 60 al siguiente).

```sql
-- Vista "hoy" por solapamiento (:day_start/:day_end calculados en Rust):
SELECT t.project_id, s.task_id,
       SUM( MIN(COALESCE(s.ended_at, :now), :day_end)
          - MAX(s.started_at, :day_start) ) AS ms
FROM time_session s
JOIN task t ON t.id = s.task_id
WHERE s.is_suspect = 0
  AND s.started_at < :day_end
  AND COALESCE(s.ended_at, :now) > :day_start
GROUP BY t.project_id, s.task_id;
```

Excluir `is_suspect` evita que una sesión huérfana infle el día.

La consulta devuelve el desglose **por tarea**. El **total por proyecto** se deriva
en la capa de aplicación sumando `ms` de sus tareas (o con un `GROUP BY
t.project_id` aparte). No se duplica la lógica en SQL; se decide en Plan 3.

## Sesión huérfana (`ended_at = NULL` al arrancar la app)

No se confía en su duración:

1. Al iniciar la app se detecta cualquier sesión con `ended_at = NULL`.
2. Se cierra en `last_heartbeat_at` (si existe) o en `started_at`.
3. Se marca `is_suspect = 1` para no contaminar reportes en silencio.

### Mecanismo (diseño de Plan 3 — no es magia)

El heartbeat y la recuperación necesitan código concreto; no existen "solos":

- **Escritor del heartbeat:** mientras una sesión corre, el **frontend** invoca un
  comando `heartbeat` (Tauri) cada ~30 s que hace
  `UPDATE time_session SET last_heartbeat_at = :now WHERE ended_at IS NULL`. Sin
  este escritor, `last_heartbeat_at` queda **siempre NULL** y la recuperación cae
  a `started_at` (0 min). Es una pieza obligatoria, no opcional.
- **Recuperación:** un caso de uso `RecoverOrphanSessionsOnStartup`, invocado desde
  el `setup` de `lib.rs` **antes** de `manage(...)`. Aunque el índice único parcial
  garantiza ≤1 sesión corriendo, el caso de uso debe iterar defensivamente (por si
  una BD legacy/migración fallida dejó varias) y cerrar todas.
- **Alcance:** ambas piezas se implementan en **Plan 3** (cronómetro). El esquema
  (columnas `last_heartbeat_at`, `is_suspect`) se crea en Plan 1; la lógica llega
  en Plan 3. Hasta entonces, la recuperación no está operativa: documentado, no
  olvidado.

## Dos relojes (UI vs BD)

La UI muestra el transcurrido calculado en JS desde `startedAt` (solo
presentación). El **dato canónico al parar lo fija el backend** con `clock.now()`.
La UI nunca es fuente de verdad del tiempo.
