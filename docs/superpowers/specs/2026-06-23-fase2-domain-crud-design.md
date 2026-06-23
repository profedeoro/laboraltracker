# Fase 2 — CRUD de dominio + auditoría — Diseño

> Spec de la **Fase 2** del roadmap SaaS ([docs/saas/06-roadmap.md](../../saas/06-roadmap.md)).
> Construye el CRUD de dominio multi-tenant sobre el esqueleto de la Fase 1, y
> aterriza el **`AuditLog`** (primer fase con mutaciones auditables). Se apoya en
> [01-data-model-prisma](../../saas/01-data-model-prisma.md),
> [02-api-and-security](../../saas/02-api-and-security.md) y el ADR
> [0011](../../decisions/0011-product-scope-payroll-and-productivity.md) (scope
> payroll-defensible + hilo de defensibilidad).
>
> **Alcance:** diseño. La implementación va por slices (2a–2e), cada uno su PR.

## Objetivo

Levantar el CRUD de dominio de la plataforma (companies, members, teams, projects,
tasks) con **RBAC + aislamiento de tenant** ya existentes, e introducir la
**infraestructura de auditoría** que el scope payroll-defensible exige desde la
primera mutación sensible (ADR 0011). Criterio de éxito no negociable: **ninguna
mutación sensible ocurre sin dejar traza inmutable en `AuditLog`**, y el aislamiento
entre empresas se mantiene (los guards de Fase 1 cubren cada endpoint nuevo).

## No-objetivos (YAGNI — fases siguientes)

- **`CompanyTrackingSettings`** (política de captura tipada): **Fase 5**. Gobierna
  screenshots/actividad/retención, que no existen hasta la captura. Diseñarla acá
  sería un objeto que no rige nada y cuyos campos deben alinearse con lo que
  realmente se captura (doc 03 + Fase 5). **Fuera de la Fase 2.**
- **Sync, `Device`, `SyncBatch`, `TimeEntry`, `ActivityLog`, `Screenshot`**: Fases 3/5.
- **Reportes** (`Report`, agregaciones): Fase 4.
- **Payroll workflow** (edición manual de tiempo, aprobación, rates, timesheet): Fase 7.
- **UI de auditoría**: Fase 6 (acá se construye solo la **escritura** del `AuditLog`).
- **Gestión de members acotada por equipo para `MANAGER`**: refinamiento futuro. En
  Fase 2, `member.manage` es de `COMPANY_ADMIN` (full); `MANAGER` solo `member.read`
  (coincide con el seed actual y doc 00 §2). El `OwnershipGuard` sigue siendo *seam*:
  el RBAC por permiso cubre lo que la Fase 2 necesita (un `EMPLOYEE` no borra un
  proyecto porque no tiene `project.delete`, no porque un guard de ownership lo frene).

## Decisiones

1. **Cinco slices:** 2a (infra de auditoría), 2b (projects & tasks), 2c (members),
   2d (teams), 2e (companies). Orden por dependencia y valor: el audit primero
   (sustrato de todo), projects/tasks segundo (lo prueban de punta a punta y son
   las entidades más simples), luego members → teams (teams depende de members),
   companies al final (tenant mgmt de Super Admin, menor frecuencia).
2. **Auditoría = UN solo nivel transaccional** (refinado tras review). El audit es
   **`AuditService.record(entry, tx)` invocado en el servicio, dentro de la misma
   transacción** que la mutación. **No hay interceptor en la Fase 2** y se descarta
   el doble mecanismo, por dos razones de correctitud:
   - **Atomicidad:** un interceptor de NestJS corre *después* de que el service
     retorna y **no puede unirse** a la transacción del service (Prisma no expone la
     transacción activa, a diferencia del `QueryRunner` de TypeORM). Resultado:
     phantom-audit si la transacción rollbackea, o mutación sin audit si el
     interceptor falla. Sólo el audit a nivel servicio, en la `tx`, es atómico.
   - **Sin doble escritura:** un único punto de escritura por mutación elimina el
     riesgo de dos filas (`@Audit` + `record()`) para la misma acción.
   - El interceptor `@Audit` queda como **seam futuro documentado** para acciones
     **gruesas no-transaccionales** (eventos de login, export de reporte en Fase 4),
     donde no hay mutación de dominio con la que ser atómico. **YAGNI en Fase 2.**
3. **`SUPER_ADMIN` = permisos de INSTANCIA, no bypass.** El `SUPER_ADMIN` arranca con
   `[]` en el seed; la Fase 2 le da **`company.admin`** (alta de tenant, plan, status,
   métricas) y **nada de permisos de negocio del tenant** (`project.read`,
   `member.manage`, …). **No** se agrega un bypass por `roleKey` al `PermissionGuard`:
   contradiría el doc 00 §2 (*"NO puede ver datos de negocio detallados de una empresa
   salvo soporte explícito/auditado"*) y, en un sistema payroll-defensible, dejaría al
   operador de la instancia leer/tocar datos de nómina sin traza. Técnicamente ya está
   doblemente cerrado: el `TenantGuard` exige `CompanyMember` activo y el Super Admin
   **no es miembro de ninguna empresa** → los endpoints tenant-scoped le dan 403 igual;
   su carril son los `/admin/*` (sin `TenantGuard`, con `company.admin`). El acceso de
   soporte a datos de una empresa = **impersonación auditada**, feature aparte (Fase 6+).
4. **Principio de auditoría (proporción por sensibilidad, no por slice):** se auditan
   **borrados, cambios de rol/status y cambios de settings**. No el rename trivial de
   una tarea o el alta de un proyecto. La sensibilidad —no el slice— decide.
5. **`AuditLog` es solo-append:** la app solo hace `INSERT`. Sin update/delete desde
   código (regla doc 01 §3.7). `onDelete: SetNull` en la relación a `Company`.
6. **Catálogo de permisos se expande** (hoy: `member.read`, `member.manage`,
   `report.view`). Cada slice agrega sus claves `dominio.acción` al seed (matriz
   doc 00 §2). El `PermissionGuard` y el `@RequirePermission` de Fase 1 se reusan
   tal cual (OCP: agregar permiso no toca código de guard). **Nota de seed:** re-correr
   `prisma db seed` **sí** reasigna `RolePermission` a los roles existentes (el map
   maneja los `upsert`); los **tokens en vuelo** quedan stale hasta el refresh — es lo
   correcto (permisos en el JWT, ADR 0006), no un bug.
7. **Cierre de huecos de Fase 1** detectados en el review de cimientos (ADR 0011):
   - `CompanyMember.updatedAt` → se agrega en **2c** (la migración de members). Las
     mutaciones de `CompanyMember` usan **`update` (no `updateMany` ni SQL crudo)**
     para que `@updatedAt` dispare.
   - `Session.ip` / `Session.userAgent` → se agregan en **2e** (con la migración de
     companies; el login los completará cuando se toque auth de nuevo). Sin urgencia,
     sin PR aislado.
8. **Envoltorio de respuesta `{ success, data }`** (doc 02 §7) sigue **diferido**
   (deuda declarada en Fase 1). Pero la **forma paginada `{ data, meta }`** de los
   listados **se establece desde ya** (ver Global Constraints): cambiar la forma de
   retorno después rompe todos los controllers. El `success` wrapper llega con el
   `ResponseInterceptor` global; hasta entonces, `{ data, meta }` desnudo.

## Infra de auditoría (slice 2a — el sustrato)

**Modelo `AuditLog`** (doc 01 §AuditLog), agregado al `schema.prisma` + migración:

```prisma
model AuditLog {
  id          String   @id                 // ULID provisto por el servicio
  companyId   String?
  actorUserId String?
  action      String                        // p. ej. "member.role_changed"
  entityType  String?
  entityId    String?
  metadata    Json     @default("{}")       // { before, after } en cambios sensibles
  createdAt   DateTime @default(now()) @db.Timestamptz(3)

  company Company? @relation(fields: [companyId], references: [id], onDelete: SetNull)

  @@index([companyId, createdAt])
  @@index([actorUserId])
}
```

(El `Company` gana la relación inversa `auditLogs AuditLog[]`.)

**Constantes de acciones** (`apps/api/src/common/audit/audit-actions.ts`): enum/objeto
de strings estables (`member.role_changed`, `project.deleted`, etc.) — no strings
mágicos desperdigados. Cada slice agrega las suyas.

**`AuditService`** (`apps/api/src/common/audit/audit.service.ts`) — **el único
mecanismo de auditoría de la Fase 2**:
- `record(entry: AuditEntry, tx: Prisma.TransactionClient): Promise<void>` — inserta
  un `AuditLog` (ULID generado acá) **usando el cliente de transacción** que le pasa el
  service, de modo que la auditoría y la mutación son **atómicas** (o se audita y muta,
  o nada). El `tx` es obligatorio en la práctica para mutaciones sensibles; la firma lo
  acepta para forzar el patrón.
- `AuditEntry = { action, companyId, actorUserId, entityType?, entityId?, before?, after? }`.
  `before`/`after` se serializan en `metadata`.
- **Patrón de uso** (en cada service que muta algo sensible):
  ```ts
  await this.prisma.$transaction(async (tx) => {
    const before = await tx.project.findUniqueOrThrow({ where: { id } });
    const after = await tx.project.update({ where: { id }, data: { deletedAt: new Date() } });
    await this.audit.record({ action: AuditActions.ProjectDeleted, companyId,
      actorUserId, entityType: 'Project', entityId: id, before, after }, tx);
    return after;
  });
  ```

> **Sin interceptor en Fase 2** (decisión §3 de "Decisiones"): un interceptor no puede
> escribir dentro de la `tx` del service → no es atómico. El `@Audit`/`AuditInterceptor`
> queda como seam futuro para acciones gruesas no-transaccionales (login, export).

**`AllExceptionsFilter`** (`apps/api/src/common/filters/all-exceptions.filter.ts`) —
segundo entregable de cimientos de 2a. Filtro global (`APP_FILTER`) que mapea
**sistemáticamente** los errores a la forma estándar (doc 02 §7), en particular
`PrismaClientKnownRequestError`:
- `P2002` (unique) → `409 Conflict`; `P2025` (no encontrado) → `404`; resto de Prisma →
  `500` **sin** filtrar el error crudo. `HttpException` se respeta tal cual. El detalle
  se loguea del lado servidor; al cliente va el código estable, nunca el stack/ORM.
- Esto hace que el mapeo `@@unique → 409` **no dependa** de que cada controller
  recuerde un `catch`: un olvido ya no filtra un 500 crudo.

**Módulo:** `AuditModule` provee y exporta `AuditService`. El `AllExceptionsFilter` se
registra como `APP_FILTER` (en `AppModule` o un `CommonModule`).

**Tests 2a:**
- Unit `AuditService`: `record(entry, tx)` inserta una fila con los campos correctos
  (incl. `metadata` con `before`/`after`).
- **Integración `audit.service` vs Postgres real** (patrón 1b): dentro de un
  `$transaction` que rollbackea, **no** queda fila de audit (prueba de atomicidad).
- Unit `AllExceptionsFilter`: `P2002 → 409`, `P2025 → 404`, error genérico → 500 sin
  cuerpo crudo, `HttpException` pasa tal cual.
- (El audit **end-to-end** sobre una mutación real se prueba en 2b.)

## Slice 2b — Projects & Tasks

**Entidades:** `Project`, `Task` (+ relaciones e índices del doc 01 §2). Migración que
agrega ambas (con sus `@@unique` y `@@index`). `Company` gana `projects Project[]` y
`tasks Task[]`; relación `Project`↔`Task`.

**Endpoints** (todos bajo `AuthGuard` global + `PermissionGuard` + `TenantGuard`;
`companyId` **siempre del token**, nunca del cliente):

| Método | Ruta | Permiso | Audit |
|---|---|---|---|
| GET | `/projects` | `project.read` | — |
| POST | `/projects` | `project.create` | — |
| PATCH | `/projects/:id` | `project.update` (rename/archive) | — |
| DELETE | `/projects/:id` | `project.delete` (soft-delete, **200** + recurso marcado) | **`AuditService` (en tx)** |
| GET | `/projects/:id/tasks` | `task.read` | — |
| POST | `/projects/:id/tasks` | `task.create` | — |
| PATCH | `/tasks/:id` | `task.update` (rename/complete) | — |
| DELETE | `/tasks/:id` | `task.delete` (soft-delete, **200** + recurso marcado) | **`AuditService` (en tx)** |

**Validaciones (DTO + servicio):** nombre `trim` no vacío; `@@unique([companyId,name])`
en projects y `@@unique([projectId,name])` en tasks (manejar la colisión con 409
`ConflictException`, no 500 crudo de Prisma); `Task.companyId == Task.project.companyId`
(coherencia de tenant, garantizada en servicio). Lecturas filtran `deletedAt IS NULL`
(centralizado en el repositorio).

**Audit (principio §3):** `project.deleted` / `task.deleted` con `before` = la entidad
antes del soft-delete. Create/rename/complete **no** se auditan (baja sensibilidad).

**Permisos nuevos al seed:** `project.read/create/update/delete`,
`task.read/create/update/delete`. Asignación (matriz doc 00 §2): `COMPANY_ADMIN` =
todos; `MANAGER` = read/create/update de ambos + delete (gestiona sus proyectos);
`EMPLOYEE` = `project.read`, `task.read`, `task.update` (completar su tarea);
`CLIENT_VIEWER` = `project.read` (solo lectura). Ajuste fino al implementar.

**Tests 2b:** unit de servicio (crear/renombrar/archivar/soft-delete, colisión de
nombre → 409, coherencia de tenant); e2e que prueba **audit end-to-end**: `COMPANY_ADMIN`
borra un proyecto → 200 **y** existe un `AuditLog` `project.deleted` con `before`; un
`EMPLOYEE` sin `project.delete` → 403; cross-tenant (A no ve/borra projects de B).

## Slice 2c — Members

**Entidad:** `CompanyMember` (ya existe; se agregan operaciones + `updatedAt`).
Migración: `CompanyMember.updatedAt DateTime @updatedAt @db.Timestamptz(3)` (hueco
Fase 1).

**Endpoints** (`PermissionGuard` + `TenantGuard`):

| Método | Ruta | Permiso | Audit |
|---|---|---|---|
| GET | `/members` | `member.read` (ya existe en Fase 1) | — |
| POST | `/members` | `member.manage` (invitar: crea/usa `User` + `CompanyMember` status `INVITED`) | **`member.invited`** |
| PATCH | `/members/:id/role` | `member.manage` (cambiar rol) | **`member.role_changed` (before/after)** |
| PATCH | `/members/:id/status` | `member.manage` (suspender/reactivar) | **`member.status_changed` (before/after)** |

**Validaciones:** `@@unique([userId, companyId])` (una membresía por persona/empresa →
409 si ya existe); invitar a un `email` ya usuario reusa el `User` global (identidad
global, doc 01 §0.3) y crea solo la membresía; no permitir auto-suspensión del último
`COMPANY_ADMIN` (invariante: una empresa no se queda sin admin — validar en servicio).

**Audit (alta sensibilidad — el corazón del slice):** cambios de rol y status con
`before`/`after` vía `AuditService` **dentro de la transacción**. Es la mutación que
más se audita en un sistema payroll-defensible.

**Permisos nuevos al seed:** `member.invite` no hace falta como permiso aparte
(`member.manage` lo cubre); se reusa `member.manage`. Sin deltas de permiso, solo de
endpoints/acciones de audit.

**Tests 2c:** unit (invitar nuevo/existente, cambio de rol audita before/after, no
suspender último admin → 422/409); e2e (admin de A cambia rol de un member de A →
`AuditLog` `member.role_changed`; `EMPLOYEE` → 403; cross-tenant: admin de A no toca
members de B → 403/404).

## Slice 2d — Teams

**Entidad:** `Team` (ya existe en el schema de Fase 1; se agregan operaciones).
Sin migración nueva salvo que falte algo (revisar al implementar).

**Endpoints** (`PermissionGuard` + `TenantGuard`):

| Método | Ruta | Permiso | Audit |
|---|---|---|---|
| GET | `/teams` | `team.read` | — |
| POST | `/teams` | `team.manage` (crear) | — |
| PATCH | `/teams/:id` | `team.manage` (renombrar) | — |
| DELETE | `/teams/:id` | `team.manage` (soft-delete) | **`team.deleted`** |
| PATCH | `/teams/:id/manager` | `team.manage` (asignar manager) | **`team.manager_changed` (before/after)** |
| POST/DELETE | `/teams/:id/members/:memberId` | `team.manage` (asignar/remover) | — |

**Validaciones:** `@@unique([companyId, name])` (409); el `manager` y los `members`
deben ser `CompanyMember` **activos de la misma empresa** (coherencia de tenant en
servicio); `onDelete: SetNull` ya protege al borrar manager/team (no cascada).

**Permisos nuevos al seed:** `team.read`, `team.manage`. `COMPANY_ADMIN` = ambos;
`MANAGER` = `team.read` (+ gestión de sus equipos como refinamiento futuro);
`EMPLOYEE`/`CLIENT_VIEWER` = ninguno o `team.read` según doc 00. Ajuste al implementar.

**Tests 2d:** unit (crear/renombrar/soft-delete, manager debe ser member activo de la
empresa, colisión de nombre → 409); e2e (audit de delete/manager_changed; cross-tenant).

## Slice 2e — Companies

**Entidad:** `Company` (ya existe; se agregan operaciones). Migración: agregar
`Session.ip String?` y `Session.userAgent String?` (hueco Fase 1; el login los llenará
al tocar auth de nuevo — acá solo el schema).

**Endpoints** (`PermissionGuard` + `TenantGuard`):

| Método | Ruta | Permiso | Audit |
|---|---|---|---|
| GET | `/company` | `company.read` (la propia, del token) | — |
| PATCH | `/company` | `company.manage` (Company Admin edita name/slug de la suya) | **`company.updated` (before/after)** |
| GET | `/admin/companies` | `company.admin` (Super Admin: listar todas) | — |
| POST | `/admin/companies` | `company.admin` (Super Admin: alta de tenant + su admin) | **`company.created`** |
| PATCH | `/admin/companies/:id` | `company.admin` (Super Admin: plan/status/soft-delete) | **`company.plan_changed`/`status_changed` (before/after)** |

> **Dos audiencias separadas a propósito:** `/company` (Company Admin sobre la suya,
> via `TenantGuard`) vs `/admin/companies` (Super Admin, instancia-wide, **sin**
> `TenantGuard` — el Super Admin no está atado a un tenant). Esto evita mezclar
> tenant-scoped con instancia-wide en un mismo controlador.

**Validaciones:** `slug` único global (409); nombre no vacío; cambiar `status` a
`CANCELLED`/`SUSPENDED` audita; el alta de tenant crea `Company` + `CompanyMember`
`COMPANY_ADMIN` en una transacción.

**Permisos nuevos al seed:** `company.read`, `company.manage` (Company Admin),
`company.admin` (Super Admin — instancia-wide). El `SUPER_ADMIN` gana `company.admin`
(hoy tiene `[]` en el seed); `COMPANY_ADMIN` gana `company.read`/`company.manage`.

**Tests 2e:** unit (editar la propia, slug duplicado → 409, alta de tenant
transaccional); e2e (Company Admin edita la suya → `company.updated` con before/after;
Company Admin NO puede `/admin/companies` → 403; Super Admin da de alta una empresa →
`company.created`; cross-tenant: Company Admin de A no edita la empresa B).

## Global Constraints (heredados de Fase 1 + nuevos)

- **PKs `String @id` (ULID provisto por el servicio), sin `@default`** (ADR 0005).
- **Tenant del token, jamás del cliente.** `companyId` sale de `@CurrentUser()`; los
  endpoints `/admin/*` de Super Admin son la única excepción (instancia-wide, sin
  `TenantGuard`, con permiso `company.admin`).
- **Guards:** `AuthGuard` global (secure-by-default, `@Public` solo login/refresh/
  health) → `PermissionGuard` (`@RequirePermission`) → `TenantGuard` por controlador.
- **Soft-delete:** las lecturas filtran `deletedAt IS NULL` (centralizado en repos).
  Nada de `DELETE` físico de histórico (`onDelete: Restrict`/`SetNull` ya en el modelo).
- **Soft-delete devuelve `200`** con el recurso marcado (`deletedAt` visible), no `204`
  — semántica informativa y consistente con APIs REST de soft-delete.
- **Auditoría atómica:** `AuditService.record(entry, tx)` corre **dentro** de la
  transacción de la mutación sensible (único mecanismo; sin interceptor). `AuditLog`
  solo-append.
- **Listados paginados desde el día uno.** Todo endpoint de lista acepta
  `?page=1&limit=20` (DTO `PaginationDto` con defaults + `Min(1)`) y devuelve
  `{ data: Dto[], meta: { page, limit, total, totalPages } }`. Los repositorios
  devuelven `{ data, total }`, **no** `Dto[]` desnudo: cambiar la forma de retorno
  después rompería todos los controllers. Convención del doc 02; se establece el
  **seam** aunque el dataset sea chico hoy.
- **Errores → HTTP centralizados:** el `AllExceptionsFilter` global (2a) mapea
  `PrismaClientKnownRequestError` (`P2002→409`, `P2025→404`, …) a la forma estándar
  (doc 02 §7). Los services pueden lanzar `ConflictException`/`NotFoundException`
  explícitas, pero el filtro es la **red**: ningún 500 crudo de Prisma llega al cliente.
- **Validación en el borde:** DTOs con `class-validator` + `ValidationPipe`
  (`whitelist` + `forbidNonWhitelisted` + `transform`).
- **Tres niveles de test por slice** (patrón Fase 1): **unit** de servicio (reglas),
  **integración** de repositorio vs **Postgres real** (`*.repository.spec.ts`,
  excluido del run default, corre en `test:integration` del CI — prueba `deletedAt IS
  NULL`, `@@unique`, `@@index` contra la BD), y **e2e** del flujo + audit.
- **Prisma 6** (ADR 0010). **argon2id** ya en uso. **CI** ya levanta Postgres (Fase 1).
- **Docs:** cada slice actualiza [CHANGELOG.md](../../../CHANGELOG.md) (bilingüe EN/ES)
  y, si toma una decisión nueva, un ADR. **Conventional commits, sin** trailer de IA.

## Resumen de deltas de permisos (seed)

| Permiso | Slice | Roles (ajuste fino al implementar) |
|---|---|---|
| `project.read/create/update/delete` | 2b | ADMIN(all) · MANAGER(all) · EMPLOYEE(read) · CLIENT(read) |
| `task.read/create/update/delete` | 2b | ADMIN(all) · MANAGER(all) · EMPLOYEE(read,update) · CLIENT(read) |
| `member.manage` (ya existe) — nuevas acciones de audit | 2c | ADMIN |
| `team.read/manage` | 2d | ADMIN(manage) · MANAGER(read) |
| `company.read/manage` | 2e | COMPANY_ADMIN |
| `company.admin` | 2e | SUPER_ADMIN |

El seed (`apps/api/prisma/seed.ts`) crece de forma idempotente (mismo patrón de
`upsert` de Fase 1). Cada slice agrega sus permisos + `RolePermission`.
