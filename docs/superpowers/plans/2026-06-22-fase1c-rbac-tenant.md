# Fase 1c — RBAC, Tenant Guard & cross-tenant isolation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close Phase 1 by adding authorization on top of 1b's authentication: a `PermissionGuard` (RBAC, permissions-as-data), a `TenantGuard` (active-membership check), the first tenant-scoped endpoint `GET /members`, the **cross-tenant isolation e2e** (the spec's non-negotiable success criterion), and a Postgres service in CI so the wired system is finally exercised.

**Architecture:** `AuthGuard` is already global (`APP_GUARD`, runs first, injects `AuthCtx`). `PermissionGuard` (reads `@RequirePermission`, checks the token's `permissions[]`) and `TenantGuard` (re-validates the user's active `CompanyMember` in the token's `companyId`) are applied per-controller via `@UseGuards`, so they run after the global AuthGuard in the order **Auth → Permission → Tenant**. A new `members` feature module exposes `GET /members`, scoped to the active company. CI gains a `postgres:16` service to run migrate/seed/integration/e2e.

**Tech Stack:** NestJS 11 guards + `Reflector` · Prisma 6 · the 1b auth stack (`AuthRepository`, `AuthCtx`) · GitHub Actions service containers.

**Spec:** [docs/superpowers/specs/2026-06-20-fase1-auth-tenancy-rbac-design.md](../specs/2026-06-20-fase1-auth-tenancy-rbac-design.md) (Slice 1c) · guards/order: [docs/saas/02-api-and-security.md](../../saas/02-api-and-security.md) §5 · structure: [docs/saas/04-backend-nestjs-structure.md](../../saas/04-backend-nestjs-structure.md) §3-4

## Global Constraints

- **Prisma pinned to `^6`** (ADR 0010). `prisma generate` runs via `postinstall` + an explicit CI step.
- **PowerShell on Windows.** Prepend `$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"` before every `pnpm`. Postgres dev: `$env:Path = "C:\Program Files\Docker\Docker\resources\bin;$env:Path"` before `docker`.
- **`.env*` is BLOCKED for the assistant.** DB/JWT vars are injected INLINE in PowerShell for local integration/e2e runs (same pattern as 1b): `$env:DATABASE_URL = "postgresql://laboraltracker:laboraltracker@localhost:5432/laboraltracker?schema=public"; $env:JWT_SECRET = "test-only-secret-at-least-32-characters-long"; $env:ACCESS_TTL = "15m"; $env:REFRESH_TTL_DAYS = "30"`. Postgres must be up + seeded (`docker compose up -d postgres`; `pnpm --filter api exec prisma db seed`).
- **The global `AuthGuard` (`APP_GUARD`) already runs first** and injects `request.authCtx: AuthCtx { userId, email, companyId, roleKey, permissions, sessionId }`. The new guards READ `request.authCtx`; they never re-parse the token.
- **Permissions are data** (ADR 0006): `PermissionGuard` checks membership in `authCtx.permissions` (an in-memory array from the JWT) — no DB query on the permission check.
- **Seed RBAC matrix** (from 1a seed): `COMPANY_ADMIN` has `member.read`, `member.manage`, `report.view`; `EMPLOYEE` has none. Demo users: `admin@a.demo`/`employee@a.demo` (empresa-a), `admin@b.demo`/`employee@b.demo` (empresa-b), password `Password123!`.
- **Integration spec exclusion:** `*.repository.spec.ts` is excluded from the default `jest` run via `testPathIgnorePatterns` (package.json). Run it with `pnpm --filter api test:integration`.
- Conventional commits, NO AI-attribution trailer. Own branch `feature/fase1c-rbac-tenant`.

---

## File Structure

```txt
apps/api/src/
├── common/
│   ├── decorators/
│   │   └── require-permission.decorator.ts   # CREATE: @RequirePermission('member.read')
│   └── guards/
│       ├── permission.guard.ts               # CREATE: RBAC — checks authCtx.permissions vs @RequirePermission
│       └── tenant.guard.ts                   # CREATE: re-validates active CompanyMember in authCtx.companyId
├── modules/
│   ├── auth/
│   │   └── auth.module.ts                     # MODIFY: provide + export PermissionGuard, TenantGuard
│   └── members/
│       ├── members.module.ts                 # CREATE (imports AuthModule for the guards)
│       ├── members.controller.ts             # CREATE: GET /members, @RequirePermission + @UseGuards(PermissionGuard, TenantGuard)
│       ├── members.service.ts                # CREATE
│       ├── members.repository.ts             # CREATE: list CompanyMember by companyId
│       └── members.types.ts                  # CREATE: MemberDto
└── app.module.ts                             # MODIFY: import MembersModule

apps/api/src/common/guards/permission.guard.spec.ts   # CREATE (unit)
apps/api/src/common/guards/tenant.guard.spec.ts       # CREATE (unit, mocks AuthRepository)
apps/api/src/modules/members/members.service.spec.ts  # CREATE (unit, mocks repo)
apps/api/test/tenant-isolation.e2e-spec.ts            # CREATE (the isolation gate)
.github/workflows/ci.yml                              # MODIFY: postgres:16 service + migrate/seed + integration + e2e
CHANGELOG.md                                          # MODIFY
```

**Why per-controller guards:** `AuthGuard` is global so EVERY route is authenticated-by-default (secure-by-default, established in 1b). Authorization is route-specific — `PermissionGuard`/`TenantGuard` only belong on tenant-scoped resource routes, so they are declared explicitly with `@UseGuards`. NestJS runs global guards before controller guards, giving the required Auth → Permission → Tenant order without extra wiring.

---

## Key design decisions (read before coding)

1. **Guard order Auth → Permission → Tenant.** Global `AuthGuard` (APP_GUARD) runs first and injects `authCtx`. `@UseGuards(PermissionGuard, TenantGuard)` on the controller runs after, in array order. `PermissionGuard` reads `authCtx.permissions` (no DB); `TenantGuard` re-checks the DB for an active membership (defense against a membership revoked after the token was minted).
2. **`PermissionGuard` is permission-as-data + OCP:** it reads the required permission from `@RequirePermission` metadata and checks array membership. Adding a permission/role never touches the guard. If a route has no `@RequirePermission`, the guard is a no-op (returns true) — it only enforces when a permission is declared.
3. **`TenantGuard` re-validates membership, does not just trust the token.** The token's `companyId` is signed, but a membership could have been revoked within the access-token TTL. `TenantGuard` calls `AuthRepository.findMembership(userId, companyId)` (already filters ACTIVE + deletedAt); null → 403.
4. **`GET /members` is scoped by the token's `companyId`**, never a client-supplied value. The repository filters `where: { companyId, deletedAt: null }`. This is the structural guarantee that company A cannot read company B.
5. **The isolation e2e is the gate:** A-admin sees only A's members; B-admin sees only B's; an `EMPLOYEE` (no `member.read`) gets 403; no token → 401. If this suite is not green, Phase 1 does not close.
6. **CI finally runs the wired system:** a `postgres:16` service + `prisma migrate deploy` + `prisma db seed` + the integration spec + the e2e. This pays down the 1b debt; "all green" now means the assembled app works.

---

### Task 1: `@RequirePermission` decorator + `PermissionGuard`

**Files:**
- Create: `apps/api/src/common/decorators/require-permission.decorator.ts`
- Create: `apps/api/src/common/guards/permission.guard.ts`
- Test: `apps/api/src/common/guards/permission.guard.spec.ts`
- Modify: `apps/api/src/modules/auth/auth.module.ts` (provide + export `PermissionGuard`)

**Interfaces:**
- Consumes: `Reflector` (NestJS), `AuthCtx` (from `common/auth/auth-context.ts`).
- Produces:
  - `REQUIRE_PERMISSION_KEY` (string) and `RequirePermission(permission: string)` decorator.
  - `PermissionGuard` (implements `CanActivate`, `canActivate(ctx): boolean`).

- [ ] **Step 1: Write `apps/api/src/common/decorators/require-permission.decorator.ts`**

```ts
import { SetMetadata } from '@nestjs/common';

export const REQUIRE_PERMISSION_KEY = 'requirePermission';

/** Declares the Permission a route requires; read by PermissionGuard. */
export const RequirePermission = (permission: string) =>
  SetMetadata(REQUIRE_PERMISSION_KEY, permission);
```

- [ ] **Step 2: Write the failing test `apps/api/src/common/guards/permission.guard.spec.ts`**

```ts
import { ExecutionContext, ForbiddenException } from '@nestjs/common';
import { Reflector } from '@nestjs/core';
import { PermissionGuard } from './permission.guard';
import type { AuthCtx } from '../auth/auth-context';

function makeContext(
  permissions: string[] | undefined,
): ExecutionContext {
  const authCtx: Partial<AuthCtx> | undefined =
    permissions === undefined ? undefined : { permissions };
  return {
    switchToHttp: () => ({ getRequest: () => ({ authCtx }) }),
    getHandler: () => 'handler',
    getClass: () => 'class',
  } as unknown as ExecutionContext;
}

function makeReflector(required: string | undefined): Reflector {
  return { getAllAndOverride: () => required } as unknown as Reflector;
}

describe('PermissionGuard', () => {
  it('allows when the route declares no required permission', () => {
    const guard = new PermissionGuard(makeReflector(undefined));
    expect(guard.canActivate(makeContext(['member.read']))).toBe(true);
  });

  it('allows when the token has the required permission', () => {
    const guard = new PermissionGuard(makeReflector('member.read'));
    expect(guard.canActivate(makeContext(['member.read', 'report.view']))).toBe(true);
  });

  it('denies (403) when the token lacks the required permission', () => {
    const guard = new PermissionGuard(makeReflector('member.read'));
    expect(() => guard.canActivate(makeContext([]))).toThrow(ForbiddenException);
  });

  it('denies (403) when there is no auth context', () => {
    const guard = new PermissionGuard(makeReflector('member.read'));
    expect(() => guard.canActivate(makeContext(undefined))).toThrow(ForbiddenException);
  });
});
```

- [ ] **Step 3: Run the test to verify it fails**

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm --filter api test -- permission.guard
```

Expected: FAIL — `Cannot find module './permission.guard'`.

- [ ] **Step 4: Write `apps/api/src/common/guards/permission.guard.ts`**

```ts
import {
  CanActivate,
  ExecutionContext,
  ForbiddenException,
  Injectable,
} from '@nestjs/common';
import { Reflector } from '@nestjs/core';
import { REQUIRE_PERMISSION_KEY } from '../decorators/require-permission.decorator';
import type { AuthCtx } from '../auth/auth-context';

@Injectable()
export class PermissionGuard implements CanActivate {
  constructor(private readonly reflector: Reflector) {}

  canActivate(context: ExecutionContext): boolean {
    const required = this.reflector.getAllAndOverride<string | undefined>(
      REQUIRE_PERMISSION_KEY,
      [context.getHandler(), context.getClass()],
    );
    // No permission declared on this route → nothing to enforce.
    if (!required) return true;

    const request = context
      .switchToHttp()
      .getRequest<{ authCtx?: AuthCtx }>();
    const permissions = request.authCtx?.permissions;
    if (!permissions || !permissions.includes(required)) {
      throw new ForbiddenException('Missing required permission');
    }
    return true;
  }
}
```

- [ ] **Step 5: Run the test to verify it passes**

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm --filter api test -- permission.guard
```

Expected: PASS (4 tests).

- [ ] **Step 6: Provide + export `PermissionGuard` in `apps/api/src/modules/auth/auth.module.ts`**

Update the module so the guard is injectable in feature modules that import `AuthModule`:

```ts
import { Module } from '@nestjs/common';
import { APP_GUARD } from '@nestjs/core';
import { JwtModule } from '@nestjs/jwt';
import { AuthController } from './auth.controller';
import { AuthService } from './auth.service';
import { AuthRepository } from './auth.repository';
import { TokenService } from './token.service';
import { AuthGuard } from '../../common/guards/auth.guard';
import { PermissionGuard } from '../../common/guards/permission.guard';

@Module({
  imports: [JwtModule.register({})],
  controllers: [AuthController],
  providers: [
    AuthService,
    AuthRepository,
    TokenService,
    AuthGuard,
    PermissionGuard,
    { provide: APP_GUARD, useClass: AuthGuard },
  ],
  exports: [TokenService, AuthGuard, PermissionGuard],
})
export class AuthModule {}
```

- [ ] **Step 7: Verify build + unit suites still green**

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm --filter api build
pnpm --filter api test
```

Expected: build OK; all unit suites green (incl. the 4 new permission.guard tests).

- [ ] **Step 8: Commit**

```bash
git add apps/api/src/common/decorators/require-permission.decorator.ts apps/api/src/common/guards/permission.guard.ts apps/api/src/common/guards/permission.guard.spec.ts apps/api/src/modules/auth/auth.module.ts
git commit -m "feat(api): RBAC PermissionGuard + @RequirePermission (Phase 1c)"
```

---

### Task 2: `TenantGuard`

**Files:**
- Create: `apps/api/src/common/guards/tenant.guard.ts`
- Test: `apps/api/src/common/guards/tenant.guard.spec.ts`
- Modify: `apps/api/src/modules/auth/auth.module.ts` (provide + export `TenantGuard`)

**Interfaces:**
- Consumes: `AuthRepository.findMembership(userId, companyId) → Promise<ResolvedMembership | null>` (from 1b), `AuthCtx`.
- Produces: `TenantGuard` (implements `CanActivate`, `canActivate(ctx): Promise<boolean>`).

- [ ] **Step 1: Write the failing test `apps/api/src/common/guards/tenant.guard.spec.ts`**

```ts
import { ExecutionContext, ForbiddenException } from '@nestjs/common';
import { TenantGuard } from './tenant.guard';
import type { AuthRepository } from '../../modules/auth/auth.repository';
import type { AuthCtx } from '../auth/auth-context';

function makeContext(authCtx: Partial<AuthCtx> | undefined): ExecutionContext {
  return {
    switchToHttp: () => ({ getRequest: () => ({ authCtx }) }),
  } as unknown as ExecutionContext;
}

const membership = {
  companyId: 'c1',
  companyName: 'Empresa A',
  companySlug: 'empresa-a',
  roleKey: 'COMPANY_ADMIN',
  permissions: ['member.read'],
};

describe('TenantGuard', () => {
  it('allows when the user has an active membership in the token company', async () => {
    const repo = { findMembership: jest.fn().mockResolvedValue(membership) } as unknown as AuthRepository;
    const guard = new TenantGuard(repo);
    await expect(
      guard.canActivate(makeContext({ userId: 'u1', companyId: 'c1' })),
    ).resolves.toBe(true);
    expect((repo.findMembership as jest.Mock)).toHaveBeenCalledWith('u1', 'c1');
  });

  it('denies (403) when there is no active membership', async () => {
    const repo = { findMembership: jest.fn().mockResolvedValue(null) } as unknown as AuthRepository;
    const guard = new TenantGuard(repo);
    await expect(
      guard.canActivate(makeContext({ userId: 'u1', companyId: 'c-other' })),
    ).rejects.toBeInstanceOf(ForbiddenException);
  });

  it('denies (403) when there is no auth context', async () => {
    const repo = { findMembership: jest.fn() } as unknown as AuthRepository;
    const guard = new TenantGuard(repo);
    await expect(guard.canActivate(makeContext(undefined))).rejects.toBeInstanceOf(
      ForbiddenException,
    );
    expect((repo.findMembership as jest.Mock)).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm --filter api test -- tenant.guard
```

Expected: FAIL — `Cannot find module './tenant.guard'`.

- [ ] **Step 3: Write `apps/api/src/common/guards/tenant.guard.ts`**

```ts
import {
  CanActivate,
  ExecutionContext,
  ForbiddenException,
  Injectable,
} from '@nestjs/common';
import { AuthRepository } from '../../modules/auth/auth.repository';
import type { AuthCtx } from '../auth/auth-context';

@Injectable()
export class TenantGuard implements CanActivate {
  constructor(private readonly repo: AuthRepository) {}

  async canActivate(context: ExecutionContext): Promise<boolean> {
    const request = context
      .switchToHttp()
      .getRequest<{ authCtx?: AuthCtx }>();
    const ctx = request.authCtx;
    if (!ctx) throw new ForbiddenException('No auth context');

    // Re-validate against the DB: the membership may have been revoked within
    // the access-token TTL. The token's companyId is signed but not live.
    const membership = await this.repo.findMembership(ctx.userId, ctx.companyId);
    if (!membership) {
      throw new ForbiddenException('No active membership in company');
    }
    return true;
  }
}
```

- [ ] **Step 4: Run the test to verify it passes**

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm --filter api test -- tenant.guard
```

Expected: PASS (3 tests).

- [ ] **Step 5: Provide + export `TenantGuard` in `apps/api/src/modules/auth/auth.module.ts`**

Add `TenantGuard` to imports, providers, and exports:

```ts
import { Module } from '@nestjs/common';
import { APP_GUARD } from '@nestjs/core';
import { JwtModule } from '@nestjs/jwt';
import { AuthController } from './auth.controller';
import { AuthService } from './auth.service';
import { AuthRepository } from './auth.repository';
import { TokenService } from './token.service';
import { AuthGuard } from '../../common/guards/auth.guard';
import { PermissionGuard } from '../../common/guards/permission.guard';
import { TenantGuard } from '../../common/guards/tenant.guard';

@Module({
  imports: [JwtModule.register({})],
  controllers: [AuthController],
  providers: [
    AuthService,
    AuthRepository,
    TokenService,
    AuthGuard,
    PermissionGuard,
    TenantGuard,
    { provide: APP_GUARD, useClass: AuthGuard },
  ],
  exports: [TokenService, AuthGuard, PermissionGuard, TenantGuard],
})
export class AuthModule {}
```

- [ ] **Step 6: Verify build + unit suites green**

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm --filter api build
pnpm --filter api test
```

Expected: build OK; all unit suites green (incl. 3 new tenant.guard tests).

- [ ] **Step 7: Commit**

```bash
git add apps/api/src/common/guards/tenant.guard.ts apps/api/src/common/guards/tenant.guard.spec.ts apps/api/src/modules/auth/auth.module.ts
git commit -m "feat(api): TenantGuard — active-membership re-validation (Phase 1c)"
```

---

### Task 3: `members` module — `GET /members`

**Files:**
- Create: `apps/api/src/modules/members/members.types.ts`
- Create: `apps/api/src/modules/members/members.repository.ts`
- Create: `apps/api/src/modules/members/members.service.ts`
- Create: `apps/api/src/modules/members/members.controller.ts`
- Create: `apps/api/src/modules/members/members.module.ts`
- Test: `apps/api/src/modules/members/members.service.spec.ts`
- Modify: `apps/api/src/app.module.ts` (import `MembersModule`)

**Interfaces:**
- Consumes: `PrismaService` (global), `PermissionGuard` + `TenantGuard` (from `AuthModule`), `@RequirePermission`, `@CurrentUser`, `AuthCtx`.
- Produces:
  - `MemberDto { id, userId, email, name, roleKey, status }`
  - `MembersRepository.listByCompany(companyId: string): Promise<MemberDto[]>`
  - `MembersService.list(companyId: string): Promise<MemberDto[]>`
  - Route `GET /api/v1/members`.

- [ ] **Step 1: Write `apps/api/src/modules/members/members.types.ts`**

```ts
export interface MemberDto {
  id: string;
  userId: string;
  email: string;
  name: string;
  roleKey: string;
  status: string;
}
```

- [ ] **Step 2: Write `apps/api/src/modules/members/members.repository.ts`**

```ts
import { Injectable } from '@nestjs/common';
import { PrismaService } from '../../database/prisma.service';
import type { MemberDto } from './members.types';

@Injectable()
export class MembersRepository {
  constructor(private readonly prisma: PrismaService) {}

  /** Lists the active (non-deleted) members of a single company. */
  async listByCompany(companyId: string): Promise<MemberDto[]> {
    const members = await this.prisma.companyMember.findMany({
      where: { companyId, deletedAt: null },
      include: {
        user: { select: { id: true, email: true, name: true } },
        role: { select: { key: true } },
      },
      orderBy: { createdAt: 'asc' },
    });
    return members.map((m) => ({
      id: m.id,
      userId: m.user.id,
      email: m.user.email,
      name: m.user.name,
      roleKey: m.role.key,
      status: m.status,
    }));
  }
}
```

- [ ] **Step 3: Write the failing test `apps/api/src/modules/members/members.service.spec.ts`**

```ts
import { MembersService } from './members.service';
import type { MembersRepository } from './members.repository';
import type { MemberDto } from './members.types';

const rows: MemberDto[] = [
  { id: 'm1', userId: 'u1', email: 'admin@a.demo', name: 'Admin A', roleKey: 'COMPANY_ADMIN', status: 'ACTIVE' },
];

describe('MembersService', () => {
  it('lists members for the given company id', async () => {
    const repo = { listByCompany: jest.fn().mockResolvedValue(rows) } as unknown as MembersRepository;
    const service = new MembersService(repo);
    const result = await service.list('c1');
    expect(repo.listByCompany).toHaveBeenCalledWith('c1');
    expect(result).toEqual(rows);
  });
});
```

- [ ] **Step 4: Run the test to verify it fails**

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm --filter api test -- members.service
```

Expected: FAIL — `Cannot find module './members.service'`.

- [ ] **Step 5: Write `apps/api/src/modules/members/members.service.ts`**

```ts
import { Injectable } from '@nestjs/common';
import { MembersRepository } from './members.repository';
import type { MemberDto } from './members.types';

@Injectable()
export class MembersService {
  constructor(private readonly repo: MembersRepository) {}

  list(companyId: string): Promise<MemberDto[]> {
    return this.repo.listByCompany(companyId);
  }
}
```

- [ ] **Step 6: Run the test to verify it passes**

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm --filter api test -- members.service
```

Expected: PASS (1 test).

- [ ] **Step 7: Write `apps/api/src/modules/members/members.controller.ts`**

The global `AuthGuard` authenticates; `@UseGuards(PermissionGuard, TenantGuard)` adds RBAC + tenant checks (in that order). `companyId` comes from the token, never the client.

```ts
import { Controller, Get, UseGuards } from '@nestjs/common';
import { MembersService } from './members.service';
import { PermissionGuard } from '../../common/guards/permission.guard';
import { TenantGuard } from '../../common/guards/tenant.guard';
import { RequirePermission } from '../../common/decorators/require-permission.decorator';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import type { AuthCtx } from '../../common/auth/auth-context';
import type { MemberDto } from './members.types';

@Controller('members')
@UseGuards(PermissionGuard, TenantGuard)
export class MembersController {
  constructor(private readonly members: MembersService) {}

  @Get()
  @RequirePermission('member.read')
  list(@CurrentUser() user: AuthCtx): Promise<MemberDto[]> {
    return this.members.list(user.companyId);
  }
}
```

- [ ] **Step 8: Write `apps/api/src/modules/members/members.module.ts`**

```ts
import { Module } from '@nestjs/common';
import { AuthModule } from '../auth/auth.module';
import { MembersController } from './members.controller';
import { MembersService } from './members.service';
import { MembersRepository } from './members.repository';

@Module({
  imports: [AuthModule], // PermissionGuard + TenantGuard (+ AuthRepository the TenantGuard uses)
  controllers: [MembersController],
  providers: [MembersService, MembersRepository],
})
export class MembersModule {}
```

- [ ] **Step 9: Import `MembersModule` in `apps/api/src/app.module.ts`**

```ts
import { Module } from '@nestjs/common';
import { ConfigModule } from '@nestjs/config';
import { AppController } from './app.controller';
import { PrismaModule } from './database/prisma.module';
import { AuthModule } from './modules/auth/auth.module';
import { MembersModule } from './modules/members/members.module';
import { validateEnv } from './config/env.validation';

@Module({
  imports: [
    ConfigModule.forRoot({ isGlobal: true, validate: validateEnv }),
    PrismaModule,
    AuthModule,
    MembersModule,
  ],
  controllers: [AppController],
  providers: [],
})
export class AppModule {}
```

- [ ] **Step 10: Verify build + unit suites green**

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm --filter api build
pnpm --filter api test
```

Expected: build OK; all unit suites green (incl. the new members.service test).

- [ ] **Step 11: Commit**

```bash
git add apps/api/src/modules/members apps/api/src/app.module.ts
git commit -m "feat(api): members module + GET /members (tenant-scoped, RBAC) (Phase 1c)"
```

---

### Task 4: Cross-tenant isolation e2e (the gate)

**Files:**
- Create: `apps/api/test/tenant-isolation.e2e-spec.ts`

**Interfaces:**
- Consumes: the full `AppModule` over HTTP; seeded demo data (`admin@a.demo`/`employee@a.demo` in empresa-a, `admin@b.demo`/`employee@b.demo` in empresa-b, password `Password123!`).

- [ ] **Step 1: Write `apps/api/test/tenant-isolation.e2e-spec.ts`**

```ts
import { Test } from '@nestjs/testing';
import { INestApplication, ValidationPipe } from '@nestjs/common';
import request from 'supertest';
import { AppModule } from '../src/app.module';

describe('Cross-tenant isolation (e2e)', () => {
  let app: INestApplication;
  const base = '/api/v1';

  beforeAll(async () => {
    const moduleRef = await Test.createTestingModule({ imports: [AppModule] }).compile();
    app = moduleRef.createNestApplication();
    app.setGlobalPrefix('api/v1');
    app.useGlobalPipes(
      new ValidationPipe({ whitelist: true, forbidNonWhitelisted: true, transform: true }),
    );
    await app.init();
  });

  afterAll(async () => {
    await app.close();
  });

  async function login(email: string, password = 'Password123!'): Promise<string> {
    const res = await request(app.getHttpServer())
      .post(`${base}/auth/login`)
      .send({ email, password })
      .expect(200);
    return res.body.accessToken as string;
  }

  it('rejects GET /members without a token (401)', async () => {
    await request(app.getHttpServer()).get(`${base}/members`).expect(401);
  });

  it('lets Empresa A admin see ONLY Empresa A members', async () => {
    const token = await login('admin@a.demo');
    const res = await request(app.getHttpServer())
      .get(`${base}/members`)
      .set('Authorization', `Bearer ${token}`)
      .expect(200);
    const emails = (res.body as Array<{ email: string }>).map((m) => m.email).sort();
    expect(emails).toEqual(['admin@a.demo', 'employee@a.demo']);
  });

  it('lets Empresa B admin see ONLY Empresa B members', async () => {
    const token = await login('admin@b.demo');
    const res = await request(app.getHttpServer())
      .get(`${base}/members`)
      .set('Authorization', `Bearer ${token}`)
      .expect(200);
    const emails = (res.body as Array<{ email: string }>).map((m) => m.email).sort();
    expect(emails).toEqual(['admin@b.demo', 'employee@b.demo']);
    // The decisive isolation assertion: B's token never surfaces A's data.
    expect(emails).not.toContain('admin@a.demo');
    expect(emails).not.toContain('employee@a.demo');
  });

  it('denies an EMPLOYEE without member.read (403)', async () => {
    const token = await login('employee@a.demo');
    await request(app.getHttpServer())
      .get(`${base}/members`)
      .set('Authorization', `Bearer ${token}`)
      .expect(403);
  });
});
```

- [ ] **Step 2: Ensure Postgres is up and seeded, then run the e2e (env inline)**

```powershell
$env:Path = "C:\Program Files\Docker\Docker\resources\bin;$env:Path"
docker compose up -d postgres
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
$env:DATABASE_URL = "postgresql://laboraltracker:laboraltracker@localhost:5432/laboraltracker?schema=public"
pnpm --filter api exec prisma db seed
$env:JWT_SECRET = "test-only-secret-at-least-32-characters-long"; $env:ACCESS_TTL = "15m"; $env:REFRESH_TTL_DAYS = "30"
pnpm --filter api test:e2e
```

Expected: BOTH e2e suites green — the existing `auth.e2e` flow AND the new `tenant-isolation.e2e` (4 tests: no-token 401, A sees only A, B sees only B + isolation assertions, EMPLOYEE 403).

- [ ] **Step 3: Commit**

```bash
git add apps/api/test/tenant-isolation.e2e-spec.ts
git commit -m "test(api): cross-tenant isolation e2e — the Phase 1 gate (Phase 1c)"
```

---

### Task 5: CI — Postgres service + run the wired system

**Files:**
- Modify: `.github/workflows/ci.yml` (the `platform` job)

- [ ] **Step 1: Add a Postgres service + env + migrate/seed/integration/e2e to the `platform` job**

Replace the `platform` job's body with this (keeping `needs`, `if`, and `runs-on`):

```yaml
  platform:
    needs: changes
    if: ${{ needs.changes.outputs.platform == 'true' }}
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_USER: laboraltracker
          POSTGRES_PASSWORD: laboraltracker
          POSTGRES_DB: laboraltracker
        ports:
          - 5432:5432
        options: >-
          --health-cmd "pg_isready -U laboraltracker"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    env:
      DATABASE_URL: postgresql://laboraltracker:laboraltracker@localhost:5432/laboraltracker?schema=public
      JWT_SECRET: ci-only-secret-at-least-32-characters-long
      ACCESS_TTL: "15m"
      REFRESH_TTL_DAYS: "30"
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
      - run: pnpm install --frozen-lockfile
      # Prisma Client is a generated, non-versioned artifact: regenerate it from
      # the schema before anything compiles against the model types.
      - run: pnpm --filter api exec prisma generate
      - run: pnpm --filter api exec prisma migrate deploy
      - run: pnpm --filter api exec prisma db seed
      - run: pnpm --filter api test            # mocked unit suites
      - run: pnpm --filter api test:integration # repository spec vs Postgres
      - run: pnpm --filter api test:e2e         # auth flow + cross-tenant isolation
      - run: pnpm --filter api build
      - run: pnpm --filter web build
```

- [ ] **Step 2: Sanity-check the YAML locally**

```powershell
$env:Path = "C:\Users\elrug\AppData\Local\pnpm\bin;$env:Path"
pnpm --filter api build
```

(There is no local Actions runner; the YAML is validated by pushing. This step just confirms the app still builds. The real CI verification happens on the PR — watch the run and confirm the `platform` job goes green with migrate/seed/integration/e2e.)

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci(api): Postgres service + migrate/seed/integration/e2e (Phase 1c)"
```

---

### Task 6: CHANGELOG + PR

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Update `CHANGELOG.md`**

Add under `## [Unreleased]` → `### 🇬🇧 Added / 🇪🇸 Añadido`:

```markdown
- **Phase 1c — RBAC + tenant isolation**: `PermissionGuard` (permissions-as-data,
  reads `@RequirePermission`, checks the token's `permissions[]`, OCP — adding a
  permission touches no controller) and `TenantGuard` (re-validates the user's
  active `CompanyMember` in the token's `companyId`), applied per-controller after
  the global `AuthGuard` (order Auth→Permission→Tenant). First tenant-scoped
  endpoint `GET /members` (scoped by the token's company, never client input). The
  **cross-tenant isolation e2e** is green: company A sees only A's members, B only
  B's, an `EMPLOYEE` without `member.read` gets 403. CI now runs a `postgres:16`
  service + `migrate deploy`/seed + the integration spec + the e2e — the wired
  system is finally exercised. Closes Phase 1. /
  **Fase 1c — RBAC + aislamiento de tenant**: `PermissionGuard` (permisos como
  datos, lee `@RequirePermission`, verifica `permissions[]` del token, OCP) y
  `TenantGuard` (revalida la `CompanyMember` activa en el `companyId` del token),
  aplicados por controlador tras el `AuthGuard` global (orden Auth→Permission→
  Tenant). Primer endpoint tenant-scoped `GET /members` (acotado por la empresa del
  token, nunca por input del cliente). El **e2e de aislamiento entre empresas** está
  en verde: la empresa A ve solo miembros de A, B solo de B, un `EMPLOYEE` sin
  `member.read` recibe 403. La CI ahora levanta un servicio `postgres:16` +
  `migrate deploy`/seed + el spec de integración + el e2e. Cierra la Fase 1.
  → `apps/api/src/common/guards/`, `apps/api/src/modules/members/`, `apps/api/test/tenant-isolation.e2e-spec.ts`
```

- [ ] **Step 2: Commit, push, PR**

```bash
git add CHANGELOG.md
git commit -m "docs(api): changelog for Phase 1c"
git push -u origin feature/fase1c-rbac-tenant
gh pr create --base master --head feature/fase1c-rbac-tenant --title "feat(api): RBAC + tenant isolation — GET /members + isolation e2e (Phase 1c)" --body "Phase 1c closes Phase 1: PermissionGuard (permissions-as-data, @RequirePermission) + TenantGuard (active-membership re-validation), applied after the global AuthGuard (Auth→Permission→Tenant). First tenant-scoped endpoint GET /members, scoped by the token's company. The cross-tenant isolation e2e is the gate: A sees only A, B only B, EMPLOYEE without member.read → 403, no token → 401. CI gains a postgres:16 service + migrate deploy/seed + integration + e2e — the wired system is now exercised in CI (pays down the 1b debt). Spec: docs/superpowers/specs/2026-06-20-fase1-auth-tenancy-rbac-design.md"
```

> At the end: superpowers:finishing-a-development-branch → review + merge to close Phase 1.

---

## Plan self-review

**Spec coverage (Slice 1c of the Fase 1 spec):**
- Guards in order Auth → Permission → Tenant → (Step 1-2 + design decision 1). AuthGuard global from 1b; Permission/Tenant per-controller run after. ✅
- `PermissionGuard` reads `@RequirePermission`, compares to token permissions (permissions-as-data, OCP) → Task 1. ✅
- `TenantGuard` confirms active `CompanyMember` in `companyId`, leaves companyId for the repo filter → Task 2. ✅
- `OwnershipGuard` left as a seam (no owned resources yet) → not built, matches spec non-goal. ✅
- Tenant-scoped `GET /members` with `@RequirePermission('member.read')`, guards in order → Task 3. ✅
- The isolation e2e (A only A; B only B; A-token can't see B; EMPLOYEE without member.read → 403) → Task 4. ✅
- CI gains a Postgres service; steps migrate deploy → seed → unit → integration → e2e → Task 5. ✅

**Deviations / notes:**
- `TenantGuard` does a DB lookup per protected request (re-validates membership). The spec wants exactly this (active-membership check); ADR 0006 keeps *permissions* off the hot path, not the tenant check. Acceptable and intended.
- `GET /members` returns a bare `MemberDto[]` (not wrapped in `{success, data}`). The doc-02 response envelope is a cross-cutting concern (ResponseInterceptor) deferred beyond Phase 1; the isolation e2e asserts the array shape. Noted as future work, consistent with 1b returning bare token objects.

**Placeholder scan:** none — every code/test step is complete.

**Type consistency:** `AuthCtx` (userId, companyId, permissions) used consistently; `MemberDto` defined once (Task 3) and asserted by shape in Task 4; `findMembership` signature matches 1b; guard constructor deps (`Reflector`, `AuthRepository`) match their providers in `AuthModule`.
