# Convenciones de LaboralTracker — Índice

> **Reglas vivas** del proyecto: lo que gobierna el código y debe estar en
> contexto cuando se programa el área correspondiente. Una sola fuente de verdad
> por tema; el spec de diseño las *enlaza*, no las repite.

## Cómo usar esto

- `CLAUDE.md` (raíz) lleva las **invariantes en una línea** + enlaces aquí. Se
  auto-carga en cada sesión; es barato y ancla al asistente.
- Estos documentos llevan el **detalle**, y se cargan **bajo demanda** según la
  tarea. No hace falta cargar todo: carga solo el tema que tocas.

## Mapa (qué leer y cuándo)

| Doc | Tema | Cárgalo cuando… |
|-----|------|-----------------|
| [01-architecture-solid.md](01-architecture-solid.md) | Capas hexagonales, puertos/adaptadores, tabla SOLID | diseñes o muevas piezas entre capas |
| [02-time-policy.md](02-time-policy.md) | Semántica del tiempo (UTC, medianoche, sesión huérfana) | toques sesiones, duraciones o la vista "hoy" |
| [03-concurrency.md](03-concurrency.md) | `Mutex<Connection>`, comandos síncronos, pragmas | toques la conexión, repos o un comando Tauri |
| [04-error-handling.md](04-error-handling.md) | Errores `thiserror`, logging local, contrato tipado | definas o propagues errores |
| [05-data-schema.md](05-data-schema.md) | DDL SQLite (canónico hasta scaffold) | toques el esquema o una consulta |

## Reglas transversales (aplican siempre)

- **Una sola fuente de verdad por concepto.** Si un dato vive aquí, el spec y el
  código lo enlazan; nunca se duplica el contenido (evita deriva → evita
  alucinaciones).
- **Rust idiomático, no Python transliterado.** El sistema de tipos y `Result`
  ya dan garantías; no se replican capas pensadas para lenguajes con excepciones.
- **Gobernanza:** núcleo universal `~/.claude/CLAUDE.md` + `~/.claude/rules/swebok.md`.
  No aplica `python.md` (este proyecto es Rust + TS).
