# LaboralTracker SaaS — Estructura de repo y roadmap de construcción

> Documento **6** (cierre del set). Convierte el análisis de los docs 00-05 en
> **fases de construcción** priorizadas, de donde salen los planes incrementales
> (spec → plan → slices, la misma disciplina del agente). Resuelve la decisión de
> **estructura de repos** que quedó pendiente desde el doc 00. Cubre el punto 21
> del brief.

---

## 1. Decisión: MONOREPO

**Resuelta (2026-06-19): monorepo.** Las dos mitades (agente + plataforma) viven en
**un solo repositorio** — este.

> Cambio respecto del doc 00: ahí había recomendado *repos separados* por las
> toolchains distintas (Rust/Cargo vs Node/npm). Sigue siendo el contra real del
> monorepo. Pero subestimé un argumento **a favor** que pesa más en este caso.

### Por qué monorepo gana acá

1. **El contrato de sync es compartido.** El agente y la plataforma intercambian los
   mismos DTOs (sesiones/actividad/capturas, por ULID, epoch-millis UTC). En
   monorepo ese contrato es **una sola fuente de verdad** y un cambio se hace
   **atómico en ambas mitades, en un solo PR**. En repos separados, cambiar el
   contrato son dos PRs coordinados y una ventana de desincronización.
2. **Ya generamos tipos cruzando lenguajes.** El agente genera TS desde Rust con
   `ts-rs`. El mismo mecanismo puede alimentar un paquete de contratos que la
   plataforma consuma. Eso es natural en monorepo.
3. **Docs y ADRs ya viven acá.** `docs/` (convenciones, decisiones, este set SaaS)
   es el hogar del producto. Mantenerlo junto al código de ambas mitades evita
   deriva.
4. **Equipo chico / un solo flujo.** Menos fricción de coordinación: una rama, un
   historial, un lugar para issues.

### El contra (y cómo se maneja)

- **Toolchains mixtas** (Cargo + Node): se aísla por carpeta. El workspace Node
  (npm/pnpm workspaces) cubre `api` + `web` + `packages`; Cargo cubre el agente. No
  se mezclan; conviven.
- **CI más compleja**: se resuelve con *path filters* — el pipeline corre los tests
  de Rust solo si cambió el agente, los de Node solo si cambió la plataforma.
- **Despliegues distintos**: el agente se empaqueta (Tauri) y la plataforma se
  despliega (contenedores). Son *targets* separados que salen del mismo repo; no es
  problema, es configuración.

### Layout propuesto

```txt
laboraltracker/                 (este repo)
├── apps/
│   ├── agent/                  # la app Tauri actual (hoy en raíz: src-tauri/ + src/)
│   ├── api/                    # backend NestJS (doc 04)
│   └── web/                    # dashboard Next.js (doc 05)
├── packages/
│   └── contracts/              # DTOs de sync compartidos (generados/derivados)
├── docs/                       # convenciones, decisiones, set SaaS (ya está acá)
├── Cargo.toml                  # workspace Rust (apps/agent)
├── package.json                # workspace Node (apps/api, apps/web, packages/*)
└── ...
```

> **Nota de migración (Fase 0):** mover el agente de la raíz a `apps/agent/` es un
> cambio mecánico **pero delicado** (rutas de `tauri.conf.json`, el `export_to` de
> `ts-rs`, CI). Va en su **propio PR**, con `cargo test` + `cargo tauri dev`
> verificados antes y después. No se mezcla con feature nueva.

---

## 2. Roadmap por fases

Cada fase produce **software que funciona y se prueba**, y se descompone en slices
(spec → plan → subagents, como venimos). El agente ya tiene Plans 1-4; la
plataforma arranca de cero.

| Fase | Entrega | Mitad | Depende de |
|------|---------|-------|-----------|
| **0** | Monorepo + scaffold (api, web, contracts, CI, Postgres en Docker) | Infra | — |
| **1** | Auth + tenancy: User/Company/CompanyMember/Role/Permission/Session, login+refresh+rotación, los 4 guards, seed RBAC | Plataforma | 0 |
| **2** | CRUD de dominio: companies, members, teams, projects, tasks (multi-tenant + RBAC) | Plataforma | 1 |
| **3** | **Sync**: Device + autorización, SyncBatch, `POST /sync/time-entries` idempotente; cliente de sync en el agente | **Ambas** | 1, 2 + agente |
| **4** | Reportes: horas por usuario/proyecto/tarea (excluye `isSuspect`) + dashboard básico | Plataforma | 2, 3 |
| **5** | Captura: actividad + screenshots en el agente, bucket + URLs firmadas, política `CompanyTrackingSettings` + blur + job de retención | **Ambas** | 3 |
| **6** | Avanzado: notificaciones, facturación, UI de auditoría, exports | Plataforma | 4, 5 |

### Qué hace especial a cada fase

- **Fase 1 es el esqueleto del que todo cuelga.** Sin auth+tenancy+RBAC no se puede
  construir nada multi-tenant con seguridad. Su slice final **debe** incluir el test
  e2e de **aislamiento entre empresas** (doc 04 §8).
- **Fase 3 es el corazón de las dos mitades.** Conecta el agente **ya construido**
  con la plataforma. Acá se materializa la idempotencia por ULID, la autorización de
  `Device` y la política de colisión de nombres (doc 02 §3). Toca ambas mitades →
  el monorepo paga su valor (PR atómico).
- **Fase 5 es la más sensible (privacidad).** Captura de pantalla/actividad: se
  construye **después** de tener auth, tenancy y sync sólidos, y respetando el doc 03
  (capturas off por defecto, blur, retención, transparencia).

---

## 3. Cómo interactúa con el trabajo pendiente del agente

El agente tiene su propia cola, que **se entrelaza** con las fases de la plataforma:

- **Plan 5 (vista "hoy")** y **Plan 6 (heartbeat + recuperación de huérfanas)**:
  independientes; mejoran el agente y conviene tenerlos antes de la Fase 3 (datos más
  limpios para sincronizar). Plan 6 además cierra el hallazgo #1 del backlog.
- **Backlog de hardening del agente** (cascade→archive, helper de lock, clock sin
  panic): quick wins en una rama propia; el de cascade conviene **antes** de exponer
  borrado.
- **Agente: cliente de sync** → es la contraparte de la Fase 3 (mismo PR, monorepo).
- **Agente: captura de actividad/screenshots** → contraparte de la Fase 5.

---

## 4. Primer slice recomendado

No arrancar por "lo más vistoso" (el dashboard), sino por el cimiento:

> **Fase 0 → Fase 1.** Primero el monorepo (mover agente, scaffold api/web/contracts,
> CI, Postgres en Docker), luego **auth + tenancy + RBAC** con su test de aislamiento.
> Recién con ese esqueleto seguro se construyen proyectos, sync y reportes encima.

Cada fase entra al flujo de siempre: `brainstorming` (spec) → `writing-plans` (plan)
→ `subagent-driven-development` (build por tasks) → review → merge. Igual que Plans
1-4 del agente.

---

## 5. Cierre del set de arquitectura

Con este documento, el análisis pedido queda cubierto de punta a punta:

| Doc | Tema |
|-----|------|
| [00](00-architecture-overview.md) | Visión, dos mitades, roles, módulos, arquitectura, flujos |
| [01](01-data-model-prisma.md) | Modelo de datos Prisma (multi-tenant, ULID, soft-delete) |
| [02](02-api-and-security.md) | API REST, sync, auth (JWT+refresh), 4 guards, hardening |
| [03](03-privacy-policy.md) | Privacidad y monitoreo (producto + legal) |
| [04](04-backend-nestjs-structure.md) | Estructura backend NestJS |
| [05](05-frontend-nextjs-structure.md) | Estructura frontend Next.js |
| **06** | **Repo (monorepo) + roadmap de construcción** |

El análisis es la base. La construcción empieza por la **Fase 0**, cuando se decida
arrancar — con la misma disciplina incremental del agente.
