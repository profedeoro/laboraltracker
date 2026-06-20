# Fase 0 — Monorepo + scaffold de plataforma — Diseño

> Spec de la **Fase 0** del roadmap SaaS ([docs/saas/06-roadmap.md](../../saas/06-roadmap.md)).
> Convierte el repo del agente en un **monorepo** que aloja las dos mitades y deja
> la plataforma lista para construir (Fase 1+). Decisión de repo: monorepo
> ([06-roadmap §1](../../saas/06-roadmap.md)).

## Objetivo

Reestructurar el repositorio a monorepo y scaffoldear los esqueletos de la
plataforma (api, web, contracts) + Postgres de desarrollo + CI, **sin alterar el
comportamiento del agente** ya construido (Plans 1-4).

**Criterio de éxito global:** el agente compila y arranca exactamente como hoy
desde su nueva ubicación, y los esqueletos de plataforma bootean.

## No-objetivos (YAGNI — Fase 1+)

- Schema Prisma, migraciones, modelos de datos.
- Autenticación, guards, endpoints reales.
- Los DTOs reales de `@lt/contracts` (llegan en Fase 3, el sync).
- Docker de producción / pipeline de despliegue.
- Cualquier feature de negocio.

## Decisiones tomadas

- **Estructura:** monorepo (este repo). Razón decisiva: contrato de sync compartido
  entre las dos mitades.
- **Gestor de paquetes (plataforma):** **pnpm workspaces** para `apps/api`,
  `apps/web`, `packages/*`. El agente queda **standalone con npm** (no entra al
  workspace pnpm); conviven aislados.
- **Corte en dos slices:** 0a reestructura (riesgo aislado) + 0b scaffold (aditivo).

## Estructura objetivo

```txt
laboraltracker/
├── apps/
│   ├── agent/              # agente actual movido como bloque (Cargo + npm standalone)
│   │   ├── src/            # frontend Svelte
│   │   ├── src-tauri/      # núcleo Rust
│   │   ├── static/
│   │   ├── svelte.config.js · vite.config.js · tsconfig.json
│   │   └── package.json · package-lock.json   # npm propio del agente
│   ├── api/               # NestJS — esqueleto que bootea (GET /health), sin Prisma
│   └── web/               # Next.js (App Router) — esqueleto por defecto
├── packages/
│   └── contracts/         # @lt/contracts — placeholder (se llena en Fase 3)
├── docs/                  # se queda
├── docker-compose.yml     # Postgres 16 para dev
├── pnpm-workspace.yaml    # apps/api, apps/web, packages/*   (NO apps/agent)
├── package.json           # raíz: workspace pnpm + scripts
├── .env.example           # DATABASE_URL, etc.
├── .gitignore             # actualizado (node_modules por app, .svelte-kit, target/)
└── CLAUDE.md · CHANGELOG.md · README.md       # se quedan en raíz
```

## Slice 0a — Reestructura a monorepo

**Qué hace:** mueve el agente a `apps/agent/` y crea el esqueleto del workspace
pnpm en la raíz. **Cero cambios de contenido del agente, solo ubicación.**

**Pasos:**
1. `git mv` (preservando historia) del agente a `apps/agent/`: `src/`, `src-tauri/`,
   `static/`, `svelte.config.js`, `vite.config.js`, `tsconfig.json`, `package.json`,
   `package-lock.json`. **Todo junto** para preservar paths relativos.
2. Crear `pnpm-workspace.yaml` (apps/api, apps/web, packages/*) y `package.json`
   raíz (workspace vacío, scripts de conveniencia).
3. Actualizar `.gitignore` raíz: `**/node_modules`, `apps/agent/.svelte-kit`,
   `apps/agent/build`, `apps/*/target`/`src-tauri/target`.
4. Limpiar artefactos viejos de la raíz (`node_modules`, `.svelte-kit` raíz se
   regeneran en `apps/agent`).

**Por qué es de bajo riesgo:** `frontendDist: "../build"` (relativo a
`apps/agent/src-tauri`) y `export_to "../../src/lib/bindings/"` (de `src-tauri` a
`src/lib/bindings`) **siguen válidos** porque `src/` y `src-tauri/` mantienen su
relación relativa al moverse juntos.

**Criterio de éxito (verificación dura):**
- `cd apps/agent && cargo test` → verde (todos los tests del agente).
- `cd apps/agent && npm install && cargo tauri dev` → la app arranca igual que hoy.
- Regenerar bindings (corre `cargo test`) deja `git status` **limpio** (sin churn
  CRLF — ya cubierto por `.gitattributes` + `core.autocrlf false`).

Si el agente no arranca idéntico, el slice **no pasa**.

## Slice 0b — Scaffold de plataforma

**Qué hace:** agrega los esqueletos de plataforma. **Aditivo: no toca `apps/agent`.**

**Pasos:**
1. `packages/contracts`: `package.json` (`@lt/contracts`, privado), `tsconfig.json`,
   `src/index.ts` con un placeholder exportable (sin DTOs reales aún).
2. `apps/api`: NestJS mínimo — `main.ts`, `app.module.ts`, y un `GET /health` que
   responde `{ status: 'ok' }`. Depende de `@lt/contracts` vía `workspace:*` (aunque
   no lo use todavía, valida el cableado del workspace). Sin Prisma, sin auth.
3. `apps/web`: Next.js (App Router) por defecto que renderiza una página.
4. `docker-compose.yml`: servicio `postgres:16` (puerto, volumen, healthcheck) para
   dev. `.env.example` con `DATABASE_URL`.
5. CI (GitHub Actions, repo `profedeoro/laboraltracker`) con **path filters**:
   - job **agent**: corre `cargo test` solo si cambió `apps/agent/**`.
   - job **platform**: `pnpm install` + build/lint solo si cambió
     `apps/api/**`, `apps/web/**` o `packages/**`.

**Criterio de éxito (verificación):**
- `pnpm install` en la raíz resuelve el workspace sin error.
- `pnpm --filter @lt/api start` (o equivalente) → `GET /health` responde 200.
- `pnpm --filter web dev` → la página renderiza.
- `docker compose up -d postgres` → contenedor `healthy`.
- CI verde en un PR de prueba.

## Prerrequisitos de tooling

- **Node + pnpm** (vía `corepack enable`).
- **Docker** (para Postgres de dev).
- Rust/Cargo ya está (agente). El plan verifica presencia antes de scaffoldear; si
  falta algo, se resuelve ahí.

## Riesgos y mitigaciones

| Riesgo | Mitigación |
|--------|-----------|
| El move rompe paths del agente | Mover `src/`+`src-tauri/` juntos; verificación dura antes de commitear 0a |
| Churn CRLF en bindings al regenerar | Ya cubierto (`.gitattributes` + `core.autocrlf false`); se verifica `git status` limpio |
| Mezcla de toolchains (Cargo + pnpm) | Aisladas por carpeta; agente fuera del workspace pnpm |
| `git mv` pierde historia | Usar `git mv` (no borrar+crear); verificar `git log --follow` |

## Entregables

- **0a:** repo reestructurado, agente en `apps/agent`, workspace pnpm vacío,
  `.gitignore` actualizado, agente verificado. Entrada en CHANGELOG.
- **0b:** `apps/api` + `apps/web` + `packages/contracts` booteando, Postgres dev,
  CI con path filters. Entrada en CHANGELOG.

Cada slice es su propio PR (ramas `feature/…`), con commits convencionales.
