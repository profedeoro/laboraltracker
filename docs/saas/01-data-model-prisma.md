# LaboralTracker SaaS — Modelo de datos (Prisma + PostgreSQL)

> Documento **1** del set de arquitectura. Se construye sobre
> [`00-architecture-overview.md`](00-architecture-overview.md). Cubre el punto 8
> del brief: el modelo de datos de la **plataforma** (no del agente — el agente ya
> tiene su SQLite, ver [`../conventions/05-data-schema.md`](../conventions/05-data-schema.md)).
>
> **Alcance:** esto es **diseño**, no código a ejecutar. El `schema.prisma` real se
> escribe cuando scaffoldeemos el backend (decisión de repo aún pendiente, ver
> doc 00). Acá fijamos entidades, campos, relaciones, índices y reglas.
>
> **Revisión incorporada (2026-06-19):** tras revisión técnica externa se agregaron
> `Device`, `SyncBatch`, `CompanyTrackingSettings`; se adoptó **identidad global +
> `CompanyMember`** (multi-empresa); se reemplazó el borrado en cascada de datos
> históricos por **borrado lógico**; y se definieron restricciones únicas de
> negocio. Ver §6 "Registro de decisiones".

---

## 0. Cuatro decisiones que gobiernan todo el modelo

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
| `createdAt`, `updatedAt`, `generatedAt`, `expiresAt`, `revokedAt`, `syncedAt`, `deletedAt`, `lastSeenAt` | **Plataforma** | `DateTime @db.Timestamptz(3)` | Son metadatos del servidor; `@default(now())` es idiomático y se ve bien en consultas/admin |

**Regla:** la duración (`durationMs`) **no se persiste**; se deriva en la consulta
de reportes (`endedAt - startedAt`), respetando la misma política del agente. La
zona horaria es **presentación** (se resuelve en el frontend / parámetro de
reporte), nunca almacenamiento.

> Nota Prisma/JS: `BigInt` evita el techo de `2^53` de `number` y es lo correcto
> para millis; el borde de serialización JSON (BigInt→string) se maneja en la capa
> de presentación (igual que el agente hace `#[ts(type = "number")]`).

### 0.3 Multi-tenant: identidad global + membresía por empresa

**Decisión (2026-06-19):** una persona puede pertenecer a **varias empresas**
(paridad con Time Doctor: freelancers/consultores). Por eso:

- **`User` = identidad global** del SaaS: `email` único **global**, sin `companyId`,
  sin rol. Una persona = un `User`.
- **`CompanyMember` = pertenencia a una empresa**: `userId` + `companyId` + `roleId`
  (+ `teamId` opcional). El **rol vive en la membresía**, no en el usuario: la misma
  persona puede ser `MANAGER` en la empresa A y `EMPLOYEE` en la B.
- Toda entidad de negocio (`Project`, `Task`, `TimeEntry`, …) lleva `companyId` **e
  índice por `companyId`**. El aislamiento se garantiza en **dos capas**:
  1. **Aplicación:** un `TenantGuard` resuelve el `companyId` del contexto del token,
     verifica que el `User` tenga una `CompanyMember` activa en esa empresa y filtra
     todo por ese `companyId` (doc 02 de seguridad).
  2. **Datos (futuro):** Postgres **Row-Level Security** por `companyId` como defensa
     en profundidad. **Anotado, no activado** en el MVP (proporción: primero el
     guard; RLS cuando el volumen/criticidad lo pidan).

### 0.4 Borrado lógico para datos históricos (no cascada)

**Decisión (2026-06-19):** los datos con valor legal/operativo **no se borran
físicamente** y **no se eliminan en cascada**. Una app laboral no puede evaporar
historial al dar de baja una empresa o un usuario.

- **Soft-delete** (`deletedAt DateTime?` + `status` donde aplica) en: `Company`,
  `User`, `CompanyMember`, `Project`, `Task`, `TimeEntry`, `Screenshot`.
- **`onDelete` por categoría:**
  - **Histórico** (`TimeEntry`, `ActivityLog`, `Screenshot`, `Report`, `AuditLog`):
    el padre usa **`Restrict`** → un `DELETE` físico del padre se **bloquea**; la
    baja es siempre lógica. `AuditLog` además nunca cuelga en cascada (`SetNull`).
  - **Auth/efímero** (`Session`, `Device`): `Cascade` al borrar el `User` es
    aceptable (no es "historial" legal; igual el `User` se da de baja lógica).
  - **Join/membresía** (`CompanyMember`, `RolePermission`): `Cascade` aceptable.
- Las consultas de lectura **siempre** filtran `deletedAt IS NULL` (se centraliza en
  el repositorio para no olvidarlo — doc 03).

---

## 1. Mapa de relaciones (alto nivel)

```txt
User (identidad global)
 └─< CompanyMember >── Company (tenant raíz)
        │  (roleId)──→ Role ──< RolePermission >── Permission
        │  (teamId)──→ Team   (Team.managerId ─→ CompanyMember)
        │
User ──┬─ Session   (refresh tokens, atados a un Device)
       ├─ Device    (instalaciones del agente: equipos autorizados)
       ├─ TimeEntry  ─→ Task ─→ Project        (+ deviceId, syncBatchId)
       ├─ ActivityLog ─→ TimeEntry             (+ deviceId, syncBatchId)
       └─ Screenshot  ─→ TimeEntry             (+ deviceId, syncBatchId)

Company ──┬─ CompanyTrackingSettings (1:1, política de captura tipada)
          ├─ Project ──< Task
          ├─ SyncBatch (lotes recibidos del agente — auditoría de sync)
          ├─ Report
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

/// Raíz del tenant. Todo dato de negocio cuelga de una Company. Baja lógica.
model Company {
  id        String   @id                 // ULID, provisto por el servicio
  name      String
  slug      String   @unique             // identificador URL-safe
  plan      Plan     @default(FREE)
  status    CompanyStatus @default(ACTIVE)
  settings  Json     @default("{}")      // SOLO cosméticos (tema, locale). La
                                         // política de captura va tipada (abajo).
  createdAt DateTime @default(now()) @db.Timestamptz(3)
  updatedAt DateTime @updatedAt @db.Timestamptz(3)
  deletedAt DateTime? @db.Timestamptz(3)

  trackingSettings CompanyTrackingSettings?
  members   CompanyMember[]
  teams     Team[]
  projects  Project[]
  tasks     Task[]
  timeEntries  TimeEntry[]
  activityLogs ActivityLog[]
  screenshots  Screenshot[]
  devices   Device[]
  syncBatches SyncBatch[]
  reports   Report[]
  auditLogs AuditLog[]
}

/// Política de captura/tracking TIPADA (no JSON): tiene peso legal y maneja al
/// agente. Cada empresa decide qué se captura y cómo. 1:1 con Company.
model CompanyTrackingSettings {
  id                    String  @id
  companyId             String  @unique
  screenshotsEnabled    Boolean @default(false)
  screenshotIntervalMin Int     @default(10)
  blurScreenshots       Boolean @default(false)
  activityTracking      Boolean @default(true)   // teclado/mouse (conteos, no keylogging)
  idleThresholdMin      Int     @default(5)
  retentionDays         Int     @default(90)     // a cuántos días se purga lo histórico
  updatedAt             DateTime @updatedAt @db.Timestamptz(3)

  company Company @relation(fields: [companyId], references: [id], onDelete: Cascade)
}

// ────────────────────────── IDENTIDAD / MEMBRESÍA ──────────────────────────

/// Identidad GLOBAL del SaaS. Sin companyId ni rol: eso vive en CompanyMember.
/// Una persona puede pertenecer a varias empresas con roles distintos.
model User {
  id           String     @id              // ULID
  email        String     @unique          // identidad global
  passwordHash String                       // bcrypt/argon2 — nunca texto plano
  name         String
  status       UserStatus @default(ACTIVE)
  createdAt    DateTime   @default(now()) @db.Timestamptz(3)
  updatedAt    DateTime   @updatedAt @db.Timestamptz(3)
  deletedAt    DateTime?  @db.Timestamptz(3)

  memberships  CompanyMember[]
  sessions     Session[]
  devices      Device[]
  syncBatches  SyncBatch[]
  timeEntries  TimeEntry[]
  activityLogs ActivityLog[]
  screenshots  Screenshot[]
  reports      Report[]
}

/// Pertenencia de un User a una Company, con su rol (y equipo) EN esa empresa.
model CompanyMember {
  id        String       @id
  userId    String
  companyId String
  roleId    String
  teamId    String?
  status    MemberStatus @default(ACTIVE)
  createdAt DateTime     @default(now()) @db.Timestamptz(3)
  deletedAt DateTime?    @db.Timestamptz(3)

  user      User    @relation(fields: [userId], references: [id], onDelete: Cascade)
  company   Company @relation(fields: [companyId], references: [id], onDelete: Cascade)
  role      Role    @relation(fields: [roleId], references: [id])
  team      Team?   @relation("TeamMembers", fields: [teamId], references: [id], onDelete: SetNull)
  managedTeams Team[] @relation("TeamManager")

  @@unique([userId, companyId])            // una membresía por persona/empresa
  @@index([companyId])
  @@index([roleId])
  @@index([teamId])
}

// ────────────────────────────── RBAC ──────────────────────────────

/// Rol. MVP: 5 roles globales fijos (RoleKey). companyId queda como SEAM para
/// roles personalizados por empresa en el futuro (ver §6, decisión diferida).
model Role {
  id          String  @id
  companyId   String?                       // NULL = rol global (todos en MVP)
  key         RoleKey
  name        String
  description String?
  members     CompanyMember[]
  permissions RolePermission[]

  @@unique([companyId, key])               // global hoy; per-empresa mañana
}

/// Capacidad atómica, p. ej. "time.read", "user.manage", "report.view".
model Permission {
  id          String  @id
  key         String  @unique             // dominio.acción
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

// ────────────────────────────── EQUIPOS ──────────────────────────────

/// Equipo bajo un manager. Manager y miembros son CompanyMember (company-scoped).
model Team {
  id        String   @id
  companyId String
  name      String
  managerId String?                         // → CompanyMember
  createdAt DateTime @default(now()) @db.Timestamptz(3)
  deletedAt DateTime? @db.Timestamptz(3)

  company   Company  @relation(fields: [companyId], references: [id], onDelete: Restrict)
  manager   CompanyMember? @relation("TeamManager", fields: [managerId], references: [id], onDelete: SetNull)
  members   CompanyMember[] @relation("TeamMembers")

  @@unique([companyId, name])
  @@index([companyId])
  @@index([managerId])
}

// ──────────────────── DISPOSITIVOS / SYNC ────────────────────

/// Instalación del agente (un equipo). Un User puede tener varios Devices.
/// Permite autorizar/revocar equipos y saber versión y última conexión.
model Device {
  id           String   @id              // ULID
  companyId    String
  userId       String
  name         String?                    // "Laptop trabajo"
  platform     DevicePlatform
  appVersion   String?
  lastSeenAt   DateTime? @db.Timestamptz(3)
  authorizedAt DateTime? @db.Timestamptz(3)   // NULL ⇒ pendiente de aprobación
  revokedAt    DateTime? @db.Timestamptz(3)   // ≠ NULL ⇒ no puede sincronizar
  createdAt    DateTime @default(now()) @db.Timestamptz(3)

  company   Company @relation(fields: [companyId], references: [id], onDelete: Restrict)
  user      User    @relation(fields: [userId], references: [id], onDelete: Cascade)
  sessions  Session[]
  syncBatches SyncBatch[]

  @@index([companyId, userId])
}

/// Lote de sincronización recibido del agente. Auditoría: qué/cuándo/si falló.
/// Responde "no aparecen mis horas" sin adivinar.
model SyncBatch {
  id          String   @id              // ULID generado por el agente (idempotente)
  companyId   String
  userId      String
  deviceId    String
  status      SyncStatus @default(RECEIVED)
  itemCount   Int      @default(0)
  receivedAt  DateTime @default(now()) @db.Timestamptz(3)
  processedAt DateTime? @db.Timestamptz(3)
  error       String?

  company Company @relation(fields: [companyId], references: [id], onDelete: Restrict)
  user    User    @relation(fields: [userId], references: [id], onDelete: Cascade)
  device  Device  @relation(fields: [deviceId], references: [id], onDelete: Cascade)

  @@index([companyId, userId, receivedAt])
  @@index([deviceId])
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
  deletedAt DateTime? @db.Timestamptz(3)

  company   Company  @relation(fields: [companyId], references: [id], onDelete: Restrict)
  tasks     Task[]

  @@unique([companyId, name])              // ver nota de consistencia con el agente (§3.8)
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
  deletedAt DateTime? @db.Timestamptz(3)

  company   Company  @relation(fields: [companyId], references: [id], onDelete: Restrict)
  project   Project  @relation(fields: [projectId], references: [id], onDelete: Restrict)
  timeEntries TimeEntry[]

  @@unique([projectId, name])              // ver §3.8
  @@index([companyId])
  @@index([projectId])
}

// ──────────────────── TIEMPO / ACTIVIDAD / CAPTURAS ────────────────────

/// Intervalo de trabajo registrado. Un `time_session` del agente consolidado es
/// UNA fuente (`source=AGENT`) de un `TimeEntry`; la plataforma también guarda filas
/// `source=MANUAL` sin sesión del agente detrás (sin heartbeat/device). Por eso el
/// nombre difiere del agente a propósito — es un superconjunto (ADR 0009).
/// id = ULID del agente ⇒ idempotente en sync (upsert por id). Baja lógica.
model TimeEntry {
  id              String   @id              // ULID del agente = idempotency key
  companyId       String
  userId          String
  taskId          String
  deviceId        String?
  syncBatchId     String?
  startedAt       BigInt                    // epoch-millis UTC (canónico del agente)
  endedAt         BigInt?                   // NULL ⇒ aún corriendo en el agente
  lastHeartbeatAt BigInt?
  isSuspect       Boolean  @default(false)  // >12 h u huérfana — excluida de reportes
  source          EntrySource @default(AGENT)
  syncedAt        DateTime @default(now()) @db.Timestamptz(3)
  deletedAt       DateTime? @db.Timestamptz(3)
  // durationMs NO se persiste: se deriva (endedAt - startedAt) en reportes.

  company  Company @relation(fields: [companyId], references: [id], onDelete: Restrict)
  user     User    @relation(fields: [userId], references: [id], onDelete: Restrict)
  task     Task    @relation(fields: [taskId], references: [id], onDelete: Restrict)
  activityLogs ActivityLog[]
  screenshots  Screenshot[]

  @@index([companyId, userId, startedAt])   // reportes por usuario/fecha
  @@index([companyId, taskId, startedAt])   // reportes por tarea/proyecto
  @@index([userId, endedAt])                // "¿tiene sesión abierta?"
}

/// Ventana de actividad (teclado/mouse) que el agente muestrea. Conteos, NO keylogging.
model ActivityLog {
  id            String   @id
  companyId     String
  userId        String
  timeEntryId   String?
  deviceId      String?
  syncBatchId   String?
  windowStartAt BigInt                      // epoch-millis UTC
  windowEndAt   BigInt
  keyboardCount Int      @default(0)
  mouseCount    Int      @default(0)
  activityPct   Int      @default(0)        // 0..100
  idle          Boolean  @default(false)

  company   Company    @relation(fields: [companyId], references: [id], onDelete: Restrict)
  user      User       @relation(fields: [userId], references: [id], onDelete: Restrict)
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
  deviceId    String?
  syncBatchId String?
  capturedAt  BigInt                        // epoch-millis UTC
  storageUrl  String                        // URL firmada / key en el bucket
  thumbnailUrl String?
  blurred     Boolean  @default(false)      // política de privacidad por empresa
  deletedAt   DateTime? @db.Timestamptz(3)  // borrado lógico (auditable); purga física = job

  company   Company    @relation(fields: [companyId], references: [id], onDelete: Restrict)
  user      User       @relation(fields: [userId], references: [id], onDelete: Restrict)
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

  company     Company @relation(fields: [companyId], references: [id], onDelete: Restrict)
  generatedBy User?   @relation(fields: [generatedById], references: [id], onDelete: SetNull)

  @@index([companyId, type, generatedAt])
}

/// Sesión de auth = refresh token (HASHEADO). Rotable, revocable, atada a un Device.
model Session {
  id               String   @id
  userId           String
  deviceId         String?
  refreshTokenHash String                    // hash del refresh token, nunca el token
  userAgent        String?
  ip               String?
  expiresAt        DateTime @db.Timestamptz(3)
  revokedAt        DateTime? @db.Timestamptz(3)
  createdAt        DateTime @default(now()) @db.Timestamptz(3)

  user   User    @relation(fields: [userId], references: [id], onDelete: Cascade)
  device Device? @relation(fields: [deviceId], references: [id], onDelete: SetNull)

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

enum Plan           { FREE  PRO  ENTERPRISE }
enum CompanyStatus  { ACTIVE  SUSPENDED  CANCELLED }
enum UserStatus     { ACTIVE  INVITED  SUSPENDED }
enum MemberStatus   { ACTIVE  INVITED  SUSPENDED }
enum RoleKey        { SUPER_ADMIN  COMPANY_ADMIN  MANAGER  EMPLOYEE  CLIENT_VIEWER }
enum EntrySource    { AGENT  MANUAL }
enum ReportType     { HOURS_BY_USER  HOURS_BY_PROJECT  HOURS_BY_TASK  PRODUCTIVITY }
enum DevicePlatform { WINDOWS  MACOS  LINUX }
enum SyncStatus     { RECEIVED  PROCESSING  DONE  FAILED }
```

---

## 3. Reglas de negocio que el schema NO expresa solo

El schema fija forma; estas reglas las hace cumplir la **capa de servicio**
(NestJS) — el doc 03 las ubica:

1. **Nombre no vacío** (`Project.name`, `Task.name`): `CHECK` en el agente; en
   Postgres se valida en el DTO/servicio (y opcionalmente un `CHECK` por migración
   SQL cruda).
2. **Coherencia de tenant**: `Task.companyId == Task.project.companyId`; `TimeEntry`
   coherente con user/task de la misma empresa; `Team.manager` y `Team.members` son
   `CompanyMember` de **esa** empresa. No hay FK compuesta cruzada que lo fuerce →
   lo garantiza el servicio + el `TenantGuard` (nunca escribe fuera del `companyId`
   del token).
3. **Membresía única**: `@@unique([userId, companyId])` en `CompanyMember`. El **rol
   se resuelve por membresía**, no por usuario (una persona puede tener roles
   distintos en empresas distintas).
4. **Idempotencia de sync**: `upsert` por `TimeEntry.id` (ULID). Reenvío del mismo
   lote → no duplica; campos mutables (`endedAt`, `isSuspect`, `lastHeartbeatAt`)
   se actualizan al cerrar la sesión. Cada push queda registrado en `SyncBatch`.
5. **Autorización de dispositivo**: el sync solo se acepta de un `Device` con
   `authorizedAt != NULL` y `revokedAt == NULL`. Equipo revocado → 403 y no
   sincroniza (doc 02).
6. **`isSuspect` excluye de reportes**: las consultas de agregación filtran
   `isSuspect = false` (regla del agente que la plataforma respeta).
7. **Borrado lógico**: las lecturas filtran `deletedAt IS NULL` (centralizado en el
   repositorio). `AuditLog` es solo `INSERT` (sin update/delete desde la app). El
   purgado físico de capturas/históricos vencidos (`retentionDays`) es un **job
   aparte** con su propia traza.
8. **Unicidad de nombre y consistencia con el agente** (sutil pero importante): la
   plataforma exige `@@unique([companyId, name])` / `@@unique([projectId, name])`,
   pero el **agente (SQLite) NO tiene esa restricción** hoy
   ([05-data-schema](../conventions/05-data-schema.md)). Si un agente empuja un
   nombre duplicado, la sync fallaría. **Resolución:** alinear el agente (agregar el
   índice único, idealmente parcial sobre no-archivados/no-borrados) **o** que el
   endpoint de sync resuelva el choque con una política definida (rechazar vs
   renombrar). Se decide en el doc 02; acá queda **flagueado**, no resuelto.
9. **`Session` (refresh)**: se guarda **hash**, no el token; rotación al refrescar;
   `revokedAt` invalida; atada a un `Device` (doc 02).

---

## 4. Índices: por qué estos y no más

Los índices siguen las **consultas reales**, no la corazonada:

- `TimeEntry @@index([companyId, userId, startedAt])` → reporte "horas por usuario
  en rango de fechas" (el caso más frecuente).
- `TimeEntry @@index([companyId, taskId, startedAt])` → "horas por proyecto/tarea".
- `TimeEntry @@index([userId, endedAt])` → "¿este usuario tiene sesión abierta?"
- `CompanyMember @@unique([userId, companyId])` → resolver membresía/rol en cada
  request (lo hace el `TenantGuard`).
- `SyncBatch @@index([companyId, userId, receivedAt])` → auditar la sync de un
  usuario en el tiempo.
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
- **Seed obligatorio**: los 5 `Role` (globales) + el catálogo de `Permission` + el
  join `RolePermission` (la matriz del doc 00, §2). Sin seed, no hay autorización:
  es parte del esquema, no un opcional.
- **RLS** (Row-Level Security por `companyId`): anotado como migración futura, no
  en el MVP.

---

## 6. Registro de decisiones (revisión técnica 2026-06-19)

Decisiones tomadas tras revisión externa, para trazabilidad (el *porqué* completo
puede pasar a un ADR al scaffoldear):

| # | Punto | Decisión | Estado |
|---|-------|----------|--------|
| 1 | Dispositivos del agente | Modelo `Device` (autorizar/revocar, versión, last-seen) | **Incorporado** |
| 2 | Lotes de sync | Modelo `SyncBatch` (auditoría de sync, debug de "faltan horas") | **Incorporado** |
| 3 | Cascada peligrosa | Soft-delete + `Restrict`/`SetNull` en histórico (§0.4) | **Incorporado** |
| 4 | Identidad de usuario | **Multi-empresa**: `User` global + `CompanyMember` | **Incorporado** |
| 5 | Roles fijos vs custom | 5 roles fijos en MVP; `@@unique([companyId, key])` deja el *seam* para roles por empresa | **Diferido (con seam)** |
| 6 | Uniques de negocio | `@@unique` en project/task (con nota de consistencia con el agente, §3.8) | **Incorporado** |
| 7 | `settings Json` riesgoso | Política de captura → `CompanyTrackingSettings` tipado; JSON solo para cosmético | **Incorporado** |
| 8 | Privacidad | Documento propio del producto | **Planificado** → nuevo doc del set |

---

## Próximos documentos

- **`02-api-and-security.md`** — endpoints REST (incluido el de **sync** agente→
  plataforma: `upsert` idempotente, autorización de `Device`, registro en
  `SyncBatch`, y la política de choque de nombres del §3.8), JWT + refresh +
  rotación, los guards (`Auth`/`Roles`/`Tenant`/`ownership`), rate-limiting y
  validación. *(brief §9-10)*
- **`03-privacy-policy.md`** (nuevo, por la revisión) — qué se captura y qué no,
  quién lo ve, retención (`retentionDays`), cómo se elimina, cómo se informa al
  empleado, y las configuraciones por empresa. Producto + legal, no solo técnico.
