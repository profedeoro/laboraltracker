# Changelog — LaboralTracker

> 🇬🇧 All notable changes to this project, bilingual. Format based on
> [Keep a Changelog](https://keepachangelog.com/); versioning will follow
> [SemVer](https://semver.org/) once the first release ships.
> The *why* of each decision lives in [`docs/decisions/`](docs/decisions/) (ADRs).
>
> 🇪🇸 Todos los cambios notables del proyecto, bilingüe. Formato basado en
> [Keep a Changelog](https://keepachangelog.com/); el versionado seguirá
> [SemVer](https://semver.org/) desde el primer release.
> El *porqué* de cada decisión vive en [`docs/decisions/`](docs/decisions/) (ADRs).

## [Unreleased]

### 🇬🇧 Added / 🇪🇸 Añadido
- **Design spec for MVP A** (local time-tracking core). /
  **Spec de diseño del MVP A** (núcleo local de *time tracking*).
  → `docs/superpowers/specs/2026-06-10-laboraltracker-mvp-a-design.md`
- **Living conventions** (architecture/SOLID, time policy, concurrency, errors,
  SQLite schema), one topic per file, loaded on demand. /
  **Convenciones vivas** (arquitectura/SOLID, política de tiempo, concurrencia,
  errores, esquema SQLite), un tema por archivo, carga bajo demanda.
  → `docs/conventions/`
- **Thin root `CLAUDE.md`**: project invariants (one line each) + links. /
  **`CLAUDE.md` raíz delgado**: invariantes del proyecto (una línea c/u) + enlaces.
- **ADR folder** with retroactive records 0001–0004 (bilingual). /
  **Carpeta de ADRs** con registros retroactivos 0001–0004 (bilingüe).
  → `docs/decisions/`
- **This bilingual CHANGELOG.** / **Este CHANGELOG bilingüe.**
- **Plan 1 (Foundation)** implementation plan (scaffold + SQLite persistence). /
  **Plan 1 (Fundación)**: plan de implementación (scaffold + persistencia SQLite).
  → `docs/superpowers/plans/2026-06-11-laboraltracker-foundation.md`
- **ADR 0005** (Accepted): local→cloud ID strategy = **ULID text PKs** (generated
  in the Rust domain via the `ulid` crate); schema and Plan 1 updated accordingly. /
  **ADR 0005** (Aceptado): estrategia de IDs local→nube = **PK ULID de texto**
  (generadas en el dominio Rust con el crate `ulid`); esquema y Plan 1 actualizados.
- **Foundation built & verified end-to-end** (Plan 1): Tauri 2 + Svelte/TS app
  launches; SQLite (WAL) wired into Tauri state via `Mutex<Connection>`; migration
  `0001` applied on startup (ULID `TEXT` PKs, partial unique index enforcing one
  running session, `CHECK` against negative duration); `health` command; 5/5 Rust
  tests green; 0 warnings. /
  **Fundación construida y verificada de extremo a extremo** (Plan 1): la app Tauri 2
  + Svelte/TS arranca; SQLite (WAL) cableado al estado de Tauri vía
  `Mutex<Connection>`; migración `0001` aplicada al arrancar (PK ULID `TEXT`, índice
  único parcial que garantiza una sola sesión activa, `CHECK` contra duración
  negativa); comando `health`; 5/5 tests Rust en verde; 0 warnings.
- **Projects slice** (Plan 2): create/list projects end-to-end — pure domain
  (`Project` + non-empty-name invariant), ports, `CreateProject`/`ListProjects`
  use cases, `SqliteProjectRepository`, `ts-rs`-generated DTO/error types, Tauri
  commands, and a Svelte UI. /
  **Slice de Proyectos** (Plan 2): crear/listar proyectos de punta a punta —
  dominio puro (`Project` + invariante de nombre), puertos, casos de uso
  `CreateProject`/`ListProjects`, `SqliteProjectRepository`, tipos DTO/error
  generados con `ts-rs`, comandos Tauri y UI Svelte.
  → `docs/superpowers/plans/2026-06-11-laboraltracker-projects.md`
- **Tasks slice** (Plan 3): create/list tasks within a project end-to-end — pure
  domain (`Task` + non-empty-name invariant), `TaskRepository` port,
  `CreateTask` (with project-existence check → `NotFound`) / `ListTasks` use
  cases, `SqliteTaskRepository`, `ts-rs`-generated `TaskDto`, Tauri commands, and
  a master-detail Svelte UI. Closes Plan 2's forward-declared `find_by_id` /
  `AppError` items (removed `#[allow(dead_code)]`). /
  **Slice de Tareas** (Plan 3): crear/listar tareas dentro de un proyecto de punta
  a punta — dominio puro (`Task` + invariante de nombre), puerto `TaskRepository`,
  casos de uso `CreateTask` (con verificación de proyecto → `NotFound`) /
  `ListTasks`, `SqliteTaskRepository`, `TaskDto` generado con `ts-rs`, comandos
  Tauri y UI Svelte maestro-detalle. Cierra los ítems forward-declared de Plan 2
  (`find_by_id` / `AppError`; se quitó `#[allow(dead_code)]`).
  → `docs/superpowers/plans/2026-06-12-laboraltracker-tasks.md`
- **Timer core** (Plan 4): start/stop a task's timer end-to-end with a single
  global active session — `TimeSession` domain (clock-backwards guard, 12 h
  suspect cap), `TimeSessionRepository` port, `StartTimer` (auto-closes the
  running session; verifies the task exists) / `StopTimer` use cases,
  `SqliteTimeSessionRepository` (partial-index defense), `ts-rs`-generated
  `TimeSessionDto`, Tauri commands (`start_timer`/`stop_timer`/`running_timer`),
  and a Svelte UI with a live elapsed counter. Adds `TaskRepository::find_by_id`. /
  **Núcleo del cronómetro** (Plan 4): iniciar/parar el cronómetro de una tarea de
  punta a punta con una sola sesión activa global — dominio `TimeSession` (guarda
  contra reloj retrocedido, cap de 12 h como sospechosa), puerto
  `TimeSessionRepository`, casos de uso `StartTimer` (cierra la sesión activa;
  verifica que la tarea exista) / `StopTimer`, `SqliteTimeSessionRepository`
  (defensa por índice único parcial), `TimeSessionDto` generado con `ts-rs`,
  comandos Tauri (`start_timer`/`stop_timer`/`running_timer`) y UI Svelte con
  cronómetro en vivo. Agrega `TaskRepository::find_by_id`.
  → `docs/superpowers/plans/2026-06-18-laboraltracker-timer-core.md`

- **SaaS platform architecture docs** (the "two halves" web platform that
  complements the desktop agent): foundational doc (vision, 5-role RBAC matrix,
  15 modules, system architecture + agent↔platform sync flows) and the Prisma
  data model — multi-tenant by `companyId`, ULID PKs shared with the agent,
  `BigInt` epoch-millis for agent-born instants vs `DateTime` for platform
  metadata. After external technical review the model adopted global-identity
  users + `CompanyMember` (multi-company), added `Device`/`SyncBatch`/
  `CompanyTrackingSettings`, replaced cascade deletes of historical data with
  soft-delete + `Restrict`, and defined business uniqueness constraints.
  Analysis only; no platform code yet. /
  **Docs de arquitectura de la plataforma SaaS** (la mitad web que complementa al
  agente de escritorio): documento fundacional (visión, matriz RBAC de 5 roles,
  15 módulos, arquitectura del sistema + flujos de sync agente↔plataforma) y el
  modelo de datos Prisma (multi-tenant por `companyId`, PK ULID compartidas con
  el agente, `BigInt` epoch-millis para instantes del agente vs `DateTime` para
  metadatos de plataforma). Tras revisión técnica externa el modelo adoptó
  identidad global + `CompanyMember` (multi-empresa), agregó `Device`/`SyncBatch`/
  `CompanyTrackingSettings`, reemplazó el borrado en cascada de datos históricos
  por borrado lógico + `Restrict`, y definió restricciones únicas de negocio.
  Solo análisis; aún sin código de plataforma.
  → `docs/saas/00-architecture-overview.md`, `docs/saas/01-data-model-prisma.md`
- **SaaS API & security doc**: REST conventions, per-module endpoints, the
  agent→platform **sync endpoint** (idempotent ULID upsert, device authorization,
  `SyncBatch` audit, partial per-item response, and the name-collision policy:
  item-level reject — no silent rename/merge), JWT access + opaque rotating
  refresh tokens, the four ordered guards (auth→permission→tenant→ownership),
  cross-cutting hardening and a standard error shape. /
  **Doc de API y seguridad SaaS**: convenciones REST, endpoints por módulo, el
  **endpoint de sync** agente→plataforma (upsert idempotente por ULID,
  autorización de dispositivo, auditoría `SyncBatch`, respuesta parcial por ítem y
  la política de colisión de nombres: rechazo por ítem — sin renombrar/mergear en
  silencio), JWT + refresh opaco rotatorio, los cuatro guards en orden
  (auth→permiso→tenant→ownership), hardening transversal y forma de error
  estándar.
  → `docs/saas/02-api-and-security.md`
- **SaaS privacy & monitoring doc** (product + legal): what is captured and what
  is **not** (activity counts, **no keylogging**, no camera/audio), per-company
  typed policy (`CompanyTrackingSettings`, screenshots off by default),
  role-scoped visibility, retention/deletion (soft-delete + retention job),
  employee transparency, and how the system **enforces** privacy by design (not
  just promises). /
  **Doc de privacidad y monitoreo SaaS** (producto + legal): qué se captura y qué
  **no** (conteos de actividad, **sin keylogging**, sin cámara/audio), política
  tipada por empresa (`CompanyTrackingSettings`, capturas off por defecto),
  visibilidad acotada por rol, retención/borrado (soft-delete + job de retención),
  transparencia hacia el empleado, y cómo el sistema **hace cumplir** la
  privacidad por diseño (no solo la promete).
  → `docs/saas/03-privacy-policy.md`
- **SaaS backend structure doc** (NestJS): feature-modular + layered organization,
  folder layout, module anatomy (controller/service/repository/dto), where the
  four guards live and why they are cross-cutting (multi-tenant safety), the
  special `auth`/`sync`/`reports` modules and retention job, boot-time hardening
  (`main.ts`), env validation, and the testing pyramid (incl. the mandatory
  cross-tenant isolation e2e test). /
  **Doc de estructura backend SaaS** (NestJS): organización modular por feature +
  capas, layout de carpetas, anatomía de módulo (controller/service/repository/
  dto), dónde viven los cuatro guards y por qué son transversales (seguridad
  multi-tenant), los módulos especiales `auth`/`sync`/`reports` y el job de
  retención, hardening en el arranque (`main.ts`), validación de entorno, y la
  pirámide de tests (incluido el test e2e obligatorio de aislamiento entre
  empresas).
  → `docs/saas/04-backend-nestjs-structure.md`
- **SaaS frontend structure doc** (Next.js App Router): feature-modular layout,
  container/presentational components, scoped global state (auth/active-company/
  role/permissions in Zustand, server data in fetch hooks), role-based UI that
  mirrors — never replaces — backend RBAC, the centralized HTTP client (token +
  transparent refresh + error mapping), React Hook Form + Zod forms, UTC-millis
  time as presentation, Server vs Client components, and frontend testing. /
  **Doc de estructura frontend SaaS** (Next.js App Router): layout modular por
  feature, componentes container/presentational, estado global acotado
  (auth/empresa-activa/rol/permisos en Zustand, datos de servidor en hooks de
  fetch), UI por rol que refleja —nunca reemplaza— el RBAC del backend, cliente
  HTTP centralizado (token + refresh transparente + mapeo de errores), formularios
  React Hook Form + Zod, tiempo UTC-millis como presentación, Server vs Client
  components y testing del frontend.
  → `docs/saas/05-frontend-nextjs-structure.md`
- **SaaS repo decision + build roadmap doc** (closes the architecture doc set):
  resolves the repo structure as a **monorepo** (decisive reason: the shared
  agent↔platform sync contract — single source of truth, atomic cross-half PRs),
  with the proposed `apps/`+`packages/` layout, and a 7-phase build roadmap
  (0 monorepo/scaffold → 1 auth+tenancy+RBAC → 2 domain CRUD → 3 sync → 4 reports
  → 5 capture/privacy → 6 advanced) feeding incremental spec→plan→slices, plus how
  it interleaves with the agent's pending work. /
  **Doc de decisión de repo + roadmap de construcción SaaS** (cierra el set de
  arquitectura): resuelve la estructura como **monorepo** (razón decisiva: el
  contrato de sync agente↔plataforma compartido — una fuente de verdad, PRs
  atómicos entre mitades), con el layout `apps/`+`packages/` propuesto, y un
  roadmap de 7 fases (0 monorepo/scaffold → 1 auth+tenancy+RBAC → 2 CRUD de
  dominio → 3 sync → 4 reportes → 5 captura/privacidad → 6 avanzado) que alimenta
  los slices incrementales, más cómo se entrelaza con el trabajo pendiente del
  agente.
  → `docs/saas/06-roadmap.md`
- **Phase 0 design spec** (monorepo restructure + platform scaffold): two slices
  (0a move the agent to `apps/agent` preserving relative paths + empty pnpm
  workspace; 0b scaffold `apps/api`/`apps/web`/`packages/contracts` + dev Postgres
  + path-filtered CI), with hard success criteria (agent builds/runs identically;
  skeletons boot). pnpm workspaces for the platform; agent stays standalone npm. /
  **Spec de diseño de la Fase 0** (reestructura a monorepo + scaffold de
  plataforma): dos slices (0a mover el agente a `apps/agent` preservando paths
  relativos + workspace pnpm vacío; 0b scaffold `apps/api`/`apps/web`/
  `packages/contracts` + Postgres de dev + CI con path filters), con criterios de
  éxito duros (el agente compila/arranca idéntico; los esqueletos bootean). pnpm
  workspaces para la plataforma; el agente queda standalone con npm.
  → `docs/superpowers/specs/2026-06-19-monorepo-fase0-design.md`

### 🇬🇧 Changed / 🇪🇸 Cambiado
- **Spec foundations hardened** after technical review: UTC epoch-millis time
  policy, midnight overlap, orphan-session recovery, partial unique index for the
  single-active-session invariant, `Mutex<Connection>` concurrency model, honest
  cloud-migration limit, typed FE↔BE contract generated from Rust. /
  **Fundaciones del spec endurecidas** tras revisión técnica: política de tiempo
  epoch-millis UTC, solapamiento en medianoche, recuperación de sesión huérfana,
  índice único parcial para el invariante de sesión única, modelo de concurrencia
  `Mutex<Connection>`, límite honesto del salto a la nube, contrato FE↔BE tipado
  generado desde Rust.
  → ADR [0003](docs/decisions/0003-time-and-schema-foundations.md)
- **Documentation reorganised** from one monolithic spec into decision-record +
  on-demand conventions, to cut token cost and document drift. /
  **Documentación reorganizada** de un spec monolítico a registro-de-decisión +
  convenciones bajo demanda, para reducir coste de tokens y deriva.
  → ADR [0004](docs/decisions/0004-docs-conventions-vs-decision-record.md)
- **Hardened after adversarial review** (2 independent agents): fixed
  `rusqlite`/`rusqlite_migration` version incompatibility in Plan 1, specified the
  heartbeat/orphan-recovery mechanism (Plan 3), removed schema duplication, fixed a
  misleading partial-index comment, added SvelteKit SSR check, and **removed the
  `Co-Authored-By` trailer** to comply with the repo rule. /
  **Endurecido tras revisión adversarial** (2 agentes independientes): corregida la
  incompatibilidad de versiones `rusqlite`/`rusqlite_migration` en Plan 1,
  especificado el mecanismo de heartbeat/recuperación (Plan 3), eliminada la
  duplicación de esquema, corregido un comentario engañoso del índice parcial,
  añadido chequeo SSR de SvelteKit, y **eliminado el trailer `Co-Authored-By`** para
  cumplir la regla del repo.

### 🇬🇧 Decided / 🇪🇸 Decidido
- **Stack:** Tauri (Rust + Svelte/TS) over Python/PySide6. /
  **Stack:** Tauri (Rust + Svelte/TS) sobre Python/PySide6.
  → ADR [0001](docs/decisions/0001-stack-tauri-rust-svelte.md)
- **Architecture:** Clean/Hexagonal + SOLID (didactic, deliberately oversized). /
  **Arquitectura:** Clean/Hexagonal + SOLID (didáctica, sobredimensionada a propósito).
  → ADR [0002](docs/decisions/0002-hexagonal-architecture-solid.md)
- **SaaS — permissions in the access token:** the JWT carries a signed
  `permissions: string[]`, recomputed at login/refresh/switch-company; the
  `PermissionGuard` becomes an O(1) check (no per-request DB resolution). /
  **SaaS — permisos en el access token:** el JWT lleva un `permissions: string[]`
  firmado, recalculado al login/refresh/switch-company; el `PermissionGuard` pasa a
  ser una verificación O(1) (sin resolución contra BD por request).
  → ADR [0006](docs/decisions/0006-permissions-in-access-token.md)
- **SaaS — sync name-collision policy:** item-level reject (`NAME_TAKEN`/409), no
  auto-rename, no auto-merge; the agent mirrors the partial unique index. /
  **SaaS — colisión de nombres en la sync:** rechazo por ítem (`NAME_TAKEN`/409),
  sin auto-renombrar ni auto-mergear; el agente espeja el índice único parcial.
  → ADR [0007](docs/decisions/0007-sync-name-collision-policy.md)
- **SaaS — observability baseline:** health checks, structured logging with
  request-id, RED metrics on `/sync/*`, and job-run records + alerting on silent
  failures (`SyncBatch` promoted to an operational signal). /
  **SaaS — línea base de observabilidad:** health checks, logging estructurado con
  request-id, métricas RED en `/sync/*`, y registros de corrida de job + alertas
  ante fallos silenciosos (`SyncBatch` promovido a señal operativa).
  → ADR [0008](docs/decisions/0008-observability-baseline.md)

---

### 🇬🇧 How to use / 🇪🇸 Cómo se usa
- Add entries under **[Unreleased]** as work happens; group by Added / Changed /
  Fixed / Removed / Decided. On release, rename the section to the version + date. /
  Añade entradas bajo **[Unreleased]** según avanza el trabajo; agrupa por
  Añadido / Cambiado / Corregido / Eliminado / Decidido. Al hacer release, renombra
  la sección a la versión + fecha.
- Each entry stays one line of *what*; the *why* goes to an ADR and is linked. /
  Cada entrada es una línea de *qué*; el *porqué* va a un ADR y se enlaza.
