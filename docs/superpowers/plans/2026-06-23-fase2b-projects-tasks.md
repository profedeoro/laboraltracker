# Fase 2b — Projects & Tasks (CRUD + audit) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** First domain CRUD on the platform — `Project` and `Task`, company-scoped,
RBAC-gated — that **proves the 2a audit substrate end-to-end** (a soft-delete writes an
immutable `AuditLog` row) and establishes the **paginated response seam**.

**Architecture:** Two NestJS modules (`projects`, `tasks`) following the established
`controller → service → repository (Prisma)` layering and the `AuthGuard(global) →
PermissionGuard → TenantGuard` chain from Phase 1. `companyId` always comes from the
token (`@CurrentUser().companyId`), never client input. Soft-deletes run inside a
`$transaction` that also calls `AuditService.record(..., tx)` — atomic audit.

**Tech Stack:** NestJS 11 · Prisma 6 · Postgres · `ulid` · `class-validator` · Jest +
supertest.

## Global Constraints

- **Tenant from the token, never the client.** Every query is scoped by
  `@CurrentUser().companyId`; no endpoint accepts a `companyId` from the body/param.
- **Guards:** `AuthGuard` is global (Phase 1). Each controller adds
  `@UseGuards(PermissionGuard, TenantGuard)` and `@RequirePermission('<key>')` per route.
- **Soft-delete:** `deletedAt` set, never physical `DELETE`. All reads filter
  `deletedAt: null`. Soft-delete endpoints return **200** with the marked resource.
- **Audit (proportion by sensitivity):** only `project.deleted` / `task.deleted` are
  audited (create/rename/archive/complete are not). The audit write is
  `AuditService.record(entry, tx)` **inside the same `$transaction`** as the soft-delete.
- **Pagination seam (day one):** list endpoints accept `?page=1&limit=20` (`PaginationDto`)
  and return `{ data, meta: { page, limit, total, totalPages } }`. Repositories return
  `{ data, total }`, never a bare array.
- **Unique names are partial** (`WHERE "deletedAt" IS NULL`) so a soft-deleted name can be
  reused. `@@unique` is NOT declared in Prisma; the partial index is raw SQL in the
  migration (same pattern as the Phase 1a `Role` global unique).
- **Errors:** the global `AllExceptionsFilter` (2a) maps Prisma `P2002→409`; services
  throw `NotFoundException` for a missing/foreign-tenant resource. No raw 500s.
- **PKs:** `String @id` ULID provided by the service (no `@default`).
- **PrismaService access:** repositories hold `PrismaService` and accept an optional
  `tx?: Prisma.TransactionClient` (same shape as `AuditRepository.insert`). The service
  owns the `$transaction` for the audited soft-delete.
- **Conventional commits, SIN** trailer de IA. CHANGELOG bilingüe EN/ES.

**Branch:** `feature/fase2b-projects-tasks` (no implementar en `master`).

**AuthCtx fields** (from Phase 1, `common/auth/auth-context.ts`): `{ userId, email,
companyId, roleKey, permissions, sessionId }`. Use `user.companyId` (tenant scope) and
`user.userId` (audit actor).

---

### Task 1: `Project` + `Task` schema, migration (partial unique), seed permissions

**Files:**
- Modify: `apps/api/prisma/schema.prisma` (add `Project`, `Task`; add `Company.projects`,
  `Company.tasks`)
- Create: `apps/api/prisma/migrations/<ts>_add_projects_tasks/migration.sql` (edited to add
  partial unique indexes)
- Modify: `apps/api/prisma/seed.ts` (add `project.*` / `task.*` permissions + role grants)

**Interfaces:**
- Produces: Prisma models `Project`/`Task` and their generated client types, consumed by
  the repositories in Tasks 3-4.

- [ ] **Step 1: Add the models + Company back-relations**

In `apps/api/prisma/schema.prisma`, add after the `Team` model (before the `RBAC`
section is fine — keep it tidy):

```prisma
// ──────────────────── PROYECTOS / TAREAS ────────────────────

/// Proyecto de una empresa (espejo del `project` del agente; mismo ULID en sync).
/// Baja lógica. Unicidad de nombre = índice parcial WHERE deletedAt IS NULL (migración).
model Project {
  id        String    @id // ULID provisto por el servicio
  companyId String
  name      String
  color     String?
  archived  Boolean   @default(false)
  createdAt DateTime  @default(now()) @db.Timestamptz(3)
  deletedAt DateTime? @db.Timestamptz(3)

  company Company @relation(fields: [companyId], references: [id], onDelete: Restrict)
  tasks   Task[]

  @@index([companyId])
}

/// Tarea dentro de un proyecto. Baja lógica. Unicidad de nombre por proyecto = índice
/// parcial WHERE deletedAt IS NULL (migración).
model Task {
  id        String    @id
  companyId String
  projectId String
  name      String
  completed Boolean   @default(false)
  createdAt DateTime  @default(now()) @db.Timestamptz(3)
  deletedAt DateTime? @db.Timestamptz(3)

  company Company @relation(fields: [companyId], references: [id], onDelete: Restrict)
  project Project @relation(fields: [projectId], references: [id], onDelete: Restrict)

  @@index([companyId])
  @@index([projectId])
}
```

In the `Company` model, add to the relation list (next to `auditLogs`):

```prisma
  projects  Project[]
  tasks     Task[]
```

- [ ] **Step 2: Generate the migration WITHOUT applying (so we can add partial indexes)**

Run: `cd apps/api && pnpm exec prisma migrate dev --create-only --name add_projects_tasks`
Expected: a new `prisma/migrations/<ts>_add_projects_tasks/migration.sql` with
`CREATE TABLE "Project"`, `CREATE TABLE "Task"`, FKs (`ON DELETE RESTRICT`), and the
two `@@index` indexes — **not yet applied**.

- [ ] **Step 3: Append the partial unique indexes to the generated migration**

Edit the new `migration.sql`, append at the end:

```sql
-- Unique project name per company, ignoring soft-deleted rows (name can be reused).
CREATE UNIQUE INDEX "Project_companyId_name_active"
  ON "Project"("companyId", "name") WHERE "deletedAt" IS NULL;

-- Unique task name per project, ignoring soft-deleted rows.
CREATE UNIQUE INDEX "Task_projectId_name_active"
  ON "Task"("projectId", "name") WHERE "deletedAt" IS NULL;
```

- [ ] **Step 4: Apply the migration**

Run: `cd apps/api && pnpm exec prisma migrate dev`
Expected: applies `add_projects_tasks`, regenerates the client. `prisma.project` /
`prisma.task` now exist.

- [ ] **Step 5: Add permissions + role grants to the seed**

In `apps/api/prisma/seed.ts`, extend the `PERMISSIONS` array and `ROLE_PERMISSIONS` map:

```ts
const PERMISSIONS = [
  'member.read',
  'member.manage',
  'report.view',
  'project.read',
  'project.create',
  'project.update',
  'project.delete',
  'task.read',
  'task.create',
  'task.update',
  'task.delete',
];

const ROLE_PERMISSIONS: Record<RoleKey, string[]> = {
  SUPER_ADMIN: [],
  COMPANY_ADMIN: [
    'member.read', 'member.manage', 'report.view',
    'project.read', 'project.create', 'project.update', 'project.delete',
    'task.read', 'task.create', 'task.update', 'task.delete',
  ],
  MANAGER: [
    'member.read', 'report.view',
    'project.read', 'project.create', 'project.update', 'project.delete',
    'task.read', 'task.create', 'task.update', 'task.delete',
  ],
  EMPLOYEE: ['project.read', 'task.read', 'task.update'],
  CLIENT_VIEWER: ['report.view', 'project.read'],
};
```

- [ ] **Step 6: Re-run the seed and verify it is idempotent**

Run: `cd apps/api && pnpm exec prisma db seed`
Expected: "Seed done." with no error; re-running grants the new permissions to existing
roles (the upsert handles it).

- [ ] **Step 7: Commit**

```bash
git add apps/api/prisma/schema.prisma apps/api/prisma/migrations apps/api/prisma/seed.ts
git commit -m "feat(api): Project + Task models, partial-unique migration, seed perms (Phase 2b)"
```

---

### Task 2: Shared pagination DTO + result shape

**Files:**
- Create: `apps/api/src/common/pagination/pagination.dto.ts`
- Create: `apps/api/src/common/pagination/paginated.ts`
- Test: `apps/api/src/common/pagination/paginated.spec.ts` (unit)

**Interfaces:**
- Produces:
  - `PaginationDto { page: number; limit: number }` (validated query DTO, defaults 1/20).
  - `Paginated<T> { data: T[]; meta: { page; limit; total; totalPages } }`.
  - `paginate<T>(data: T[], total: number, dto: PaginationDto): Paginated<T>`.

- [ ] **Step 1: Write the failing unit test for the meta builder**

`apps/api/src/common/pagination/paginated.spec.ts`:

```ts
import { paginate } from './paginated';

describe('paginate', () => {
  it('builds meta with ceil(total/limit) pages', () => {
    const res = paginate(['a', 'b'], 5, { page: 1, limit: 2 });
    expect(res.data).toEqual(['a', 'b']);
    expect(res.meta).toEqual({ page: 1, limit: 2, total: 5, totalPages: 3 });
  });

  it('reports 0 pages for an empty set', () => {
    const res = paginate([], 0, { page: 1, limit: 20 });
    expect(res.meta).toEqual({ page: 1, limit: 20, total: 0, totalPages: 0 });
  });
});
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `cd apps/api && pnpm test -- paginated`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement the DTO and the result shape**

`apps/api/src/common/pagination/pagination.dto.ts`:

```ts
import { Type } from 'class-transformer';
import { IsInt, IsOptional, Max, Min } from 'class-validator';

export class PaginationDto {
  @IsOptional()
  @Type(() => Number)
  @IsInt()
  @Min(1)
  page: number = 1;

  @IsOptional()
  @Type(() => Number)
  @IsInt()
  @Min(1)
  @Max(100)
  limit: number = 20;
}
```

`apps/api/src/common/pagination/paginated.ts`:

```ts
export interface Paginated<T> {
  data: T[];
  meta: { page: number; limit: number; total: number; totalPages: number };
}

export function paginate<T>(
  data: T[],
  total: number,
  opts: { page: number; limit: number },
): Paginated<T> {
  return {
    data,
    meta: {
      page: opts.page,
      limit: opts.limit,
      total,
      totalPages: Math.ceil(total / opts.limit),
    },
  };
}
```

- [ ] **Step 4: Run the test to confirm it passes**

Run: `cd apps/api && pnpm test -- paginated`
Expected: PASS (2/2).

- [ ] **Step 5: Commit**

```bash
git add apps/api/src/common/pagination
git commit -m "feat(api): shared PaginationDto + Paginated result shape (Phase 2b)"
```

---

### Task 3: Projects module (CRUD + audited soft-delete)

**Files:**
- Create: `apps/api/src/modules/projects/projects.types.ts`
- Create: `apps/api/src/modules/projects/dto/create-project.dto.ts`
- Create: `apps/api/src/modules/projects/dto/update-project.dto.ts`
- Create: `apps/api/src/modules/projects/projects.repository.ts`
- Create: `apps/api/src/modules/projects/projects.service.ts`
- Create: `apps/api/src/modules/projects/projects.controller.ts`
- Create: `apps/api/src/modules/projects/projects.module.ts`
- Test: `apps/api/src/modules/projects/projects.service.spec.ts` (unit)
- Test: `apps/api/src/modules/projects/projects.repository.spec.ts` (integration)
- Modify: `apps/api/src/app.module.ts` (import `ProjectsModule`)

**Interfaces:**
- Consumes: `AuditService` (global), `AuditActions.ProjectDeleted`, `PrismaService`,
  `PermissionGuard`/`TenantGuard` (from `AuthModule`), `PaginationDto`/`paginate`.
- Produces: `ProjectDto { id, companyId, name, color, archived, createdAt, deletedAt }`;
  REST routes under `/projects`.

- [ ] **Step 1: Types + DTOs**

`apps/api/src/modules/projects/projects.types.ts`:

```ts
export interface ProjectDto {
  id: string;
  companyId: string;
  name: string;
  color: string | null;
  archived: boolean;
  createdAt: Date;
  deletedAt: Date | null;
}
```

`apps/api/src/modules/projects/dto/create-project.dto.ts`:

```ts
import { IsHexColor, IsOptional, IsString, MaxLength, MinLength } from 'class-validator';
import { Transform } from 'class-transformer';

export class CreateProjectDto {
  @IsString()
  @Transform(({ value }) => (typeof value === 'string' ? value.trim() : value))
  @MinLength(1)
  @MaxLength(120)
  name!: string;

  @IsOptional()
  @IsHexColor()
  color?: string;
}
```

`apps/api/src/modules/projects/dto/update-project.dto.ts`:

```ts
import { IsBoolean, IsHexColor, IsOptional, IsString, MaxLength, MinLength } from 'class-validator';
import { Transform } from 'class-transformer';

export class UpdateProjectDto {
  @IsOptional()
  @IsString()
  @Transform(({ value }) => (typeof value === 'string' ? value.trim() : value))
  @MinLength(1)
  @MaxLength(120)
  name?: string;

  @IsOptional()
  @IsHexColor()
  color?: string;

  @IsOptional()
  @IsBoolean()
  archived?: boolean;
}
```

- [ ] **Step 2: Repository (tenant-scoped, tx-aware)**

`apps/api/src/modules/projects/projects.repository.ts`:

```ts
import { Injectable } from '@nestjs/common';
import { Prisma, Project } from '@prisma/client';
import { PrismaService } from '../../database/prisma.service';

type Db = PrismaService | Prisma.TransactionClient;

@Injectable()
export class ProjectsRepository {
  constructor(private readonly prisma: PrismaService) {}

  async listByCompany(
    companyId: string,
    opts: { page: number; limit: number },
  ): Promise<{ data: Project[]; total: number }> {
    const where = { companyId, deletedAt: null };
    const [data, total] = await this.prisma.$transaction([
      this.prisma.project.findMany({
        where,
        orderBy: { createdAt: 'asc' },
        skip: (opts.page - 1) * opts.limit,
        take: opts.limit,
      }),
      this.prisma.project.count({ where }),
    ]);
    return { data, total };
  }

  findActive(companyId: string, id: string, tx?: Prisma.TransactionClient): Promise<Project | null> {
    const db: Db = tx ?? this.prisma;
    return db.project.findFirst({ where: { id, companyId, deletedAt: null } });
  }

  create(id: string, companyId: string, data: { name: string; color?: string }): Promise<Project> {
    return this.prisma.project.create({
      data: { id, companyId, name: data.name, color: data.color ?? null },
    });
  }

  update(id: string, data: Prisma.ProjectUpdateInput, tx?: Prisma.TransactionClient): Promise<Project> {
    const db: Db = tx ?? this.prisma;
    return db.project.update({ where: { id }, data });
  }
}
```

- [ ] **Step 3: Write the failing service unit test**

`apps/api/src/modules/projects/projects.service.spec.ts`:

```ts
import { NotFoundException } from '@nestjs/common';
import { ProjectsService } from './projects.service';
import { ProjectsRepository } from './projects.repository';
import { AuditService } from '../../common/audit/audit.service';
import { AuditActions } from '../../common/audit/audit-actions';

function makeProject(over: Partial<Record<string, unknown>> = {}) {
  return {
    id: 'p1', companyId: 'c1', name: 'Alpha', color: null, archived: false,
    createdAt: new Date(), deletedAt: null, ...over,
  };
}

describe('ProjectsService', () => {
  it('soft-deletes inside a tx and records an audit entry with before/after', async () => {
    const before = makeProject();
    const after = makeProject({ deletedAt: new Date() });
    const findActive = jest.fn().mockResolvedValue(before);
    const update = jest.fn().mockResolvedValue(after);
    const repo = { findActive, update } as unknown as ProjectsRepository;
    const record = jest.fn().mockResolvedValue(undefined);
    const audit = { record } as unknown as AuditService;
    // prisma.$transaction(cb) just runs the callback with a fake tx
    const prisma = { $transaction: (cb: (tx: unknown) => unknown) => cb({}) } as never;

    const service = new ProjectsService(prisma, repo, audit);
    const result = await service.remove('c1', 'u1', 'p1');

    expect(result.deletedAt).not.toBeNull();
    expect(record).toHaveBeenCalledTimes(1);
    const [entry] = record.mock.calls[0];
    expect(entry).toMatchObject({
      action: AuditActions.ProjectDeleted,
      companyId: 'c1',
      actorUserId: 'u1',
      entityType: 'Project',
      entityId: 'p1',
    });
    expect(entry.before).toBe(before);
    expect(entry.after).toBe(after);
  });

  it('throws NotFound when the project is absent or belongs to another tenant', async () => {
    const findActive = jest.fn().mockResolvedValue(null);
    const repo = { findActive } as unknown as ProjectsRepository;
    const audit = { record: jest.fn() } as unknown as AuditService;
    const prisma = { $transaction: (cb: (tx: unknown) => unknown) => cb({}) } as never;

    const service = new ProjectsService(prisma, repo, audit);
    await expect(service.remove('c1', 'u1', 'missing')).rejects.toBeInstanceOf(NotFoundException);
  });
});
```

- [ ] **Step 4: Run it to confirm it fails**

Run: `cd apps/api && pnpm test -- projects.service`
Expected: FAIL — service/repo modules not found.

- [ ] **Step 5: Implement the service**

`apps/api/src/modules/projects/projects.service.ts`:

```ts
import { Injectable, NotFoundException } from '@nestjs/common';
import { ulid } from 'ulid';
import { Project } from '@prisma/client';
import { PrismaService } from '../../database/prisma.service';
import { ProjectsRepository } from './projects.repository';
import { AuditService } from '../../common/audit/audit.service';
import { AuditActions } from '../../common/audit/audit-actions';
import { PaginationDto } from '../../common/pagination/pagination.dto';
import { paginate, Paginated } from '../../common/pagination/paginated';
import { ProjectDto } from './projects.types';
import { CreateProjectDto } from './dto/create-project.dto';
import { UpdateProjectDto } from './dto/update-project.dto';

@Injectable()
export class ProjectsService {
  constructor(
    private readonly prisma: PrismaService,
    private readonly repo: ProjectsRepository,
    private readonly audit: AuditService,
  ) {}

  async list(companyId: string, pagination: PaginationDto): Promise<Paginated<ProjectDto>> {
    const { data, total } = await this.repo.listByCompany(companyId, pagination);
    return paginate(data.map(this.toDto), total, pagination);
  }

  async create(companyId: string, dto: CreateProjectDto): Promise<ProjectDto> {
    const project = await this.repo.create(ulid(), companyId, dto);
    return this.toDto(project);
  }

  async update(companyId: string, id: string, dto: UpdateProjectDto): Promise<ProjectDto> {
    const existing = await this.repo.findActive(companyId, id);
    if (!existing) throw new NotFoundException('Project not found');
    const updated = await this.repo.update(id, {
      name: dto.name,
      color: dto.color,
      archived: dto.archived,
    });
    return this.toDto(updated);
  }

  async remove(companyId: string, actorUserId: string, id: string): Promise<ProjectDto> {
    const after = await this.prisma.$transaction(async (tx) => {
      const before = await this.repo.findActive(companyId, id, tx);
      if (!before) throw new NotFoundException('Project not found');
      const deleted = await this.repo.update(id, { deletedAt: new Date() }, tx);
      await this.audit.record(
        {
          action: AuditActions.ProjectDeleted,
          companyId,
          actorUserId,
          entityType: 'Project',
          entityId: id,
          before,
          after: deleted,
        },
        tx,
      );
      return deleted;
    });
    return this.toDto(after);
  }

  private toDto(p: Project): ProjectDto {
    return {
      id: p.id,
      companyId: p.companyId,
      name: p.name,
      color: p.color,
      archived: p.archived,
      createdAt: p.createdAt,
      deletedAt: p.deletedAt,
    };
  }
}
```

- [ ] **Step 6: Run the service test to confirm it passes**

Run: `cd apps/api && pnpm test -- projects.service`
Expected: PASS (2/2).

- [ ] **Step 7: Controller + module**

`apps/api/src/modules/projects/projects.controller.ts`:

```ts
import { Body, Controller, Delete, Get, Param, Patch, Post, Query, UseGuards } from '@nestjs/common';
import { PermissionGuard } from '../../common/guards/permission.guard';
import { TenantGuard } from '../../common/guards/tenant.guard';
import { RequirePermission } from '../../common/decorators/require-permission.decorator';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import type { AuthCtx } from '../../common/auth/auth-context';
import { PaginationDto } from '../../common/pagination/pagination.dto';
import { ProjectsService } from './projects.service';
import { CreateProjectDto } from './dto/create-project.dto';
import { UpdateProjectDto } from './dto/update-project.dto';

@Controller('projects')
@UseGuards(PermissionGuard, TenantGuard)
export class ProjectsController {
  constructor(private readonly projects: ProjectsService) {}

  @Get()
  @RequirePermission('project.read')
  list(@CurrentUser() user: AuthCtx, @Query() pagination: PaginationDto) {
    return this.projects.list(user.companyId, pagination);
  }

  @Post()
  @RequirePermission('project.create')
  create(@CurrentUser() user: AuthCtx, @Body() dto: CreateProjectDto) {
    return this.projects.create(user.companyId, dto);
  }

  @Patch(':id')
  @RequirePermission('project.update')
  update(@CurrentUser() user: AuthCtx, @Param('id') id: string, @Body() dto: UpdateProjectDto) {
    return this.projects.update(user.companyId, id, dto);
  }

  @Delete(':id')
  @RequirePermission('project.delete')
  remove(@CurrentUser() user: AuthCtx, @Param('id') id: string) {
    return this.projects.remove(user.companyId, user.userId, id);
  }
}
```

`apps/api/src/modules/projects/projects.module.ts`:

```ts
import { Module } from '@nestjs/common';
import { AuthModule } from '../auth/auth.module';
import { ProjectsController } from './projects.controller';
import { ProjectsService } from './projects.service';
import { ProjectsRepository } from './projects.repository';

@Module({
  imports: [AuthModule], // PermissionGuard + TenantGuard (+ AuthRepository the TenantGuard uses)
  controllers: [ProjectsController],
  providers: [ProjectsService, ProjectsRepository],
})
export class ProjectsModule {}
```

Register in `apps/api/src/app.module.ts` `imports`: add `ProjectsModule` (after `MembersModule`).

- [ ] **Step 8: Integration test for the repository (real Postgres)**

`apps/api/src/modules/projects/projects.repository.spec.ts`:

```ts
import { ulid } from 'ulid';
import { PrismaService } from '../../database/prisma.service';
import { ProjectsRepository } from './projects.repository';

describe('ProjectsRepository (integration, real DB)', () => {
  const prisma = new PrismaService();
  const repo = new ProjectsRepository(prisma);
  let companyId: string;
  const made: string[] = [];

  beforeAll(async () => {
    await prisma.$connect();
    const company = await prisma.company.findFirstOrThrow({ where: { slug: 'empresa-a' } });
    companyId = company.id;
  });
  afterAll(async () => {
    if (made.length) await prisma.project.deleteMany({ where: { id: { in: made } } });
    await prisma.$disconnect();
  });

  it('creates, lists (excluding soft-deleted), and scopes by company', async () => {
    const name = `IT Project ${ulid()}`;
    const created = await repo.create(ulid(), companyId, { name });
    made.push(created.id);

    const listed = await repo.listByCompany(companyId, { page: 1, limit: 100 });
    expect(listed.data.some((p) => p.id === created.id)).toBe(true);

    await repo.update(created.id, { deletedAt: new Date() });
    const afterDelete = await repo.listByCompany(companyId, { page: 1, limit: 100 });
    expect(afterDelete.data.some((p) => p.id === created.id)).toBe(false);

    // findActive does not return a soft-deleted row
    expect(await repo.findActive(companyId, created.id)).toBeNull();
  });

  it('findActive is tenant-scoped (other company cannot see it)', async () => {
    const created = await repo.create(ulid(), companyId, { name: `IT Scoped ${ulid()}` });
    made.push(created.id);
    expect(await repo.findActive('non-existent-company', created.id)).toBeNull();
  });
});
```

- [ ] **Step 9: Run integration + full unit suite + build**

Run: `cd apps/api && pnpm test:integration -- projects.repository`
Expected: PASS (2/2).
Run: `cd apps/api && pnpm test`
Expected: PASS (existing + `projects.service` + `paginated`).
Run: `cd apps/api && pnpm build`
Expected: exit 0.

- [ ] **Step 10: Commit**

```bash
git add apps/api/src/modules/projects apps/api/src/app.module.ts
git commit -m "feat(api): Projects CRUD with audited soft-delete + pagination (Phase 2b)"
```

---

### Task 4: Tasks module (CRUD + audited soft-delete)

**Files:**
- Create: `apps/api/src/modules/tasks/tasks.types.ts`
- Create: `apps/api/src/modules/tasks/dto/create-task.dto.ts`
- Create: `apps/api/src/modules/tasks/dto/update-task.dto.ts`
- Create: `apps/api/src/modules/tasks/tasks.repository.ts`
- Create: `apps/api/src/modules/tasks/tasks.service.ts`
- Create: `apps/api/src/modules/tasks/tasks.controller.ts`
- Create: `apps/api/src/modules/tasks/tasks.module.ts`
- Test: `apps/api/src/modules/tasks/tasks.service.spec.ts` (unit)
- Test: `apps/api/src/modules/tasks/tasks.repository.spec.ts` (integration)
- Modify: `apps/api/src/app.module.ts` (import `TasksModule`)

**Interfaces:**
- Consumes: same as Projects + the `Project` ownership check (a task is created under a
  project of the same company).
- Produces: `TaskDto { id, companyId, projectId, name, completed, createdAt, deletedAt }`;
  routes `GET/POST /projects/:projectId/tasks`, `PATCH/DELETE /tasks/:id`.

- [ ] **Step 1: Types + DTOs**

`apps/api/src/modules/tasks/tasks.types.ts`:

```ts
export interface TaskDto {
  id: string;
  companyId: string;
  projectId: string;
  name: string;
  completed: boolean;
  createdAt: Date;
  deletedAt: Date | null;
}
```

`apps/api/src/modules/tasks/dto/create-task.dto.ts`:

```ts
import { IsString, MaxLength, MinLength } from 'class-validator';
import { Transform } from 'class-transformer';

export class CreateTaskDto {
  @IsString()
  @Transform(({ value }) => (typeof value === 'string' ? value.trim() : value))
  @MinLength(1)
  @MaxLength(160)
  name!: string;
}
```

`apps/api/src/modules/tasks/dto/update-task.dto.ts`:

```ts
import { IsBoolean, IsOptional, IsString, MaxLength, MinLength } from 'class-validator';
import { Transform } from 'class-transformer';

export class UpdateTaskDto {
  @IsOptional()
  @IsString()
  @Transform(({ value }) => (typeof value === 'string' ? value.trim() : value))
  @MinLength(1)
  @MaxLength(160)
  name?: string;

  @IsOptional()
  @IsBoolean()
  completed?: boolean;
}
```

- [ ] **Step 2: Repository (tenant + project scoped, tx-aware)**

`apps/api/src/modules/tasks/tasks.repository.ts`:

```ts
import { Injectable } from '@nestjs/common';
import { Prisma, Task } from '@prisma/client';
import { PrismaService } from '../../database/prisma.service';

type Db = PrismaService | Prisma.TransactionClient;

@Injectable()
export class TasksRepository {
  constructor(private readonly prisma: PrismaService) {}

  async listByProject(
    companyId: string,
    projectId: string,
    opts: { page: number; limit: number },
  ): Promise<{ data: Task[]; total: number }> {
    const where = { companyId, projectId, deletedAt: null };
    const [data, total] = await this.prisma.$transaction([
      this.prisma.task.findMany({
        where,
        orderBy: { createdAt: 'asc' },
        skip: (opts.page - 1) * opts.limit,
        take: opts.limit,
      }),
      this.prisma.task.count({ where }),
    ]);
    return { data, total };
  }

  findActive(companyId: string, id: string, tx?: Prisma.TransactionClient): Promise<Task | null> {
    const db: Db = tx ?? this.prisma;
    return db.task.findFirst({ where: { id, companyId, deletedAt: null } });
  }

  /** A project of THIS company that is not soft-deleted (ownership check for create). */
  activeProjectExists(companyId: string, projectId: string): Promise<{ id: string } | null> {
    return this.prisma.project.findFirst({
      where: { id: projectId, companyId, deletedAt: null },
      select: { id: true },
    });
  }

  create(
    id: string,
    companyId: string,
    projectId: string,
    data: { name: string },
  ): Promise<Task> {
    return this.prisma.task.create({ data: { id, companyId, projectId, name: data.name } });
  }

  update(id: string, data: Prisma.TaskUpdateInput, tx?: Prisma.TransactionClient): Promise<Task> {
    const db: Db = tx ?? this.prisma;
    return db.task.update({ where: { id }, data });
  }
}
```

- [ ] **Step 3: Write the failing service unit test**

`apps/api/src/modules/tasks/tasks.service.spec.ts`:

```ts
import { NotFoundException } from '@nestjs/common';
import { TasksService } from './tasks.service';
import { TasksRepository } from './tasks.repository';
import { AuditService } from '../../common/audit/audit.service';
import { AuditActions } from '../../common/audit/audit-actions';

function makeTask(over: Partial<Record<string, unknown>> = {}) {
  return {
    id: 't1', companyId: 'c1', projectId: 'p1', name: 'Task', completed: false,
    createdAt: new Date(), deletedAt: null, ...over,
  };
}

describe('TasksService', () => {
  it('rejects create under a project of another tenant (NotFound)', async () => {
    const activeProjectExists = jest.fn().mockResolvedValue(null);
    const repo = { activeProjectExists } as unknown as TasksRepository;
    const audit = { record: jest.fn() } as unknown as AuditService;
    const prisma = { $transaction: (cb: (tx: unknown) => unknown) => cb({}) } as never;

    const service = new TasksService(prisma, repo, audit);
    await expect(service.create('c1', 'p1', { name: 'X' })).rejects.toBeInstanceOf(NotFoundException);
  });

  it('soft-deletes inside a tx and audits task.deleted with before/after', async () => {
    const before = makeTask();
    const after = makeTask({ deletedAt: new Date() });
    const findActive = jest.fn().mockResolvedValue(before);
    const update = jest.fn().mockResolvedValue(after);
    const repo = { findActive, update } as unknown as TasksRepository;
    const record = jest.fn().mockResolvedValue(undefined);
    const audit = { record } as unknown as AuditService;
    const prisma = { $transaction: (cb: (tx: unknown) => unknown) => cb({}) } as never;

    const service = new TasksService(prisma, repo, audit);
    await service.remove('c1', 'u1', 't1');

    const [entry] = record.mock.calls[0];
    expect(entry).toMatchObject({
      action: AuditActions.TaskDeleted,
      companyId: 'c1',
      actorUserId: 'u1',
      entityType: 'Task',
      entityId: 't1',
    });
    expect(entry.before).toBe(before);
    expect(entry.after).toBe(after);
  });
});
```

- [ ] **Step 4: Run it to confirm it fails**

Run: `cd apps/api && pnpm test -- tasks.service`
Expected: FAIL — modules not found.

- [ ] **Step 5: Implement the service**

`apps/api/src/modules/tasks/tasks.service.ts`:

```ts
import { Injectable, NotFoundException } from '@nestjs/common';
import { ulid } from 'ulid';
import { Task } from '@prisma/client';
import { PrismaService } from '../../database/prisma.service';
import { TasksRepository } from './tasks.repository';
import { AuditService } from '../../common/audit/audit.service';
import { AuditActions } from '../../common/audit/audit-actions';
import { PaginationDto } from '../../common/pagination/pagination.dto';
import { paginate, Paginated } from '../../common/pagination/paginated';
import { TaskDto } from './tasks.types';
import { CreateTaskDto } from './dto/create-task.dto';
import { UpdateTaskDto } from './dto/update-task.dto';

@Injectable()
export class TasksService {
  constructor(
    private readonly prisma: PrismaService,
    private readonly repo: TasksRepository,
    private readonly audit: AuditService,
  ) {}

  async list(companyId: string, projectId: string, pagination: PaginationDto): Promise<Paginated<TaskDto>> {
    const { data, total } = await this.repo.listByProject(companyId, projectId, pagination);
    return paginate(data.map(this.toDto), total, pagination);
  }

  async create(companyId: string, projectId: string, dto: CreateTaskDto): Promise<TaskDto> {
    const project = await this.repo.activeProjectExists(companyId, projectId);
    if (!project) throw new NotFoundException('Project not found');
    const task = await this.repo.create(ulid(), companyId, projectId, dto);
    return this.toDto(task);
  }

  async update(companyId: string, id: string, dto: UpdateTaskDto): Promise<TaskDto> {
    const existing = await this.repo.findActive(companyId, id);
    if (!existing) throw new NotFoundException('Task not found');
    const updated = await this.repo.update(id, { name: dto.name, completed: dto.completed });
    return this.toDto(updated);
  }

  async remove(companyId: string, actorUserId: string, id: string): Promise<TaskDto> {
    const after = await this.prisma.$transaction(async (tx) => {
      const before = await this.repo.findActive(companyId, id, tx);
      if (!before) throw new NotFoundException('Task not found');
      const deleted = await this.repo.update(id, { deletedAt: new Date() }, tx);
      await this.audit.record(
        {
          action: AuditActions.TaskDeleted,
          companyId,
          actorUserId,
          entityType: 'Task',
          entityId: id,
          before,
          after: deleted,
        },
        tx,
      );
      return deleted;
    });
    return this.toDto(after);
  }

  private toDto(t: Task): TaskDto {
    return {
      id: t.id,
      companyId: t.companyId,
      projectId: t.projectId,
      name: t.name,
      completed: t.completed,
      createdAt: t.createdAt,
      deletedAt: t.deletedAt,
    };
  }
}
```

- [ ] **Step 6: Run the service test to confirm it passes**

Run: `cd apps/api && pnpm test -- tasks.service`
Expected: PASS (2/2).

- [ ] **Step 7: Controller + module**

`apps/api/src/modules/tasks/tasks.controller.ts`:

```ts
import { Body, Controller, Delete, Get, Param, Patch, Post, Query, UseGuards } from '@nestjs/common';
import { PermissionGuard } from '../../common/guards/permission.guard';
import { TenantGuard } from '../../common/guards/tenant.guard';
import { RequirePermission } from '../../common/decorators/require-permission.decorator';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import type { AuthCtx } from '../../common/auth/auth-context';
import { PaginationDto } from '../../common/pagination/pagination.dto';
import { TasksService } from './tasks.service';
import { CreateTaskDto } from './dto/create-task.dto';
import { UpdateTaskDto } from './dto/update-task.dto';

@Controller()
@UseGuards(PermissionGuard, TenantGuard)
export class TasksController {
  constructor(private readonly tasks: TasksService) {}

  @Get('projects/:projectId/tasks')
  @RequirePermission('task.read')
  list(
    @CurrentUser() user: AuthCtx,
    @Param('projectId') projectId: string,
    @Query() pagination: PaginationDto,
  ) {
    return this.tasks.list(user.companyId, projectId, pagination);
  }

  @Post('projects/:projectId/tasks')
  @RequirePermission('task.create')
  create(
    @CurrentUser() user: AuthCtx,
    @Param('projectId') projectId: string,
    @Body() dto: CreateTaskDto,
  ) {
    return this.tasks.create(user.companyId, projectId, dto);
  }

  @Patch('tasks/:id')
  @RequirePermission('task.update')
  update(@CurrentUser() user: AuthCtx, @Param('id') id: string, @Body() dto: UpdateTaskDto) {
    return this.tasks.update(user.companyId, id, dto);
  }

  @Delete('tasks/:id')
  @RequirePermission('task.delete')
  remove(@CurrentUser() user: AuthCtx, @Param('id') id: string) {
    return this.tasks.remove(user.companyId, user.userId, id);
  }
}
```

`apps/api/src/modules/tasks/tasks.module.ts`:

```ts
import { Module } from '@nestjs/common';
import { AuthModule } from '../auth/auth.module';
import { TasksController } from './tasks.controller';
import { TasksService } from './tasks.service';
import { TasksRepository } from './tasks.repository';

@Module({
  imports: [AuthModule],
  controllers: [TasksController],
  providers: [TasksService, TasksRepository],
})
export class TasksModule {}
```

Register `TasksModule` in `apps/api/src/app.module.ts` `imports` (after `ProjectsModule`).

- [ ] **Step 8: Integration test for the repository (real Postgres)**

`apps/api/src/modules/tasks/tasks.repository.spec.ts`:

```ts
import { ulid } from 'ulid';
import { PrismaService } from '../../database/prisma.service';
import { TasksRepository } from './tasks.repository';

describe('TasksRepository (integration, real DB)', () => {
  const prisma = new PrismaService();
  const repo = new TasksRepository(prisma);
  let companyId: string;
  let projectId: string;

  beforeAll(async () => {
    await prisma.$connect();
    const company = await prisma.company.findFirstOrThrow({ where: { slug: 'empresa-a' } });
    companyId = company.id;
    const project = await prisma.project.create({
      data: { id: ulid(), companyId, name: `IT Tasks Project ${ulid()}` },
    });
    projectId = project.id;
  });
  afterAll(async () => {
    await prisma.task.deleteMany({ where: { projectId } });
    await prisma.project.delete({ where: { id: projectId } });
    await prisma.$disconnect();
  });

  it('creates a task and lists it under its project, excluding soft-deleted', async () => {
    const created = await repo.create(ulid(), companyId, projectId, { name: `T ${ulid()}` });
    const listed = await repo.listByProject(companyId, projectId, { page: 1, limit: 100 });
    expect(listed.data.some((t) => t.id === created.id)).toBe(true);

    await repo.update(created.id, { deletedAt: new Date() });
    const after = await repo.listByProject(companyId, projectId, { page: 1, limit: 100 });
    expect(after.data.some((t) => t.id === created.id)).toBe(false);
  });

  it('activeProjectExists is tenant-scoped', async () => {
    expect(await repo.activeProjectExists(companyId, projectId)).not.toBeNull();
    expect(await repo.activeProjectExists('other-company', projectId)).toBeNull();
  });
});
```

- [ ] **Step 9: Run integration + full unit suite + build**

Run: `cd apps/api && pnpm test:integration -- tasks.repository`
Expected: PASS (2/2).
Run: `cd apps/api && pnpm test` → PASS. Run: `cd apps/api && pnpm build` → exit 0.

- [ ] **Step 10: Commit**

```bash
git add apps/api/src/modules/tasks apps/api/src/app.module.ts
git commit -m "feat(api): Tasks CRUD with audited soft-delete (Phase 2b)"
```

---

### Task 5: e2e — audit end-to-end + tenant isolation

**Files:**
- Create: `apps/api/test/projects-tasks.e2e-spec.ts`

**Interfaces:**
- Consumes: the running `AppModule`, the seeded users (`admin@a.demo`, `employee@a.demo`,
  `admin@b.demo`), `Password123!`.

- [ ] **Step 1: Write the e2e (the slice's gate)**

`apps/api/test/projects-tasks.e2e-spec.ts`:

```ts
import { Test } from '@nestjs/testing';
import { INestApplication, ValidationPipe } from '@nestjs/common';
import request from 'supertest';
import { PrismaClient } from '@prisma/client';
import { AppModule } from '../src/app.module';

describe('Projects/Tasks + audit (e2e)', () => {
  let app: INestApplication;
  const prisma = new PrismaClient();
  const base = '/api/v1';

  beforeAll(async () => {
    const moduleRef = await Test.createTestingModule({ imports: [AppModule] }).compile();
    app = moduleRef.createNestApplication();
    app.setGlobalPrefix('api/v1');
    app.useGlobalPipes(
      new ValidationPipe({ whitelist: true, forbidNonWhitelisted: true, transform: true }),
    );
    await app.init();
    await prisma.$connect();
  });

  afterAll(async () => {
    await app.close();
    await prisma.$disconnect();
  });

  async function login(email: string): Promise<string> {
    const res = await request(app.getHttpServer())
      .post(`${base}/auth/login`)
      .send({ email, password: 'Password123!' })
      .expect(200);
    return res.body.accessToken as string;
  }

  it('admin creates a project, deletes it (200) and an immutable AuditLog row is written', async () => {
    const token = await login('admin@a.demo');
    const name = `E2E Project ${Date.now()}`;

    const created = await request(app.getHttpServer())
      .post(`${base}/projects`)
      .set('Authorization', `Bearer ${token}`)
      .send({ name })
      .expect(201);
    const id = created.body.id as string;

    const deleted = await request(app.getHttpServer())
      .delete(`${base}/projects/${id}`)
      .set('Authorization', `Bearer ${token}`)
      .expect(200);
    expect(deleted.body.deletedAt).not.toBeNull();

    const audit = await prisma.auditLog.findFirst({
      where: { action: 'project.deleted', entityId: id },
    });
    expect(audit).not.toBeNull();
    expect((audit!.metadata as { before?: unknown }).before).toBeDefined();

    await prisma.auditLog.delete({ where: { id: audit!.id } }); // test cleanup
    await prisma.project.delete({ where: { id } });
  });

  it('denies an EMPLOYEE without project.delete (403)', async () => {
    const adminToken = await login('admin@a.demo');
    const name = `E2E Forbidden ${Date.now()}`;
    const created = await request(app.getHttpServer())
      .post(`${base}/projects`)
      .set('Authorization', `Bearer ${adminToken}`)
      .send({ name })
      .expect(201);
    const id = created.body.id as string;

    const empToken = await login('employee@a.demo');
    await request(app.getHttpServer())
      .delete(`${base}/projects/${id}`)
      .set('Authorization', `Bearer ${empToken}`)
      .expect(403);

    await prisma.project.delete({ where: { id } }); // cleanup
  });

  it('isolates tenants: B admin cannot delete A project (404)', async () => {
    const aToken = await login('admin@a.demo');
    const created = await request(app.getHttpServer())
      .post(`${base}/projects`)
      .set('Authorization', `Bearer ${aToken}`)
      .send({ name: `E2E Tenant ${Date.now()}` })
      .expect(201);
    const id = created.body.id as string;

    const bToken = await login('admin@b.demo');
    await request(app.getHttpServer())
      .delete(`${base}/projects/${id}`)
      .set('Authorization', `Bearer ${bToken}`)
      .expect(404); // B's tenant scope never sees A's project

    await prisma.project.delete({ where: { id } }); // cleanup
  });

  it('returns a paginated shape for the list', async () => {
    const token = await login('admin@a.demo');
    const res = await request(app.getHttpServer())
      .get(`${base}/projects?page=1&limit=5`)
      .set('Authorization', `Bearer ${token}`)
      .expect(200);
    expect(res.body).toHaveProperty('data');
    expect(res.body).toHaveProperty('meta.total');
    expect(res.body).toHaveProperty('meta.totalPages');
  });
});
```

- [ ] **Step 2: Run the e2e**

Run: `cd apps/api && pnpm test:e2e -- projects-tasks`
Expected: PASS (4/4). (Requires the seeded DB + applied migrations.)

- [ ] **Step 3: Commit**

```bash
git add apps/api/test/projects-tasks.e2e-spec.ts
git commit -m "test(api): e2e for projects/tasks audit + tenant isolation (Phase 2b)"
```

---

### Task 6: CHANGELOG

**Files:**
- Modify: `CHANGELOG.md` (Unreleased → Added, bilingual)

- [ ] **Step 1: Add a bilingual entry**

```md
- **Phase 2b — Projects & Tasks CRUD**: company-scoped `Project`/`Task` with RBAC +
  tenant guards; soft-delete returns 200 and writes an immutable `AuditLog`
  (`project.deleted`/`task.deleted`, with the pre-image) inside the same transaction —
  proving the 2a audit substrate end-to-end. Establishes the paginated `{ data, meta }`
  response seam. Unique names are partial (`WHERE deletedAt IS NULL`) so a deleted name
  can be reused. /
  **Fase 2b — CRUD de Projects & Tasks**: `Project`/`Task` por empresa con guards de RBAC
  + tenant; el soft-delete devuelve 200 y escribe un `AuditLog` inmutable
  (`project.deleted`/`task.deleted`, con la pre-imagen) dentro de la misma transacción —
  probando el sustrato de auditoría de 2a de punta a punta. Establece el seam de respuesta
  paginada `{ data, meta }`. Los nombres únicos son parciales (`WHERE deletedAt IS NULL`)
  para poder reusar un nombre borrado.
  → `apps/api/src/modules/projects/`, `apps/api/src/modules/tasks/`, `apps/api/src/common/pagination/`, `apps/api/prisma/`
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): record Phase 2b projects/tasks CRUD"
```

---

## Self-Review

- **Spec coverage:** projects + tasks CRUD (Tasks 3-4), audited soft-delete (`project.deleted`
  / `task.deleted` only — proportion), pagination seam (Task 2), permissions seed (Task 1),
  e2e proving audit end-to-end + tenant isolation + 403 (Task 5). Matches spec slice 2b.
- **Tenant safety:** every read/mutation is scoped by `companyId` from the token;
  `findActive(companyId, id)` returns null for a foreign tenant → `NotFoundException` (404),
  proven by the e2e (B admin → 404 on A's project) and repo integration tests.
- **Atomic audit:** soft-delete + `audit.record` share one `$transaction`; the service owns
  the transaction, repo methods accept the `tx`. Mirrors the 2a pattern.
- **Type consistency:** `ProjectDto`/`TaskDto`, repository signatures, and service calls
  line up across tasks; controllers use `user.companyId` (scope) and `user.userId` (actor).
- **No interceptor, no client-supplied companyId, partial unique indexes** — all per the
  Global Constraints.
- **Known nuance:** `@@unique` is intentionally absent from the Prisma schema (the unique is
  the raw partial index); Prisma won't expose it as a compound unique, which is fine — we
  rely on the DB index + `AllExceptionsFilter` P2002→409.
