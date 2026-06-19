# LaboralTracker SaaS — Modelo de datos (Prisma + PostgreSQL)

> Documento **1** del set de arquitectura. Se construye sobre
> [`00-architecture-overview.md`](00-architecture-overview.md). Cubre el punto 8
> del brief: el modelo de datos de la **plataforma** (no del agente — el agente ya
> tiene su SQLite, ver [`../conventions/05-data-schema.md`](../conventions/05-data-schema.md)).
>
> **Alcance:** esto es **diseño**, no código a ejecutar. El `schema.prisma` real se
> escribe cuando scaffoldeemos el backend (decisión de repo aún pendiente, ver
> doc 00). Acá fijamos entidades, campos, relaciones, índices y reglas.

---

## 0. Tres decisiones que gobiernan todo el modelo

Antes de los modelos, las decisiones transversales. Si estas están bien, el resto
se cae de maduro.

### 0.1 IDs = ULID en `TEXT` (no `cuid`/`uuid`/autoincrement)

Misma estrategia que el agente (ADR [0005](../decisions/0005-id-strategy-local-to-cloud.md)).
**El id que el agente genera local ES el id en la nube.** Consecuencias en Prisma:

- Todos los PK son `String @id` **sin `@default(...)`**: el id siempre se provee
  (lo trae el agente en la sync, o lo genera el servicio NestJS con un ULID al
  crear desde la web). No dejamos que Prisma/Postgres inventen el id.
- Para entidades que nacen del agente (`TimeEntry`, `ActivityLog`, `Screenshot`),
  ese ULID es además la **clave de idempotencia**: el endpoint de sync hace
  `upsert` por `id` → reenviar el mismo lote no duplica (doc 00, §"Comunicación
  agente↔plataforma").

### 0.2 Tiempo: `BigInt` (epoch-millis UTC) para instantes del agente · `DateTime` para metadatos de plataforma

Esta es la decisión **no trivial** del modelo. El agente persiste **epoch-millis
UTC** y deriva la duración, nunca la guarda ([time-policy](../conventions/02-time-policy.md)).
Para no romper esa invariante al consolidar:

| Tipo de instante | Nace en | Tipo Prisma | Por qué |
|---|---|---|---|
| `startedAt`, `endedAt`, `capturedAt`, `windowStartAt/EndAt`, `lastHeartbeatAt` | **Agente** | `BigInt` (millis UTC) | Copia exacta y sin pérdida del valor canónico del agente; cero conversión de zona al consolidar agentes de distintas zonas |
| `createdAt`, `updatedAt`, `generatedAt`, `expiresAt`, `revokedAt`, `syncedAt` | **Plataforma** | `DateTime @db.Timestamptz(3)` | Son metadatos del servidor; `@default(now())` es idiomático y se ve bien en consultas/admin |

**Regla:** la duración (`durationMs`) **no se persiste**; se deriva en la consulta
de reportes (`endedAt - startedAt`), respetando la misma política del agente. La
zona horaria es **presentación** (se resuelve en el frontend / parámetro de
reporte), nunca almacenamiento.

> Nota Prisma/JS: `BigInt` evita el techo de `2^53` de `number` y es lo correcto
> para millis; el borde de serialización JSON (BigInt→string) se maneja en la capa
> de presentación (igual que el agente hace `#[ts(type = "number")]`).

### 0.3 Multi-tenant por `companyId`

Toda entidad de negocio lleva `companyId` (FK a `Company`) **e índice por
`companyId`**. El aislamiento entre empresas se garantiza en **dos capas**:

1. **Aplicación:** un `TenantGuard` filtra por `companyId` del token en cada query
   (doc 02 de seguridad).
2. **Datos (futuro):** Postgres **Row-Level Security** por `companyId` como defensa
   en profundidad. Se deja **anotado, no activado** en el MVP (proporción: primero
   el guard; RLS cuando el volumen/criticidad lo pidan).

`Company` es la **raíz del tenant**; `User`, `Role`, `Permission` y `Session`
tienen tratamiento especial (ver notas por modelo).

---

## 1. Mapa de relaciones (alto nivel)

```txt
Company (tenant raíz)
 ├─ User ──┬─ (roleId) ─→ Role ──< RolePermission >── Permission
 │         ├─ (teamId) ─→ Team   (Team.managerId ─→ User)
 │         ├─ Session            (refresh tokens)
 │         ├─ TimeEntry  ─→ Task ─→ Project
 │         ├─ ActivityLog ─→ TimeEntry
 │         └─ Screenshot  ─→ TimeEntry
 ├─ Project ──< Task
 ├─ Report   (definición/caché de reporte)
 └─ AuditLog (inmutable)
```

Las tres entidades **compartidas con el agente** son `Project`, `Task` y
`TimeEntry` (el `TimeSession` del agente, ya consolidado). El resto es exclusivo
de la plataforma.

---

## 2. Schema Prisma (diseño)

> Borrador del `schema.prisma`. Datasource/generator van arriba; los modelos abajo.
> Comentarios `///` documentan reglas de negocio donde importan.

```prisma
generator client {
  provider = "prisma-client-js"
}

datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

// ────────────────────────────── TENANT ──────────────────────────────

/// Raíz del tenant. Todo dato de negocio cuelga de una Company.
model Company {
  id        String   @id                 // ULID, provisto por el servicio
  name      String
  slug      String   @unique             // identificador URL-safe
  plan      Plan     @default(FREE)
  status    CompanyStatus @default(ACTIVE)
  settings  Json     @default("{}")      // política de capturas, intervalo, etc.
  createdAt DateTime @default(now()) @db.Timestamptz(3)
  updatedAt DateTime @updatedAt @db.Timestamptz(3)

  users     User[]
  teams     Team[]
  projects  Project[]
  tasks     Task[]
  timeEntries  TimeEntry[]
  activityLogs ActivityLog[]
  screenshots  Screenshot[]
  reports   Report[]
  auditLogs AuditLog[]
}

// ────────────────────────────── RBAC ──────────────────────────────

/// Rol del sistema. Cinco semillas (ver enum RoleKey). Permisos COMO DATOS.
model Role {
  id          String  @id
  key         RoleKey @unique            // SUPER_ADMIN, COMPANY_ADMIN, ...
  name        String
  description String?
  users       User[]
  permissions RolePermission[]
}

/// Capacidad atómica, p. ej. "time.read", "user.manage", "report.view".
model Permission {
  id          String  @id
  key         String  @unique            // dominio.acción
  description String?
  roles       RolePermission[]
}

/// Join Role↔Permission (muchos a muchos). Agregar permiso ≠ tocar código (OCP).
model RolePermission {
  roleId       String
  permissionId String
  role         Role       @relation(fields: [roleId], references: [id], onDelete: Cascade)
  permission   Permission @relation(fields: [permissionId], references: [id], onDelete: Cascade)

  @@id([roleId, permissionId])
  @@index([permissionId])
}

// ────────────────────────────── USERS / TEAMS ──────────────────────────────

/// Usuario. companyId NULL solo para SUPER_ADMIN (cross-tenant).
model User {
  id           String     @id              // ULID
  companyId    String?                      // NULL ⇒ super admin
  roleId       String
  teamId       String?
  email        String     @unique
  passwordHash String                       // bcrypt/argon2 — nunca texto plano
  name         String
  status       UserStatus @default(ACTIVE)
  createdAt    DateTime   @default(now()) @db.Timestamptz(3)
  updatedAt    DateTime   @updatedAt @db.Timestamptz(3)

  company      Company?   @relation(fields: [companyId], references: [id], onDelete: Cascade)
  role         Role       @relation(fields: [roleId], references: [id])
  team         Team?      @relation("TeamMembers", fields: [teamId], references: [id], onDelete: SetNull)
  managedTeams Team[]     @relation("TeamManager")
  sessions     Session[]
  timeEntries  TimeEntry[]
  activityLogs ActivityLog[]
  screenshots  Screenshot[]
  reports      Report[]

  @@index([companyId])
  @@index([roleId])
  @@index([teamId])
}

/// Equipo bajo un manager. MVP: un usuario pertenece a 0..1 equipo (teamId).
model Team {
  id        String   @id
  companyId String
  name      String
  managerId String?
  createdAt DateTime @default(now()) @db.Timestamptz(3)

  company   Company  @relation(fields: [companyId], references: [id], onDelete: Cascade)
  manager   User?    @relation("TeamManager", fields: [managerId], references: [id], onDelete: SetNull)
  members   User[]   @relation("TeamMembers")

  @@index([companyId])
  @@index([managerId])
}

// ──────────────────── PROYECTOS / TAREAS (compartidos con el agente) ────────────────────

/// Espejo de `project` del agente + companyId. id = mismo ULID en agente y nube.
model Project {
  id        String   @id
  companyId String
  name      String                         // invariante: trim no vacío (validado en servicio)
  color     String?
  archived  Boolean  @default(false)
  createdAt DateTime @default(now()) @db.Timestamptz(3)

  company   Company  @relation(fields: [companyId], references: [id], onDelete: Cascade)
  tasks     Task[]

  @@index([companyId])
}

/// Espejo de `task` del agente + companyId.
model Task {
  id        String   @id
  companyId String
  projectId String
  name      String                         // invariante: trim no vacío
  completed Boolean  @default(false)
  createdAt DateTime @default(now()) @db.Timestamptz(3)

  company   Company  @relation(fields: [companyId], references: [id], onDelete: Cascade)
  project   Project  @relation(fields: [projectId], references: [id], onDelete: Cascade)
  timeEntries TimeEntry[]

  @@index([companyId])
  @@index([projectId])
}

// ──────────────────── TIEMPO / ACTIVIDAD / CAPTURAS ────────────────────

/// Sesión de tiempo consolidada (el `time_session` del agente).
/// id = ULID del agente ⇒ idempotente en sync (upsert por id).
model TimeEntry {
  id              String   @id              // ULID del agente = idempotency key
  companyId       String
  userId          String
  taskId          String
  startedAt       BigInt                    // epoch-millis UTC (canónico del agente)
  endedAt         BigInt?                   // NULL ⇒ aún corriendo en el agente
  lastHeartbeatAt BigInt?
  isSuspect       Boolean  @default(false)  // >12 h u huérfana — excluida de reportes
  source          EntrySource @default(AGENT)
  syncedAt        DateTime @default(now()) @db.Timestamptz(3)
  // durationMs NO se persiste: se deriva (endedAt - startedAt) en reportes.

  company  Company @relation(fields: [companyId], references: [id], onDelete: Cascade)
  user     User    @relation(fields: [userId], references: [id], onDelete: Cascade)
  task     Task    @relation(fields: [taskId], references: [id], onDelete: Cascade)
  activityLogs ActivityLog[]
  screenshots  Screenshot[]

  @@index([companyId, userId, startedAt])   // reportes por usuario/fecha
  @@index([companyId, taskId, startedAt])   // reportes por tarea/proyecto
  @@index([userId, endedAt])                // "¿tiene sesión abierta?"
}

/// Ventana de actividad (teclado/mouse) que el agente muestrea.
model ActivityLog {
  id            String   @id
  companyId     String
  userId        String
  timeEntryId   String?
  windowStartAt BigInt                      // epoch-millis UTC
  windowEndAt   BigInt
  keyboardCount Int      @default(0)
  mouseCount    Int      @default(0)
  activityPct   Int      @default(0)        // 0..100
  idle          Boolean  @default(false)

  company   Company    @relation(fields: [companyId], references: [id], onDelete: Cascade)
  user      User       @relation(fields: [userId], references: [id], onDelete: Cascade)
  timeEntry TimeEntry? @relation(fields: [timeEntryId], references: [id], onDelete: SetNull)

  @@index([companyId, userId, windowStartAt])
  @@index([timeEntryId])
}

/// Metadato de captura. La imagen vive en object storage (S3/R2); acá solo URL.
model Screenshot {
  id          String   @id
  companyId   String
  userId      String
  timeEntryId String?
  capturedAt  BigInt                        // epoch-millis UTC
  storageUrl  String                        // URL firmada / key en el bucket
  thumbnailUrl String?
  blurred     Boolean  @default(false)      // política de privacidad por empresa
  deleted     Boolean  @default(false)      // borrado lógico (auditable)

  company   Company    @relation(fields: [companyId], references: [id], onDelete: Cascade)
  user      User       @relation(fields: [userId], references: [id], onDelete: Cascade)
  timeEntry TimeEntry? @relation(fields: [timeEntryId], references: [id], onDelete: SetNull)

  @@index([companyId, userId, capturedAt])
  @@index([timeEntryId])
}

// ──────────────────── REPORTES / AUTH / AUDITORÍA ────────────────────

/// Definición (o caché) de un reporte: tipo + parámetros + quién/cuándo.
model Report {
  id           String     @id
  companyId    String
  type         ReportType
  params       Json       @default("{}")    // rango, filtros (usuario/proyecto/equipo)
  generatedById String?
  generatedAt  DateTime   @default(now()) @db.Timestamptz(3)
  storageUrl   String?                       // export PDF/CSV si se materializó

  company     Company @relation(fields: [companyId], references: [id], onDelete: Cascade)
  generatedBy User?   @relation(fields: [generatedById], references: [id], onDelete: SetNull)

  @@index([companyId, type, generatedAt])
}

/// Sesión de auth = refresh token (HASHEADO). Rotable y revocable.
model Session {
  id               String   @id
  userId           String
  refreshTokenHash String                    // hash del refresh token, nunca el token
  userAgent        String?
  ip               String?
  expiresAt        DateTime @db.Timestamptz(3)
  revokedAt        DateTime? @db.Timestamptz(3)
  createdAt        DateTime @default(now()) @db.Timestamptz(3)

  user User @relation(fields: [userId], references: [id], onDelete: Cascade)

  @@index([userId])
  @@index([expiresAt])
}

/// Registro INMUTABLE de acciones sensibles (solo append).
model AuditLog {
  id          String   @id
  companyId   String?
  actorUserId String?
  action      String                         // p. ej. "user.deleted", "report.exported"
  entityType  String?
  entityId    String?
  metadata    Json     @default("{}")
  createdAt   DateTime @default(now()) @db.Timestamptz(3)

  company Company? @relation(fields: [companyId], references: [id], onDelete: SetNull)

  @@index([companyId, createdAt])
  @@index([actorUserId])
}

// ────────────────────────────── ENUMS ──────────────────────────────

enum Plan          { FREE  PRO  ENTERPRISE }
enum CompanyStatus { ACTIVE  SUSPENDED  CANCELLED }
enum UserStatus    { ACTIVE  INVITED  SUSPENDED }
enum RoleKey       { SUPER_ADMIN  COMPANY_ADMIN  MANAGER  EMPLOYEE  CLIENT_VIEWER }
enum EntrySource   { AGENT  MANUAL }
enum ReportType    { HOURS_BY_USER  HOURS_BY_PROJECT  HOURS_BY_TASK  PRODUCTIVITY }
```

---

## 3. Reglas de negocio que el schema NO expresa solo

El schema fija forma; estas reglas las hace cumplir la **capa de servicio**
(NestJS) — el doc 03 las ubica:

1. **Nombre no vacío** (`Project.name`, `Task.name`): `CHECK` en el agente; en
   Postgres se valida en el DTO/servicio (y opcionalmente un `CHECK` por migración
   SQL cruda).
2. **`Task.companyId` == `Task.project.companyId`** y **`TimeEntry` coherente con
   user/task de la misma empresa**: lo garantiza el servicio al construir el
   registro (no hay FK compuesta cruzada que lo fuerce). Defensa: el `TenantGuard`
   nunca deja escribir fuera del `companyId` del token.
3. **Idempotencia de sync**: `upsert` por `TimeEntry.id` (ULID). Reenvío del mismo
   lote → no duplica; campos mutables (`endedAt`, `isSuspect`, `lastHeartbeatAt`)
   se actualizan al cerrar la sesión.
4. **`isSuspect` excluye de reportes**: las consultas de agregación filtran
   `isSuspect = false` (regla del agente que la plataforma respeta).
5. **`Session` (refresh)**: se guarda **hash**, no el token; rotación al refrescar;
   `revokedAt` invalida (doc 02).
6. **`AuditLog` inmutable**: solo `INSERT`. Sin `update`/`delete` desde la app
   (se puede reforzar con permisos de rol Postgres más adelante).
7. **Borrado de capturas**: lógico (`Screenshot.deleted = true`) por auditabilidad;
   el purgado físico del bucket es un job aparte con su propia traza.

---

## 4. Índices: por qué estos y no más

Los índices siguen las **consultas reales**, no la corazonada:

- `TimeEntry @@index([companyId, userId, startedAt])` → reporte "horas por usuario
  en rango de fechas" (el caso más frecuente).
- `TimeEntry @@index([companyId, taskId, startedAt])` → "horas por proyecto/tarea".
- `TimeEntry @@index([userId, endedAt])` → "¿este usuario tiene sesión abierta?"
  (sync / vista en vivo).
- `@@index([companyId])` en todas las entidades de negocio → el filtro de tenant
  es omnipresente.
- Join `RolePermission` indexado por `permissionId` para resolver permisos por rol.

No se agregan índices "por si acaso": cada índice cuesta en escritura y la sync es
*write-heavy*. Se medirá con `EXPLAIN ANALYZE` sobre datos reales antes de sumar.

---

## 5. Migraciones y seeds

- **Migraciones versionadas** desde el inicio (`prisma migrate`), una intención por
  migración, sin editar migraciones aplicadas (misma disciplina que el agente con
  `rusqlite_migration`).
- **Seed obligatorio**: los 5 `Role` + el catálogo de `Permission` + el join
  `RolePermission` (la matriz del doc 00, §2). Sin seed, no hay autorización: es
  parte del esquema, no un opcional.
- **RLS** (Row-Level Security por `companyId`): anotado como migración futura, no
  en el MVP.

---

## Próximo documento

**`02-api-and-security.md`** — endpoints REST (incluido el de **sync** agente→
plataforma que hace el `upsert` idempotente), JWT + refresh + rotación, los guards
(`Auth`/`Roles`/`Tenant`/`ownership`), rate-limiting y validación. Ahí se ve **cómo
se accede** a este modelo y **quién puede** tocar qué. *(brief §9-10)*
