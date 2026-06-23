# Fase 2a — Infraestructura de auditoría — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the audit substrate every later Phase-2 slice depends on: an immutable
`AuditLog`, an atomic `AuditService.record(entry, tx)`, and a global
`AllExceptionsFilter` that maps Prisma errors to clean HTTP responses.

**Architecture:** `AuditService` (builds the row: ULID + metadata) → `AuditRepository`
(persists via the caller's `Prisma.TransactionClient`, so audit and mutation are
atomic). No interceptor (cannot join Prisma's active tx). `AllExceptionsFilter`
(`APP_FILTER`) standardizes errors so a forgotten `catch` never leaks a raw 500.

**Tech Stack:** NestJS 11 · Prisma 6 (`@prisma/client`) · Postgres · `ulid` · Jest +
ts-jest · supertest (later slices).

## Global Constraints

- **Prisma 6** (ADR 0010); `PrismaService extends PrismaClient`, `url` in schema.
- **PKs `String @id` (ULID provisto por el servicio), sin `@default`** (ADR 0005).
- **`AuditLog` solo-append** desde código de app (sólo `INSERT`; sin update/delete).
- **Auditoría atómica:** la escritura usa el `Prisma.TransactionClient` del llamador
  (`tx`), nunca abre su propia conexión por fuera de la transacción de la mutación.
- **Tres niveles de test** (patrón Fase 1): unit (`*.spec.ts`, run default) e
  integración vs Postgres real (`*.repository.spec.ts`, excluida del run default por
  `testPathIgnorePatterns`, corre en `pnpm --filter api test:integration`).
- **Conventional commits, SIN** trailer `Co-Authored-By` ni atribución de IA.
- Error response shape = doc 02 §7: `{ success: false, error: { code, message }, timestamp }`.

**Branch:** `feature/fase2a-audit-infra` (no implementar en `master`).

---

### Task 1: `AuditLog` model + migration

**Files:**
- Modify: `apps/api/prisma/schema.prisma` (add `AuditLog` model + `Company.auditLogs`)
- Create: `apps/api/prisma/migrations/<timestamp>_add_audit_log/migration.sql` (generada)

**Interfaces:**
- Produces: el modelo Prisma `AuditLog` y el tipo generado `Prisma.AuditLogCreateInput`,
  consumidos por `AuditRepository` (Task 2).

- [ ] **Step 1: Add the `AuditLog` model + Company back-relation to the schema**

En `apps/api/prisma/schema.prisma`, agregar al final (antes de los enums) el modelo:

```prisma
/// Registro INMUTABLE de acciones sensibles (solo append: sin update/delete desde la app).
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

Y en el modelo `Company`, agregar la relación inversa junto a las demás
(`members`, `teams`, `sessions`):

```prisma
  auditLogs AuditLog[]
```

- [ ] **Step 2: Generate and apply the migration**

Run: `cd apps/api && pnpm exec prisma migrate dev --name add_audit_log`
Expected: nueva carpeta `prisma/migrations/<ts>_add_audit_log/` con `CREATE TABLE
"AuditLog"`, las dos `CREATE INDEX`, y la FK a `Company` con `ON DELETE SET NULL`.
El cliente Prisma se regenera (`AuditLog` queda disponible en el client).

- [ ] **Step 3: Verify the client type exists**

Run: `cd apps/api && pnpm exec tsc --noEmit`
Expected: compila sin errores (el client regenerado expone `prisma.auditLog`).

- [ ] **Step 4: Commit**

```bash
git add apps/api/prisma/schema.prisma apps/api/prisma/migrations
git commit -m "feat(api): add immutable AuditLog model + migration (Phase 2a)"
```

---

### Task 2: `AuditService` + `AuditRepository` + `AuditModule`

**Files:**
- Create: `apps/api/src/common/audit/audit.types.ts`
- Create: `apps/api/src/common/audit/audit-actions.ts`
- Create: `apps/api/src/common/audit/audit.repository.ts`
- Create: `apps/api/src/common/audit/audit.service.ts`
- Create: `apps/api/src/common/audit/audit.module.ts`
- Test: `apps/api/src/common/audit/audit.service.spec.ts` (unit)
- Test: `apps/api/src/common/audit/audit.repository.spec.ts` (integration, real DB)
- Modify: `apps/api/src/app.module.ts` (import `AuditModule`)

**Interfaces:**
- Consumes: `PrismaService` (from `database/prisma.service`), `Prisma.TransactionClient`.
- Produces:
  - `AuditEntry { action: string; companyId: string | null; actorUserId: string | null;
    entityType?: string; entityId?: string; before?: unknown; after?: unknown }`
  - `AuditService.record(entry: AuditEntry, tx?: Prisma.TransactionClient): Promise<void>`
  - `AuditActions` constant map (string labels) — later slices reference its keys.
  - `AuditModule` (`@Global`, exports `AuditService`) — domain modules inject
    `AuditService` without importing anything.

- [ ] **Step 1: Write the types and the action vocabulary**

`apps/api/src/common/audit/audit.types.ts`:

```ts
export interface AuditEntry {
  action: string;
  companyId: string | null;
  actorUserId: string | null;
  entityType?: string;
  entityId?: string;
  before?: unknown;
  after?: unknown;
}

export interface AuditLogRow {
  id: string;
  action: string;
  companyId: string | null;
  actorUserId: string | null;
  entityType: string | null;
  entityId: string | null;
  metadata: Record<string, unknown>;
}
```

`apps/api/src/common/audit/audit-actions.ts` (single source of truth for the Phase-2
audit vocabulary; later slices use these constants instead of magic strings):

```ts
export const AuditActions = {
  ProjectDeleted: 'project.deleted',
  TaskDeleted: 'task.deleted',
  MemberInvited: 'member.invited',
  MemberRoleChanged: 'member.role_changed',
  MemberStatusChanged: 'member.status_changed',
  TeamDeleted: 'team.deleted',
  TeamManagerChanged: 'team.manager_changed',
  CompanyUpdated: 'company.updated',
  CompanyCreated: 'company.created',
  CompanyPlanChanged: 'company.plan_changed',
  CompanyStatusChanged: 'company.status_changed',
} as const;

export type AuditAction = (typeof AuditActions)[keyof typeof AuditActions];
```

- [ ] **Step 2: Write the failing unit test for `AuditService`**

`apps/api/src/common/audit/audit.service.spec.ts`:

```ts
import { AuditService } from './audit.service';
import { AuditRepository } from './audit.repository';
import { AuditActions } from './audit-actions';

describe('AuditService', () => {
  it('builds a row with a ULID and serializes before/after into metadata', async () => {
    const insert = jest.fn().mockResolvedValue(undefined);
    const repo = { insert } as unknown as AuditRepository;
    const service = new AuditService(repo);

    await service.record({
      action: AuditActions.ProjectDeleted,
      companyId: 'c1',
      actorUserId: 'u1',
      entityType: 'Project',
      entityId: 'p1',
      before: { name: 'Old' },
      after: { name: 'Old', deletedAt: 'now' },
    });

    expect(insert).toHaveBeenCalledTimes(1);
    const [row, tx] = insert.mock.calls[0];
    expect(row.id).toMatch(/^[0-9A-HJKMNP-TV-Z]{26}$/); // ULID
    expect(row.action).toBe('project.deleted');
    expect(row.companyId).toBe('c1');
    expect(row.entityType).toBe('Project');
    expect(row.metadata).toEqual({ before: { name: 'Old' }, after: { name: 'Old', deletedAt: 'now' } });
    expect(tx).toBeUndefined();
  });

  it('omits before/after from metadata when not provided, and forwards tx', async () => {
    const insert = jest.fn().mockResolvedValue(undefined);
    const repo = { insert } as unknown as AuditRepository;
    const service = new AuditService(repo);
    const fakeTx = {} as never;

    await service.record(
      { action: AuditActions.MemberInvited, companyId: 'c1', actorUserId: 'u1' },
      fakeTx,
    );

    const [row, tx] = insert.mock.calls[0];
    expect(row.metadata).toEqual({});
    expect(row.entityType).toBeNull();
    expect(tx).toBe(fakeTx);
  });
});
```

- [ ] **Step 3: Run it to confirm it fails**

Run: `cd apps/api && pnpm test -- audit.service`
Expected: FAIL — `audit.service` / `audit.repository` modules don't exist yet.

- [ ] **Step 4: Implement the repository and service**

`apps/api/src/common/audit/audit.repository.ts`:

```ts
import { Injectable } from '@nestjs/common';
import { Prisma } from '@prisma/client';
import { PrismaService } from '../../database/prisma.service';
import { AuditLogRow } from './audit.types';

@Injectable()
export class AuditRepository {
  constructor(private readonly prisma: PrismaService) {}

  /** Append-only insert. Uses the caller's tx when given, so audit and the
   *  mutation commit/rollback together. */
  async insert(row: AuditLogRow, tx?: Prisma.TransactionClient): Promise<void> {
    const db = tx ?? this.prisma;
    await db.auditLog.create({
      data: {
        id: row.id,
        action: row.action,
        companyId: row.companyId,
        actorUserId: row.actorUserId,
        entityType: row.entityType,
        entityId: row.entityId,
        metadata: row.metadata as Prisma.InputJsonValue,
      },
    });
  }
}
```

`apps/api/src/common/audit/audit.service.ts`:

```ts
import { Injectable } from '@nestjs/common';
import { Prisma } from '@prisma/client';
import { ulid } from 'ulid';
import { AuditRepository } from './audit.repository';
import { AuditEntry } from './audit.types';

@Injectable()
export class AuditService {
  constructor(private readonly repo: AuditRepository) {}

  /** Record a sensitive mutation. Pass the mutation's `tx` for atomicity. */
  async record(entry: AuditEntry, tx?: Prisma.TransactionClient): Promise<void> {
    const metadata: Record<string, unknown> = {};
    if (entry.before !== undefined) metadata.before = entry.before;
    if (entry.after !== undefined) metadata.after = entry.after;

    await this.repo.insert(
      {
        id: ulid(),
        action: entry.action,
        companyId: entry.companyId,
        actorUserId: entry.actorUserId,
        entityType: entry.entityType ?? null,
        entityId: entry.entityId ?? null,
        metadata,
      },
      tx,
    );
  }
}
```

`apps/api/src/common/audit/audit.module.ts`:

```ts
import { Global, Module } from '@nestjs/common';
import { AuditService } from './audit.service';
import { AuditRepository } from './audit.repository';

@Global()
@Module({
  providers: [AuditService, AuditRepository],
  exports: [AuditService],
})
export class AuditModule {}
```

- [ ] **Step 5: Run the unit test to confirm it passes**

Run: `cd apps/api && pnpm test -- audit.service`
Expected: PASS (2/2).

- [ ] **Step 6: Write the integration test (real Postgres) for `AuditRepository`**

`apps/api/src/common/audit/audit.repository.spec.ts` (named `*.repository.spec.ts` →
excluded from the default run, picked up by `test:integration`):

```ts
import { ulid } from 'ulid';
import { PrismaService } from '../../database/prisma.service';
import { AuditRepository } from './audit.repository';

describe('AuditRepository (integration, real DB)', () => {
  const prisma = new PrismaService();
  const repo = new AuditRepository(prisma);

  beforeAll(async () => {
    await prisma.$connect();
  });
  afterAll(async () => {
    await prisma.$disconnect();
  });

  it('inserts an append-only audit row', async () => {
    const id = ulid();
    await repo.insert({
      id, action: 'test.insert', companyId: null, actorUserId: null,
      entityType: null, entityId: null, metadata: { before: { a: 1 } },
    });

    const found = await prisma.auditLog.findUnique({ where: { id } });
    expect(found?.action).toBe('test.insert');
    expect(found?.metadata).toEqual({ before: { a: 1 } });

    await prisma.auditLog.delete({ where: { id } }); // test cleanup only
  });

  it('is atomic: an insert inside a rolled-back tx leaves no row', async () => {
    const id = ulid();
    await expect(
      prisma.$transaction(async (tx) => {
        await repo.insert(
          { id, action: 'test.rollback', companyId: null, actorUserId: null,
            entityType: null, entityId: null, metadata: {} },
          tx,
        );
        throw new Error('force rollback');
      }),
    ).rejects.toThrow('force rollback');

    const found = await prisma.auditLog.findUnique({ where: { id } });
    expect(found).toBeNull();
  });
});
```

- [ ] **Step 7: Run the integration test against the seeded DB**

Run: `cd apps/api && pnpm test:integration -- audit.repository`
Expected: PASS (2/2). (Requires `DATABASE_URL` + applied migration — same as the
Phase 1 `auth.repository.spec.ts`.)

- [ ] **Step 8: Wire `AuditModule` into `AppModule`**

In `apps/api/src/app.module.ts`, add `AuditModule` to `imports` (after `PrismaModule`):

```ts
import { AuditModule } from './common/audit/audit.module';
// ...
imports: [
  ConfigModule.forRoot({ isGlobal: true, validate: validateEnv }),
  PrismaModule,
  AuditModule,
  AuthModule,
  MembersModule,
],
```

- [ ] **Step 9: Confirm the app still boots and unit tests are green**

Run: `cd apps/api && pnpm test`
Expected: PASS (existing suites + `audit.service` green; `audit.repository` excluded).

- [ ] **Step 10: Commit**

```bash
git add apps/api/src/common/audit apps/api/src/app.module.ts
git commit -m "feat(api): atomic AuditService + AuditRepository, global AuditModule (Phase 2a)"
```

---

### Task 3: `AllExceptionsFilter` (global Prisma→HTTP mapping)

**Files:**
- Create: `apps/api/src/common/filters/all-exceptions.filter.ts`
- Test: `apps/api/src/common/filters/all-exceptions.filter.spec.ts` (unit)
- Modify: `apps/api/src/app.module.ts` (register `APP_FILTER`)

**Interfaces:**
- Produces: a global `APP_FILTER` that maps `PrismaClientKnownRequestError`
  (`P2002→409`, `P2025→404`), respects `HttpException`, and maps everything else to a
  500 with no raw ORM/stack leak. Output shape = doc 02 §7.

- [ ] **Step 1: Write the failing unit test**

`apps/api/src/common/filters/all-exceptions.filter.spec.ts`:

```ts
import { ArgumentsHost, ForbiddenException, HttpStatus } from '@nestjs/common';
import { Prisma } from '@prisma/client';
import { AllExceptionsFilter } from './all-exceptions.filter';

function mockHost(): { host: ArgumentsHost; status: jest.Mock; json: jest.Mock } {
  const json = jest.fn();
  const status = jest.fn().mockReturnValue({ json });
  const host = {
    switchToHttp: () => ({ getResponse: () => ({ status }) }),
  } as unknown as ArgumentsHost;
  return { host, status, json };
}

describe('AllExceptionsFilter', () => {
  const filter = new AllExceptionsFilter();

  it('maps Prisma P2002 (unique) to 409 CONFLICT', () => {
    const { host, status, json } = mockHost();
    const err = new Prisma.PrismaClientKnownRequestError('unique', {
      code: 'P2002', clientVersion: '6.0.0',
    });
    filter.catch(err, host);
    expect(status).toHaveBeenCalledWith(HttpStatus.CONFLICT);
    expect(json.mock.calls[0][0]).toMatchObject({ success: false, error: { code: 'CONFLICT' } });
  });

  it('maps Prisma P2025 (not found) to 404', () => {
    const { host, status } = mockHost();
    const err = new Prisma.PrismaClientKnownRequestError('missing', {
      code: 'P2025', clientVersion: '6.0.0',
    });
    filter.catch(err, host);
    expect(status).toHaveBeenCalledWith(HttpStatus.NOT_FOUND);
  });

  it('respects an HttpException status (403)', () => {
    const { host, status, json } = mockHost();
    filter.catch(new ForbiddenException('nope'), host);
    expect(status).toHaveBeenCalledWith(HttpStatus.FORBIDDEN);
    expect(json.mock.calls[0][0].error.code).toBe('FORBIDDEN');
  });

  it('maps an unknown error to 500 without leaking the raw message', () => {
    const { host, status, json } = mockHost();
    filter.catch(new Error('SELECT * FROM secret leaked'), host);
    expect(status).toHaveBeenCalledWith(HttpStatus.INTERNAL_SERVER_ERROR);
    const body = json.mock.calls[0][0];
    expect(body.error.code).toBe('INTERNAL_ERROR');
    expect(JSON.stringify(body)).not.toContain('secret leaked');
  });
});
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `cd apps/api && pnpm test -- all-exceptions`
Expected: FAIL — filter module doesn't exist yet.

- [ ] **Step 3: Implement the filter**

`apps/api/src/common/filters/all-exceptions.filter.ts`:

```ts
import {
  ArgumentsHost,
  Catch,
  ExceptionFilter,
  HttpException,
  HttpStatus,
  Logger,
} from '@nestjs/common';
import { Prisma } from '@prisma/client';
import { Response } from 'express';

@Catch()
export class AllExceptionsFilter implements ExceptionFilter {
  private readonly logger = new Logger(AllExceptionsFilter.name);

  catch(exception: unknown, host: ArgumentsHost): void {
    const res = host.switchToHttp().getResponse<Response>();
    const { status, code, message } = this.map(exception);

    if (status >= HttpStatus.INTERNAL_SERVER_ERROR) {
      this.logger.error(exception); // full detail server-side only
    }

    res.status(status).json({
      success: false,
      error: { code, message },
      timestamp: new Date().toISOString(),
    });
  }

  private map(exception: unknown): { status: number; code: string; message: string } {
    if (exception instanceof HttpException) {
      const status = exception.getStatus();
      const resp = exception.getResponse();
      const raw =
        typeof resp === 'string'
          ? resp
          : ((resp as { message?: string | string[] }).message ?? exception.message);
      return {
        status,
        code: this.codeFor(status),
        message: Array.isArray(raw) ? raw.join(', ') : raw,
      };
    }

    if (exception instanceof Prisma.PrismaClientKnownRequestError) {
      if (exception.code === 'P2002') {
        return { status: HttpStatus.CONFLICT, code: 'CONFLICT', message: 'Resource already exists' };
      }
      if (exception.code === 'P2025') {
        return { status: HttpStatus.NOT_FOUND, code: 'NOT_FOUND', message: 'Resource not found' };
      }
    }

    return {
      status: HttpStatus.INTERNAL_SERVER_ERROR,
      code: 'INTERNAL_ERROR',
      message: 'Internal server error',
    };
  }

  private codeFor(status: number): string {
    switch (status) {
      case HttpStatus.BAD_REQUEST: return 'BAD_REQUEST';
      case HttpStatus.UNAUTHORIZED: return 'UNAUTHORIZED';
      case HttpStatus.FORBIDDEN: return 'FORBIDDEN';
      case HttpStatus.NOT_FOUND: return 'NOT_FOUND';
      case HttpStatus.CONFLICT: return 'CONFLICT';
      default: return status >= 500 ? 'INTERNAL_ERROR' : 'ERROR';
    }
  }
}
```

- [ ] **Step 4: Run the unit test to confirm it passes**

Run: `cd apps/api && pnpm test -- all-exceptions`
Expected: PASS (4/4).

- [ ] **Step 5: Register the filter globally in `AppModule`**

In `apps/api/src/app.module.ts`, add to `providers`:

```ts
import { APP_FILTER } from '@nestjs/core';
import { AllExceptionsFilter } from './common/filters/all-exceptions.filter';
// ...
providers: [{ provide: APP_FILTER, useClass: AllExceptionsFilter }],
```

- [ ] **Step 6: Confirm the whole unit suite is green**

Run: `cd apps/api && pnpm test`
Expected: PASS (all existing suites + `audit.service` + `all-exceptions`).

- [ ] **Step 7: Commit**

```bash
git add apps/api/src/common/filters apps/api/src/app.module.ts
git commit -m "feat(api): global AllExceptionsFilter mapping Prisma errors to HTTP (Phase 2a)"
```

---

### Task 4: CHANGELOG

**Files:**
- Modify: `CHANGELOG.md` (Unreleased → Added, bilingual EN/ES)

- [ ] **Step 1: Add a bilingual entry**

Bajo `## [Unreleased]` → `### 🇬🇧 Added / 🇪🇸 Añadido`, agregar:

```md
- **Phase 2a — audit infrastructure**: immutable `AuditLog` model + migration; an
  atomic `AuditService.record(entry, tx)` (+ `AuditRepository`) that writes inside the
  caller's transaction so audit and mutation commit/rollback together — no interceptor
  (it cannot join Prisma's active tx); and a global `AllExceptionsFilter` mapping
  Prisma errors (`P2002→409`, `P2025→404`) to the standard error shape, so a forgotten
  `catch` never leaks a raw 500. The substrate every later Phase-2 mutation audits to. /
  **Fase 2a — infraestructura de auditoría**: modelo inmutable `AuditLog` + migración;
  un `AuditService.record(entry, tx)` atómico (+ `AuditRepository`) que escribe dentro
  de la transacción del llamador (auditoría y mutación commitean/rollbackean juntas) —
  sin interceptor (no puede unirse a la tx activa de Prisma); y un `AllExceptionsFilter`
  global que mapea errores de Prisma (`P2002→409`, `P2025→404`) a la forma estándar,
  para que un `catch` olvidado nunca filtre un 500 crudo.
  → `apps/api/src/common/audit/`, `apps/api/src/common/filters/`, `apps/api/prisma/`
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): record Phase 2a audit infrastructure"
```

---

## Self-Review

- **Spec coverage:** 2a = `AuditLog` (Task 1) + `AuditService`/`AuditRepository`/
  `AuditModule` (Task 2) + `AllExceptionsFilter` (Task 3) + CHANGELOG (Task 4). Matches
  the spec's "Infra de auditoría (slice 2a)" deliverables. Pagination DTO is **not**
  here (no list endpoint in 2a) — it lands in 2b with the first list.
- **No interceptor** anywhere — consistent with the spec decision §2.
- **Type consistency:** `AuditEntry`/`AuditLogRow` defined in Task 2 Step 1; `record`
  and `insert` signatures match across service, repository, and tests.
- **Atomicity** is the one behavior worth proving against a real DB → the rollback
  integration test (Task 2 Step 6) is the gate.
