# Fase 1a — Capa de datos (Prisma + modelo + migración + seed) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Levantar la capa de datos de la plataforma en `apps/api`: Prisma + el subconjunto Fase 1 del modelo + migración versionada + seed con 2 empresas demo, para que 1b (auth) y 1c (guards/test) tengan sobre qué construir.

**Architecture:** PostgreSQL vía Prisma. `PrismaService` (provider NestJS) envuelve `PrismaClient`. El schema vive en `apps/api/prisma/schema.prisma`; el seed en `apps/api/prisma/seed.ts`. Hashing de passwords del seed con `@node-rs/argon2` (binarios precompilados, sin node-gyp).

**Tech Stack:** Prisma · `@prisma/client` · `@node-rs/argon2` · `ulid` · PostgreSQL 16 (docker-compose).

**Spec:** [docs/superpowers/specs/2026-06-20-fase1-auth-tenancy-rbac-design.md](../specs/2026-06-20-fase1-auth-tenancy-rbac-design.md)

---

## Entorno de ejecución (LEER antes de empezar)

- **OS:** Windows / PowerShell. También está la herramienta Bash (git bash).
- **pnpm** no está en el PATH por defecto de cada shell: anteponer en PowerShell
  `$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"` antes de cada `pnpm`.
- **Docker** para Postgres: anteponer `$env:Path = "C:\Program Files\Docker\Docker\resources\bin;$env:Path"` antes de cada `docker`.
- **Build scripts (pnpm 11):** al instalar deps nuevas con scripts de build, pnpm
  las ignora y falla con `ERR_PNPM_IGNORED_BUILDS`. Se aprueban en
  `pnpm-workspace.yaml` con `allowBuilds: <pkg>: true` (NO `onlyBuiltDependencies`).
- **`.env` está BLOQUEADO** para las herramientas de escritura del asistente; el
  archivo `apps/api/.env` lo crea el usuario (Task 1, Step 2).
- **Postgres dev** debe estar arriba: `docker compose up -d postgres` (healthy).
- Commits convencionales, sin trailer de IA. Rama propia `feature/fase1a-data`.

---

## File Structure

- Create: `apps/api/prisma/schema.prisma` — datasource + el modelo Fase 1.
- Create: `apps/api/prisma/seed.ts` — roles/permisos + 2 empresas demo.
- Create: `apps/api/src/database/prisma.service.ts` — `PrismaService` (provider).
- Create: `apps/api/src/database/prisma.module.ts` — módulo global.
- Modify: `apps/api/package.json` — deps + script `prisma` + bloque `prisma.seed`.
- Modify: `pnpm-workspace.yaml` — `allowBuilds` para los nuevos build scripts.
- Modify: `apps/api/src/app.module.ts` — importar `PrismaModule`.
- User-create: `apps/api/.env` — `DATABASE_URL` (no lo puede escribir el asistente).

---

### Task 1: Dependencias + entorno

**Files:**
- Modify: `apps/api/package.json`
- Modify: `pnpm-workspace.yaml`
- User-create: `apps/api/.env`

- [ ] **Step 1: Agregar dependencias a `apps/api`**

Desde la raíz del repo (PowerShell con pnpm en PATH):

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm --filter api add @prisma/client @node-rs/argon2 ulid
pnpm --filter api add -D prisma
```

Expected: `apps/api/package.json` lista `@prisma/client`, `@node-rs/argon2`, `ulid`
en dependencies y `prisma` en devDependencies.

- [ ] **Step 2: (USUARIO) Crear `apps/api/.env`**

El asistente no puede escribir `.env*`. Crear `apps/api/.env` con:

```dotenv
DATABASE_URL="postgresql://laboraltracker:laboraltracker@localhost:5432/laboraltracker?schema=public"
```

- [ ] **Step 3: Aprobar build scripts e instalar**

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm install
```

Si falla con `ERR_PNPM_IGNORED_BUILDS`, agregar los paquetes señalados a
`pnpm-workspace.yaml` bajo `allowBuilds` con valor `true` (junto a los ya
aprobados), por ejemplo:

```yaml
allowBuilds:
  sharp: true
  unrs-resolver: true
  "@prisma/client": true
  "@prisma/engines": true
  prisma: true
  esbuild: true
```

Reintentar `pnpm install` hasta `EXIT 0`.

- [ ] **Step 4: Verificar Postgres arriba**

```powershell
$env:Path = "C:\Program Files\Docker\Docker\resources\bin;$env:Path"
docker compose up -d postgres
docker compose ps
```

Expected: `laboraltracker-db` en estado `healthy`.

---

### Task 2: Schema Prisma (modelo Fase 1)

**Files:**
- Create: `apps/api/prisma/schema.prisma`

- [ ] **Step 1: Escribir `apps/api/prisma/schema.prisma`**

```prisma
generator client {
  provider = "prisma-client-js"
}

datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

// ────────────────────────────── TENANT ──────────────────────────────

model Company {
  id        String        @id
  name      String
  slug      String        @unique
  plan      Plan          @default(FREE)
  status    CompanyStatus @default(ACTIVE)
  createdAt DateTime      @default(now()) @db.Timestamptz(3)
  updatedAt DateTime      @updatedAt @db.Timestamptz(3)
  deletedAt DateTime?     @db.Timestamptz(3)

  members  CompanyMember[]
  teams    Team[]
  sessions Session[]
}

// ──────────────────── IDENTIDAD / MEMBRESÍA ────────────────────

model User {
  id           String     @id
  email        String     @unique
  passwordHash String
  name         String
  status       UserStatus @default(ACTIVE)
  createdAt    DateTime   @default(now()) @db.Timestamptz(3)
  updatedAt    DateTime   @updatedAt @db.Timestamptz(3)
  deletedAt    DateTime?  @db.Timestamptz(3)

  memberships CompanyMember[]
  sessions    Session[]
}

model CompanyMember {
  id        String       @id
  userId    String
  companyId String
  roleId    String
  teamId    String?
  status    MemberStatus @default(ACTIVE)
  createdAt DateTime     @default(now()) @db.Timestamptz(3)
  deletedAt DateTime?    @db.Timestamptz(3)

  user         User   @relation(fields: [userId], references: [id], onDelete: Cascade)
  company      Company @relation(fields: [companyId], references: [id], onDelete: Cascade)
  role         Role   @relation(fields: [roleId], references: [id])
  team         Team?  @relation("TeamMembers", fields: [teamId], references: [id], onDelete: SetNull)
  managedTeams Team[] @relation("TeamManager")

  @@unique([userId, companyId])
  @@index([companyId])
  @@index([roleId])
  @@index([teamId])
}

model Team {
  id        String    @id
  companyId String
  name      String
  managerId String?
  createdAt DateTime  @default(now()) @db.Timestamptz(3)
  deletedAt DateTime? @db.Timestamptz(3)

  company Company        @relation(fields: [companyId], references: [id], onDelete: Restrict)
  manager CompanyMember? @relation("TeamManager", fields: [managerId], references: [id], onDelete: SetNull)
  members CompanyMember[] @relation("TeamMembers")

  @@unique([companyId, name])
  @@index([companyId])
  @@index([managerId])
}

// ────────────────────────────── RBAC ──────────────────────────────

model Role {
  id          String  @id
  companyId   String?
  key         RoleKey
  name        String
  description String?

  members     CompanyMember[]
  permissions RolePermission[]

  @@unique([companyId, key])
}

model Permission {
  id          String  @id
  key         String  @unique
  description String?

  roles RolePermission[]
}

model RolePermission {
  roleId       String
  permissionId String

  role       Role       @relation(fields: [roleId], references: [id], onDelete: Cascade)
  permission Permission @relation(fields: [permissionId], references: [id], onDelete: Cascade)

  @@id([roleId, permissionId])
  @@index([permissionId])
}

// ────────────────────────────── AUTH ──────────────────────────────

model Session {
  id               String    @id
  userId           String
  companyId        String
  deviceId         String?
  refreshTokenHash String
  expiresAt        DateTime  @db.Timestamptz(3)
  revokedAt        DateTime? @db.Timestamptz(3)
  createdAt        DateTime  @default(now()) @db.Timestamptz(3)

  user    User    @relation(fields: [userId], references: [id], onDelete: Cascade)
  company Company @relation(fields: [companyId], references: [id], onDelete: Cascade)

  @@index([userId, companyId])
  @@index([expiresAt])
}

// ────────────────────────────── ENUMS ──────────────────────────────

enum Plan          { FREE  PRO  ENTERPRISE }
enum CompanyStatus { ACTIVE  SUSPENDED  CANCELLED }
enum UserStatus    { ACTIVE  INVITED  SUSPENDED }
enum MemberStatus  { ACTIVE  INVITED  SUSPENDED }
enum RoleKey       { SUPER_ADMIN  COMPANY_ADMIN  MANAGER  EMPLOYEE  CLIENT_VIEWER }
```

- [ ] **Step 2: Validar el schema**

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm --filter api exec prisma validate
```

Expected: `The schema at apps\api\prisma\schema.prisma is valid 🚀`.

---

### Task 3: PrismaService + PrismaModule

**Files:**
- Create: `apps/api/src/database/prisma.service.ts`
- Create: `apps/api/src/database/prisma.module.ts`
- Modify: `apps/api/src/app.module.ts`

- [ ] **Step 1: Generar el cliente Prisma**

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm --filter api exec prisma generate
```

Expected: `Generated Prisma Client`.

- [ ] **Step 2: Crear `apps/api/src/database/prisma.service.ts`**

```ts
import { Injectable, OnModuleInit, OnModuleDestroy } from '@nestjs/common';
import { PrismaClient } from '@prisma/client';

@Injectable()
export class PrismaService
  extends PrismaClient
  implements OnModuleInit, OnModuleDestroy
{
  async onModuleInit(): Promise<void> {
    await this.$connect();
  }

  async onModuleDestroy(): Promise<void> {
    await this.$disconnect();
  }
}
```

- [ ] **Step 3: Crear `apps/api/src/database/prisma.module.ts`**

```ts
import { Global, Module } from '@nestjs/common';
import { PrismaService } from './prisma.service';

@Global()
@Module({
  providers: [PrismaService],
  exports: [PrismaService],
})
export class PrismaModule {}
```

- [ ] **Step 4: Importar `PrismaModule` en `apps/api/src/app.module.ts`**

```ts
import { Module } from '@nestjs/common';
import { AppController } from './app.controller';
import { PrismaModule } from './database/prisma.module';

@Module({
  imports: [PrismaModule],
  controllers: [AppController],
  providers: [],
})
export class AppModule {}
```

- [ ] **Step 5: Verificar que la api sigue compilando y el test pasa**

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm --filter api build
pnpm --filter api test
```

Expected: build ok; `GET /health returns ok` sigue en verde.

---

### Task 4: Primera migración

**Files:**
- Create: `apps/api/prisma/migrations/**` (generado por Prisma)

- [ ] **Step 1: Crear y aplicar la migración inicial**

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm --filter api exec prisma migrate dev --name init_auth_tenancy
```

Expected: crea `apps/api/prisma/migrations/<timestamp>_init_auth_tenancy/migration.sql`
y la aplica a la base `laboraltracker`. Mensaje `Your database is now in sync`.

- [ ] **Step 2: Confirmar las tablas creadas**

```powershell
$env:Path = "C:\Program Files\Docker\Docker\resources\bin;$env:Path"
docker exec laboraltracker-db psql -U laboraltracker -d laboraltracker -c "\dt"
```

Expected: aparecen `Company`, `User`, `CompanyMember`, `Team`, `Role`,
`Permission`, `RolePermission`, `Session`, `_prisma_migrations`.

---

### Task 5: Seed (roles, permisos, 2 empresas demo)

**Files:**
- Create: `apps/api/prisma/seed.ts`
- Modify: `apps/api/package.json` (bloque `prisma.seed` + dep `ts-node` ya presente)

- [ ] **Step 1: Crear `apps/api/prisma/seed.ts`**

```ts
import { PrismaClient, RoleKey } from '@prisma/client';
import { hash } from '@node-rs/argon2';
import { ulid } from 'ulid';

const prisma = new PrismaClient();

// seed login (DEV/TEST ONLY):
//   admin@a.demo / employee@a.demo  (Empresa A)
//   admin@b.demo / employee@b.demo  (Empresa B)
//   password = DEV_SEED_PASSWORD (override con env SEED_PASSWORD)
const DEV_SEED_PASSWORD = process.env.SEED_PASSWORD ?? 'Password123!';

// Catálogo mínimo de permisos para Fase 1.
const PERMISSIONS = ['member.read', 'member.manage', 'report.view'];

// Qué permisos tiene cada rol (Fase 1; matriz doc 00 §2, recortada).
const ROLE_PERMISSIONS: Record<RoleKey, string[]> = {
  SUPER_ADMIN: [],
  COMPANY_ADMIN: ['member.read', 'member.manage', 'report.view'],
  MANAGER: ['member.read', 'report.view'],
  EMPLOYEE: [],
  CLIENT_VIEWER: ['report.view'],
};

async function main(): Promise<void> {
  const passwordHash = await hash(DEV_SEED_PASSWORD);

  // Permisos
  const permIds = new Map<string, string>();
  for (const key of PERMISSIONS) {
    const id = ulid();
    permIds.set(key, id);
    await prisma.permission.upsert({
      where: { key },
      update: {},
      create: { id, key },
    });
  }

  // Roles globales (companyId = null) + RolePermission
  const roleIds = new Map<RoleKey, string>();
  for (const key of Object.keys(ROLE_PERMISSIONS) as RoleKey[]) {
    const existing = await prisma.role.findFirst({
      where: { companyId: null, key },
    });
    const id = existing?.id ?? ulid();
    roleIds.set(key, id);
    if (!existing) {
      await prisma.role.create({ data: { id, key, name: key } });
    }
    for (const permKey of ROLE_PERMISSIONS[key]) {
      const permId = (await prisma.permission.findUnique({
        where: { key: permKey },
      }))!.id;
      await prisma.rolePermission.upsert({
        where: { roleId_permissionId: { roleId: id, permissionId: permId } },
        update: {},
        create: { roleId: id, permissionId: permId },
      });
    }
  }

  // Empresas demo + usuarios + membresías
  const companies = [
    { slug: 'empresa-a', name: 'Empresa A', adminEmail: 'admin@a.demo', empEmail: 'employee@a.demo' },
    { slug: 'empresa-b', name: 'Empresa B', adminEmail: 'admin@b.demo', empEmail: 'employee@b.demo' },
  ];

  for (const c of companies) {
    const company = await prisma.company.upsert({
      where: { slug: c.slug },
      update: {},
      create: { id: ulid(), slug: c.slug, name: c.name },
    });

    for (const [email, name, roleKey] of [
      [c.adminEmail, `Admin ${c.name}`, RoleKey.COMPANY_ADMIN],
      [c.empEmail, `Employee ${c.name}`, RoleKey.EMPLOYEE],
    ] as const) {
      const user = await prisma.user.upsert({
        where: { email },
        update: {},
        create: { id: ulid(), email, name, passwordHash },
      });
      await prisma.companyMember.upsert({
        where: { userId_companyId: { userId: user.id, companyId: company.id } },
        update: {},
        create: {
          id: ulid(),
          userId: user.id,
          companyId: company.id,
          roleId: roleIds.get(roleKey)!,
        },
      });
    }
  }

  console.log('Seed done.');
}

main()
  .catch((e) => {
    console.error(e);
    process.exit(1);
  })
  .finally(() => prisma.$disconnect());
```

- [ ] **Step 2: Registrar el seed en `apps/api/package.json`**

Agregar al final del objeto raíz (después de `"jest": { ... }`):

```json
,
  "prisma": {
    "seed": "ts-node prisma/seed.ts"
  }
```

(NestJS ya trae `ts-node` en devDependencies; no hace falta agregarlo.)

---

### Task 6: Correr el seed y verificar

**Files:** ninguno (verificación)

- [ ] **Step 1: Ejecutar el seed**

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm --filter api exec prisma db seed
```

Expected: imprime `Seed done.` sin errores.

- [ ] **Step 2: Verificar los datos sembrados**

```powershell
$env:Path = "C:\Program Files\Docker\Docker\resources\bin;$env:Path"
docker exec laboraltracker-db psql -U laboraltracker -d laboraltracker -c "SELECT (SELECT count(*) FROM \"Company\") AS companies, (SELECT count(*) FROM \"User\") AS users, (SELECT count(*) FROM \"Role\") AS roles, (SELECT count(*) FROM \"Permission\") AS perms, (SELECT count(*) FROM \"CompanyMember\") AS members;"
```

Expected: `companies=2, users=4, roles=5, perms=3, members=4`.

- [ ] **Step 3: Verificar idempotencia (correr el seed otra vez no duplica)**

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm --filter api exec prisma db seed
```

Luego repetir la query del Step 2. Expected: los mismos counts (2/4/5/3/4) — el
seed usa `upsert`, no duplica.

---

### Task 7: CHANGELOG + commit + PR

- [ ] **Step 1: Actualizar CHANGELOG**

Agregar bajo `## [Unreleased] → Added` en [CHANGELOG.md](../../../CHANGELOG.md):

```markdown
- **Phase 1a — data layer**: Prisma + the Phase 1 model subset
  (Company/User/CompanyMember/Team/Role/Permission/RolePermission/Session with
  `companyId`), initial migration `init_auth_tenancy`, `PrismaService`/`PrismaModule`,
  and an idempotent seed (5 roles + permissions + 2 demo companies with users and
  memberships; `@node-rs/argon2` hashing, documented dev password). /
  **Fase 1a — capa de datos**: Prisma + el subconjunto Fase 1 del modelo, migración
  inicial, `PrismaService`/`PrismaModule`, y un seed idempotente (5 roles +
  permisos + 2 empresas demo con usuarios y membresías; hashing `@node-rs/argon2`,
  password de dev documentado).
```

- [ ] **Step 2: Commit**

```bash
git add apps/api package.json pnpm-workspace.yaml pnpm-lock.yaml CHANGELOG.md
git commit -m "feat(api): Prisma data layer + Phase 1 model + migration + seed (Phase 1a)"
```

- [ ] **Step 3: Push + PR**

```bash
git push -u origin feature/fase1a-data
gh pr create --title "feat(api): data layer — Prisma model + migration + seed (Phase 1a)" --body "Phase 1a: Prisma + Phase-1 model subset, init migration, PrismaService/Module, idempotent seed (5 roles + permissions + 2 demo companies). Verified: migrate applies, seed runs idempotently (2/4/5/3/4 counts). Spec: docs/superpowers/specs/2026-06-20-fase1-auth-tenancy-rbac-design.md"
```

> Nota CI: el job `platform` aún NO migra/siembra (eso llega en 1c con el service
> Postgres). En 1a, CI solo corre el unit test existente; la migración/seed se
> verifican localmente contra el Postgres dockerizado.

> Al terminar: superpowers:finishing-a-development-branch → mergear antes de 1b.
