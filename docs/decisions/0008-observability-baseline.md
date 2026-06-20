# 0008 — Línea base de observabilidad de la plataforma

- **Status:** Accepted
- **Date:** 2026-06-19

## 🇬🇧 Context
Doc 02 §6 covers `AuditLog` (a **business** audit trail of sensitive actions) plus a
global `ExceptionFilter` that maps errors to a safe shape and logs the detail
server-side. That is real, but it is **business** observability — not **operational**
observability. There is currently no story for: health checks, metrics
(latency / throughput / error rate), or alerting.

For a SaaS that **captures screenshots and activity and ingests sync batches**, the
operational blind spots are concrete and costly:

- The **retention/purge job** (`retentionDays`, doc 01) fails silently → storage
  keeps growing → discovered 90 days later when the CFO asks why storage costs never
  dropped.
- **Sync processing** error rate spikes at Monday-morning peak and nobody gets a
  signal until employees report "my hours aren't showing".
- **Object storage** credentials rotate / the bucket goes unreachable → screenshot
  sync starts failing with no alert.

A system that cannot be observed cannot be operated. This must be decided now because
it shapes the NestJS structure (a health module, a logging interceptor, a job-run
record pattern) before doc 04 freezes it.

## 🇪🇸 Contexto
El doc 02 §6 cubre `AuditLog` (rastro de auditoría de **negocio** de acciones
sensibles) más un `ExceptionFilter` global que mapea errores a una forma segura y
loguea el detalle del lado servidor. Eso es real, pero es observabilidad de
**negocio** — no observabilidad **operativa**. Hoy no hay historia para: health
checks, métricas (latencia / throughput / tasa de error) ni alertas.

Para un SaaS que **captura pantalla y actividad e ingiere lotes de sync**, los puntos
ciegos operativos son concretos y caros:

- El **job de retención/purga** (`retentionDays`, doc 01) falla en silencio → el
  storage sigue creciendo → se descubre 90 días después cuando el CFO pregunta por
  qué no bajaron los costos de almacenamiento.
- La tasa de error del **procesamiento de sync** se dispara en el pico del lunes a la
  mañana y nadie recibe señal hasta que los empleados reportan "no aparecen mis
  horas".
- Rotan las credenciales del **object storage** / el bucket queda inalcanzable → la
  sync de capturas empieza a fallar sin alerta.

Un sistema que no se puede observar no se puede operar. Hay que decidirlo ahora
porque condiciona la estructura NestJS (módulo de health, interceptor de logging,
patrón de registro de corridas de job) antes de que el doc 04 la congele.

## 🇬🇧 Decision / 🇪🇸 Decisión
Establish a **minimum operational observability baseline**, proportional to the MVP
but never absent. Define it now; implement incrementally with the backend scaffold.
/ Establecer una **línea base mínima de observabilidad operativa**, proporcional al
MVP pero nunca ausente. Definirla ahora; implementarla de forma incremental con el
scaffold del backend.

1. **Health checks.** `/health` (liveness) and `/health/ready` (readiness: DB
   reachable + object storage reachable), via NestJS Terminus. / `/health`
   (liveness) y `/health/ready` (readiness: BD alcanzable + object storage
   alcanzable), con NestJS Terminus.
2. **Structured logging with request id.** JSON logs, a correlation/request id
   propagated per request (interceptor), extending the existing server-side error
   logging. **Never** log secrets or sensitive PII (consistent with doc 02 §6). /
   Logs JSON, un id de correlación/request propagado por request (interceptor),
   extendiendo el logging de errores ya existente. **Nunca** loguear secretos ni PII
   sensible (coherente con doc 02 §6).
3. **Metrics — RED on the hot paths.** Rate, Errors, Duration on HTTP, with special
   attention to `/sync/*` (P99 latency, counts of accepted / conflict / error) and to
   background jobs. Exposition mechanism (Prometheus scrape vs hosted) deferred. /
   **Métricas — RED en los caminos calientes.** Rate, Errors, Duration en HTTP, con
   atención especial a `/sync/*` (latencia P99, conteos de accepted / conflict /
   error) y a los jobs. El mecanismo de exposición (scrape Prometheus vs hosted) se
   difiere.
4. **Job-run records + alerting on silent failure.** Every scheduled job (retention
   purge, etc.) writes a run record (start / end / outcome / counts) — the same
   philosophy as `SyncBatch` does for sync. Alert when a scheduled job does not report
   success within its window, or when the `SyncBatch` `FAILED` rate crosses a
   threshold. `SyncBatch` is already a first-class signal — surface it. / **Registros
   de corrida de job + alertas ante fallo silencioso.** Cada job programado (purga de
   retención, etc.) escribe un registro de corrida (inicio / fin / resultado /
   conteos) — la misma filosofía que `SyncBatch` para la sync. Alertar cuando un job
   no reporta éxito en su ventana, o cuando la tasa `FAILED` de `SyncBatch` cruza un
   umbral. `SyncBatch` ya es una señal de primera clase — exponerla.

**Proportion:** define the baseline, no gold-plating. No full APM / distributed
tracing stack in the MVP; that is a later decision when volume justifies it. /
**Proporción:** definir la base, sin sobre-ingeniería. Sin stack completo de APM /
tracing distribuido en el MVP; eso es una decisión posterior cuando el volumen lo
justifique.

## 🇬🇧 Consequences / 🇪🇸 Consecuencias
- Doc 04 (backend structure) must include a `health` module, a logging interceptor
  with request-id propagation, and a job-run record pattern. / El doc 04 (estructura
  backend) debe incluir un módulo `health`, un interceptor de logging con propagación
  de request-id, y un patrón de registro de corridas de job.
- `SyncBatch` (doc 01) is promoted from sync-audit to an **operational signal**:
  dashboards and alerts read its `status` / `error`. / `SyncBatch` (doc 01) se
  promueve de auditoría de sync a **señal operativa**: dashboards y alertas leen su
  `status` / `error`.
- Modest infra cost (metrics scrape, alert routing); tool choice deferred
  (Prometheus/Grafana vs hosted). / Coste de infra modesto (scrape de métricas, ruteo
  de alertas); elección de herramienta diferida (Prometheus/Grafana vs hosted).
- The closed risk is explicit: operational failures get caught by the **team via a
  signal**, not by the customer or the CFO months later. / El riesgo que se cierra es
  explícito: las fallas operativas las detecta el **equipo por una señal**, no el
  cliente ni el CFO meses después.
- This may grow into a dedicated operations/monitoring doc in the SaaS set if the
  detail outgrows the ADR. / Esto puede crecer a un doc dedicado de operación/
  monitoreo en el set SaaS si el detalle supera al ADR.
