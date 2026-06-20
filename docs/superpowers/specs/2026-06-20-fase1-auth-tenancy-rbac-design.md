# Fase 1 — Auth + Tenancy + RBAC — Diseño

> Spec de la **Fase 1** del roadmap SaaS ([docs/saas/06-roadmap.md](../../saas/06-roadmap.md)).
> Construye el esqueleto multi-tenant del que cuelga todo: identidad, membresía,
> RBAC y los guards. Se apoya en [01-data-model-prisma](../../saas/01-data-model-prisma.md),
> [02-api-and-security](../../saas/02-api-and-security.md) y los ADR
> [0006](../../decisions/0006-permissions-in-access-token.md) /
> [0009](../../decisions/0009-time-naming-seam-agent-vs-platform.md).
>
> **Alcance:** diseño. La implementación va por slices (1a/1b/1c), cada uno su PR.

## Objetivo

Levantar auth + tenancy + RBAC en `apps/api` (NestJS + Prisma + Postgres), con el
**test e2e de aislamiento entre empresas** como criterio de éxito no negociable.

## No-objetivos (YAGNI — fases siguientes)

- **Registro/onboarding** (crear empresa + admin): Fase 2. La Fase 1 usa **datos
  sembrados** (seed).
- CRUD de dominio (projects, tasks), reportes, sync, devices, screenshots.
- `OwnershipGuard` completo (self vs equipo): se deja el seam; se materializa
  cuando haya recursos propios que proteger (Fase 2+).
- Roles personalizados por empresa (seam ya en el modelo; 5 roles fijos por ahora).

## Decisiones

1. **Tres slices:** 1a (datos+seed), 1b (auth), 1c (guards+RBAC+test de aislamiento).
2. **Modelo = subconjunto Fase 1**, no todo el doc 01 (el resto llega en sus fases).
3. **Hashing: `argon2id`** (recomendación OWASP; mejor que bcrypt). Dependencia
   nativa → aprobación vía `allowBuilds` en `pnpm-workspace.yaml`.
4. **CI gana un servicio Postgres** en el job `platform` para correr migración +
   e2e (hoy no tiene DB).

### Refinamientos por revisión técnica (2026-06-20)

- **Refresh con contexto de empresa.** Como los permisos son estables **por
  membresía** (no por usuario) y `switch-company` recalcula el JWT, el refresh se
  ata a `(userId, companyId)`, no solo a `userId`. **`Session` lleva `companyId`**
  (refina el modelo del doc 01). La rotación y la detección de reuso operan sobre
  la sesión company-scoped: un refresh emitido "en Empresa A" solo acuña access
  tokens de A.
- **Passwords del seed visibles.** Constante documentada en el seed
  (`DEV_SEED_PASSWORD`) + override opcional por `SEED_PASSWORD`; referenciada en
  este spec. No se esconden.

## Slice 1a — Datos (Prisma + modelo + migración + seed)

**Stack:** `prisma` + `@prisma/client`. Schema en `apps/api/prisma/schema.prisma`.
`DATABASE_URL` desde env (`.env`, ejemplo en `.env.example`).

**Modelos (subconjunto):** `Company`, `User`, `CompanyMember`, `Team`, `Role`,
`Permission`, `RolePermission`, `Session` (+`companyId`), enums `Plan`,
`CompanyStatus`, `UserStatus`, `MemberStatus`, `RoleKey`. Campos/relaciones según
doc 01, con estas notas:

- `Session`: `id, userId, companyId, deviceId?(null en Fase 1), refreshTokenHash,
  expiresAt, revokedAt?, createdAt`. **`companyId` agregado** (refinamiento).
- PKs `String @id` (ULID provisto por el servicio), sin `@default`.
- `Company`/`User` solo con las relaciones a modelos que existen en Fase 1
  (members, teams, sessions). Las demás relaciones se agregan en sus fases.

**Migración:** `prisma migrate dev --name init_auth_tenancy` (versionada).

**Seed** (`apps/api/prisma/seed.ts`):
- 5 `Role` (globales) + catálogo de `Permission` + `RolePermission` (matriz doc 00 §2).
- **2 empresas demo** (`Empresa A`, `Empresa B`) con: 1 `COMPANY_ADMIN` + 1
  `EMPLOYEE` cada una, sus `CompanyMember`. Password de todos = `DEV_SEED_PASSWORD`
  (constante documentada; override por `SEED_PASSWORD`), hasheado con argon2id.
- Comentario en el seed con los emails y el password, p. ej.:
  `// seed login: admin@a.demo / admin@b.demo / employee@a.demo … pass = DEV_SEED_PASSWORD`.

**Verificación 1a:** `prisma migrate` aplica; `prisma db seed` corre; una query
confirma 2 empresas, 5 roles, los permisos y las membresías.

## Slice 1b — Auth (login / refresh / switch-company / me / logout)

**Módulo `auth/`** + `ConfigModule` con validación de env al arrancar
(`DATABASE_URL`, `JWT_SECRET`, `ACCESS_TTL`, `REFRESH_TTL`).

- **Access token:** JWT (`@nestjs/jwt`), TTL corto. Claims: `sub`(userId),
  `email`, `companyId`, `roleKey`, `permissions: string[]` del rol de la membresía
  activa (ADR 0006/0009).
- **Refresh:** opaco (`crypto.randomBytes`), **hasheado** (argon2) en `Session`.
  Atado a `(userId, companyId)`.
- **`POST /auth/login`** `{email, password}` → verifica argon2 → resuelve
  membresías → fija empresa activa (única membresía → esa; si hay varias → la
  primera por ahora, `switch-company` cambia) → crea `Session(userId, companyId)` →
  devuelve access + refresh.
- **`POST /auth/refresh`** → ubica `Session` por hash → si `revokedAt` ≠ null →
  **reuso detectado** → revoca toda la cadena de esa `(userId, companyId)` → 401.
  Si válida → **rota** (nuevo refresh, revoca el viejo) → nuevo access para **el
  mismo `companyId`**.
- **`POST /auth/switch-company`** `{companyId}` → verifica `CompanyMember` activa en
  el target → crea `Session` para esa empresa → devuelve nuevo par (la sesión de la
  empresa anterior sigue válida; multi-empresa simultánea permitido).
- **`GET /me`** → identidad + lista de membresías (empresa + rol) + empresa activa.
- **`POST /auth/logout`** → revoca la `Session` actual.

**Verificación 1b:** tests unitarios del `AuthService` (hash ok/fallo, rotación,
reuso → revoca cadena, switch a empresa sin membresía → rechazo).

## Slice 1c — RBAC, guards y test de aislamiento

**Guards (`common/guards/`), en orden:**
1. `AuthGuard` — valida el JWT (firma + no expirado). Inyecta `AuthCtx`
   (`userId, companyId, roleKey, permissions`).
2. `PermissionGuard` — lee `@RequirePermission('...')` y lo compara contra
   `permissions` del token (permisos como datos, OCP).
3. `TenantGuard` — confirma `CompanyMember` activa del `userId` en `companyId`;
   deja el `companyId` disponible para que el repositorio filtre.

(`OwnershipGuard` queda como seam — sin recursos propios que proteger aún.)

**Endpoint tenant-scoped:** `GET /members` → lista los `CompanyMember` de la
**empresa activa** del token (con `@RequirePermission('member.read')`,
`AuthGuard`+`PermissionGuard`+`TenantGuard`).

**El test de aislamiento (e2e, obligatorio):**
1. Login admin de Empresa A → `GET /members` devuelve **solo** miembros de A.
2. Login admin de Empresa B → `GET /members` devuelve **solo** miembros de B.
3. Un token de A **no** puede ver datos de B (el `TenantGuard` filtra por el
   `companyId` del token; pedir un recurso de B → 403/404).
4. `EMPLOYEE` sin `member.read` → 403 en `GET /members` (RBAC funciona).

Este test protege el invariante multi-tenant. Si no pasa, la Fase 1 no cierra.

## Estrategia de test + CI

- **Local:** e2e corre contra el Postgres dockerizado, base `laboraltracker_test`
  (separada de dev). `prisma migrate deploy` + seed antes; estado limpio por corrida.
- **CI:** el job `platform` gana un **service `postgres:16`**; pasos: `pnpm install`
  → `prisma migrate deploy` → `prisma db seed` → `pnpm --filter api test` (unit) →
  `pnpm --filter api test:e2e` (incluye el aislamiento). `DATABASE_URL` apunta al
  service.

## Slicing en PRs

| Slice | Entrega | Verificación |
|-------|---------|--------------|
| **1a** | Prisma + modelo + migración + seed | migración aplica, seed corre, query confirma datos |
| **1b** | `auth/` (login/refresh/switch/me/logout) | tests unitarios del AuthService |
| **1c** | guards + `GET /members` + e2e aislamiento + CI con Postgres | **e2e de aislamiento verde en CI** |

Cada slice su rama `feature/…` y PR. 1a→merge→1b→merge→1c.

## Riesgos

| Riesgo | Mitigación |
|--------|-----------|
| argon2 nativo no compila (Windows/pnpm) | `allowBuilds` (patrón ya conocido); fallback `@node-rs/argon2` (prebuilt) si node-gyp falla |
| e2e en CI sin DB | service `postgres:16` en el job platform |
| Refresh cruza contexto de empresa | `Session.companyId` + rotación/reuso company-scoped (refinamiento) |
| Seed passwords no descubribles | constante documentada en seed + spec + override `SEED_PASSWORD` |
