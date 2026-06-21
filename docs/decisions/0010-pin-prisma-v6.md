# 0010 — Pin Prisma to v6 (defer the v7 driver-adapter model)

- **Status:** Accepted
- **Date:** 2026-06-21

## 🇬🇧 Context
While executing Phase 1a (data layer), `pnpm add prisma @prisma/client` resolved to
**Prisma 7.8.0** (the latest), not the v6 the plan assumed. Prisma 7 introduces two
breaking changes that ripple into our design:

1. `url = env("DATABASE_URL")` is **no longer allowed in `schema.prisma`**. The
   connection must move to a `prisma.config.ts` file and a **driver adapter**
   (`@prisma/adapter-pg` + `pg`) passed to the `PrismaClient` constructor.
2. Single-line enum bodies no longer parse (cosmetic, easy to fix).

Change (1) is the consequential one: it changes how `PrismaService` is constructed
and adds two runtime dependencies plus a config file. Prisma 7 was released only
weeks before this work — it is the official forward path, but bleeding-edge for the
data foundation of a SaaS that has no migrations yet.

## 🇪🇸 Contexto
Durante la Fase 1a (capa de datos), `pnpm add prisma @prisma/client` resolvió a
**Prisma 7.8.0** (la última), no la v6 que el plan asumía. Prisma 7 trae dos cambios
incompatibles que impactan el diseño:

1. `url = env("DATABASE_URL")` **ya no se permite en `schema.prisma`**. La conexión
   pasa a un `prisma.config.ts` y a un **driver adapter** (`@prisma/adapter-pg` +
   `pg`) que se pasa al constructor de `PrismaClient`.
2. Los enums en una sola línea ya no parsean (cosmético, fácil de arreglar).

El cambio (1) es el de fondo: altera cómo se construye `PrismaService` y suma dos
dependencias de runtime más un archivo de config. Prisma 7 salió semanas antes de
este trabajo — es el camino oficial a futuro, pero bleeding-edge para los cimientos
de datos de un SaaS que todavía no tiene migraciones.

## 🇬🇧 Options / 🇪🇸 Opciones
- **A) Pin to Prisma 6 (recommended).** `prisma@^6` + `@prisma/client@^6`. The
  classic model works as planned: `url` in the schema, `PrismaService extends
  PrismaClient`, no adapter, no config file. Stable, heavily documented with NestJS.
  Cost: a future v6→v7 migration (cheap while greenfield, no data). / **Pinear a
  Prisma 6 (recomendada).** El modelo clásico funciona como estaba planeado: `url` en
  el schema, `PrismaService extends PrismaClient`, sin adapter ni config. Estable y
  muy documentado con NestJS. Coste: una futura migración v6→v7 (barata en greenfield,
  sin datos).
- **B) Adopt Prisma 7 now.** Stay on latest: `prisma.config.ts`, driver adapter
  (`@prisma/adapter-pg` + `pg`), `PrismaService` built with the adapter, multi-line
  enums. The forward model, but bleeding-edge in the project's foundation with fewer
  NestJS examples. / **Adoptar Prisma 7 ahora.** Quedarse en la última:
  `prisma.config.ts`, driver adapter, `PrismaService` con el adapter, enums
  multilínea. El modelo a futuro, pero bleeding-edge en los cimientos y con menos
  ejemplos NestJS.

## 🇬🇧 Decision / 🇪🇸 Decisión
**Option A.** Pin both packages to `^6` (`>=6.x <7.0.0`, which also blocks an
accidental jump to v7) and keep the classic `PrismaService extends PrismaClient` with
`url` in the schema. We favour the simplest sufficient design for Phase 1 over
adopting a weeks-old major in the data layer; the v7 adapter model adds moving parts
with no benefit for auth/tenancy/RBAC. The v6→v7 upgrade is **declared technical
debt**, tracked below. /
**Opción A.** Pinear ambos paquetes a `^6` (`>=6.x <7.0.0`, que además bloquea un
salto accidental a v7) y mantener el `PrismaService extends PrismaClient` clásico con
`url` en el schema. Preferimos el menor diseño suficiente para la Fase 1 antes que
adoptar un major de semanas en la capa de datos; el modelo de adapter de v7 suma
piezas móviles sin beneficio para auth/tenancy/RBAC. El salto v6→v7 es **deuda
técnica declarada**, registrada abajo.

## 🇬🇧 Consequences / 🇪🇸 Consecuencias
- `apps/api/package.json` pins `prisma@^6` and `@prisma/client@^6`; build scripts
  (`prisma`, `@prisma/engines`, `@prisma/client`) approved in `pnpm-workspace.yaml`
  `allowBuilds`. / `apps/api/package.json` pinea `prisma@^6` y `@prisma/client@^6`;
  build scripts aprobados en `allowBuilds`.
- **Declared debt — v6→v7 migration:** move `DATABASE_URL` to `prisma.config.ts`,
  add `@prisma/adapter-pg` + `pg`, rebuild `PrismaService` with the adapter, and
  migrate the `package.json#prisma.seed` block to the config file (it already warns
  as deprecated under v6). Do it deliberately, not by an unpinned `pnpm update`. /
  **Deuda declarada — migración v6→v7:** mover `DATABASE_URL` a `prisma.config.ts`,
  agregar `@prisma/adapter-pg` + `pg`, reconstruir `PrismaService` con el adapter, y
  migrar el bloque `package.json#prisma.seed` al archivo de config (ya avisa como
  deprecado en v6). Hacerlo deliberadamente, no por un `pnpm update` sin pin.
- The `package.json#prisma.seed` deprecation warning under v6 is expected and
  harmless until the migration. / El warning de deprecación de `package.json#prisma.seed`
  en v6 es esperado e inofensivo hasta la migración.
