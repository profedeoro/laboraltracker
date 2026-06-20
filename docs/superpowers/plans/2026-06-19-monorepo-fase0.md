# Fase 0 — Monorepo + scaffold de plataforma — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reestructurar el repo a monorepo (agente en `apps/agent`) y scaffoldear los esqueletos de plataforma (api/web/contracts) + Postgres de dev + CI, sin alterar el comportamiento del agente.

**Architecture:** Dos slices secuenciales, cada uno su propia rama y PR. **0a** mueve el agente como bloque (paths relativos intactos) y crea el workspace pnpm vacío. **0b** agrega, de forma aditiva, los esqueletos de plataforma. El agente queda standalone con npm, fuera del workspace pnpm.

**Tech Stack:** pnpm workspaces · NestJS · Next.js (App Router) · Docker (Postgres 16) · GitHub Actions. Agente: Tauri 2 + Rust + Svelte (sin cambios de contenido).

**Spec:** [docs/superpowers/specs/2026-06-19-monorepo-fase0-design.md](../specs/2026-06-19-monorepo-fase0-design.md)

---

## Entorno de ejecución (LEER antes de empezar)

- **OS:** Windows. Shell primaria PowerShell; también está disponible la herramienta Bash (git bash, sintaxis POSIX).
- **Cargo en PowerShell:** anteponer `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"` antes de cualquier `cargo`. En Bash: `export PATH="$HOME/.cargo/bin:$PATH"`.
- **pnpm:** habilitar vía `corepack enable` (incluye pnpm). Verificar con `pnpm --version`.
- **Docker:** requerido para Postgres de dev.
- **Commits:** conventional commits, **sin** trailer `Co-Authored-By` ni atribución de IA (regla del repo).
- **EOL:** `.gitattributes` + `core.autocrlf false` ya configurados; los bindings TS regenerados no deben ensuciar `git status`.

---

## File Structure

**Slice 0a — reestructura:**
- Mover (git mv): `src/`, `src-tauri/`, `static/`, `svelte.config.js`, `vite.config.js`, `tsconfig.json`, `package.json`, `package-lock.json` → `apps/agent/`.
- Crear: `pnpm-workspace.yaml`, `package.json` (raíz, workspace).
- Modificar: `.gitignore` (patrones no anclados a raíz).

**Slice 0b — scaffold (aditivo, no toca `apps/agent`):**
- Crear: `packages/contracts/{package.json,tsconfig.json,src/index.ts}`.
- Crear: `apps/api/` (NestJS mínimo con `GET /health` + test).
- Crear: `apps/web/` (Next.js App Router por defecto).
- Crear: `docker-compose.yml`, `.env.example`.
- Crear: `.github/workflows/ci.yml` (path filters).

---

# SLICE 0a — Reestructura a monorepo

> Rama propia. Al terminar: PR + merge antes de empezar 0b.

### Task 0a.1: Crear rama y mover el agente a `apps/agent/`

**Files:**
- Move: todo el agente a `apps/agent/`

- [ ] **Step 1: Crear rama desde master actualizado**

```bash
git checkout master
git pull
git checkout -b feature/monorepo-0a
```

- [ ] **Step 2: Crear el directorio destino y mover con `git mv` (preserva historia)**

```bash
mkdir -p apps/agent
git mv src apps/agent/src
git mv src-tauri apps/agent/src-tauri
git mv static apps/agent/static
git mv svelte.config.js apps/agent/svelte.config.js
git mv vite.config.js apps/agent/vite.config.js
git mv tsconfig.json apps/agent/tsconfig.json
git mv package.json apps/agent/package.json
git mv package-lock.json apps/agent/package-lock.json
```

- [ ] **Step 3: Verificar que el árbol movido quedó correcto**

```bash
git status
ls apps/agent
```

Expected: `apps/agent/` contiene `src/`, `src-tauri/`, `static/`, los 3 config y los 2 `package*.json`. `git status` muestra renames (R) de cada path, sin borrados sueltos.

- [ ] **Step 4: Confirmar que los paths internos del agente NO requieren cambios**

```bash
grep -n "frontendDist" apps/agent/src-tauri/tauri.conf.json
grep -rn "export_to" apps/agent/src-tauri/src
```

Expected: `frontendDist: "../build"` (sigue válido: relativo a `apps/agent/src-tauri` → `apps/agent/build`). `export_to = "../../src/lib/bindings/"` (sigue válido: de `apps/agent/src-tauri` → `apps/agent/src/lib/bindings`). **No editar nada**: son correctos por ser relativos.

### Task 0a.2: Workspace pnpm raíz + `.gitignore`

**Files:**
- Create: `pnpm-workspace.yaml`
- Create: `package.json` (raíz)
- Modify: `.gitignore`

- [ ] **Step 1: Crear `pnpm-workspace.yaml` (el agente queda fuera a propósito)**

```yaml
packages:
  - apps/api
  - apps/web
  - packages/*
```

- [ ] **Step 2: Crear `package.json` raíz (workspace privado)**

```json
{
  "name": "laboraltracker-workspace",
  "private": true,
  "version": "0.1.0",
  "description": "LaboralTracker monorepo: desktop agent (apps/agent) + SaaS platform (apps/api, apps/web)."
}
```

- [ ] **Step 3: Reemplazar `.gitignore` con patrones no anclados a raíz**

Razón: los patrones `/build` y `/.svelte-kit` estaban anclados a la raíz y ya no matchean en `apps/agent/`. Contenido completo:

```gitignore
.DS_Store
node_modules/

# Build outputs (cualquier nivel)
build/
dist/
.svelte-kit/
.next/
package/
target/

# Env
.env
.env.*
!.env.example

vite.config.js.timestamp-*
vite.config.ts.timestamp-*

# SQLite local DB (lives in app data dir; safety net)
*.db
*.db-wal
*.db-shm

# Local AI agent / tooling state (not part of the project)
.claude/
```

- [ ] **Step 4: Verificar que git no quiere trackear artefactos**

```bash
git add -A
git status --short
```

Expected: aparecen los renames del agente, `pnpm-workspace.yaml`, `package.json`, `.gitignore` modificado. **No** aparecen `node_modules/`, `target/`, `build/`, `.svelte-kit/`.

### Task 0a.3: Verificación dura del agente + commit/PR

**Files:** ninguno nuevo (verificación + CHANGELOG)

- [ ] **Step 1: Instalar deps del agente y correr los tests Rust**

PowerShell:
```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cd apps/agent
npm install
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: todos los tests del agente en **verde** (los mismos que pasaban antes del move).

- [ ] **Step 2: Verificar que la app arranca desde la nueva ubicación**

```powershell
cargo tauri dev
```

Expected: la app Tauri abre la ventana como antes. Cerrar tras confirmar. (Desde `apps/agent`, el `tauri` script y los paths relativos funcionan.)

- [ ] **Step 3: Verificar que regenerar bindings deja el árbol limpio**

```bash
cd ../..
git status --short
```

Expected: tras `cargo test` (que regenera los `.ts` de `ts-rs`), `git status` **no** muestra cambios en `apps/agent/src/lib/bindings/` (sin churn CRLF). Si aparecen, detener y revisar `.gitattributes`.

- [ ] **Step 4: Actualizar CHANGELOG**

Añadir bajo `## [Unreleased] → Added` en [CHANGELOG.md](../../../CHANGELOG.md):

```markdown
- **Monorepo restructure** (Phase 0a): the desktop agent moved to `apps/agent/`
  (relative paths preserved — builds and runs identically); empty pnpm workspace
  skeleton at the root (`pnpm-workspace.yaml` + root `package.json`); `.gitignore`
  switched to non-root-anchored patterns. /
  **Reestructura a monorepo** (Fase 0a): el agente de escritorio movido a
  `apps/agent/` (paths relativos preservados — compila y arranca idéntico);
  esqueleto del workspace pnpm en la raíz; `.gitignore` con patrones no anclados.
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor(repo): move agent to apps/agent; add empty pnpm workspace (Phase 0a)"
```

- [ ] **Step 6: Push y PR**

```bash
git push -u origin feature/monorepo-0a
gh pr create --title "refactor(repo): monorepo restructure — agent to apps/agent (Phase 0a)" --body "Moves the desktop agent to apps/agent/ preserving relative paths; adds empty pnpm workspace. Agent verified: cargo test green, tauri dev boots, bindings regen clean."
```

> **Gate:** mergear este PR a `master` antes de empezar 0b. Usar superpowers:finishing-a-development-branch.

---

# SLICE 0b — Scaffold de plataforma

> Rama propia desde `master` ya con 0a mergeado. Aditivo: no toca `apps/agent`.

### Task 0b.1: Paquete `packages/contracts`

**Files:**
- Create: `packages/contracts/package.json`
- Create: `packages/contracts/tsconfig.json`
- Create: `packages/contracts/src/index.ts`

- [ ] **Step 1: Crear rama**

```bash
git checkout master && git pull && git checkout -b feature/monorepo-0b
```

- [ ] **Step 2: `packages/contracts/package.json`**

```json
{
  "name": "@lt/contracts",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "main": "./src/index.ts",
  "types": "./src/index.ts",
  "exports": { ".": "./src/index.ts" }
}
```

- [ ] **Step 3: `packages/contracts/tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "declaration": true,
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true
  },
  "include": ["src"]
}
```

- [ ] **Step 4: `packages/contracts/src/index.ts` (placeholder, sin DTOs reales)**

```ts
// Shared agent↔platform sync contracts.
// Real DTOs (sync time-entries/activity/screenshots) land in Phase 3.
export const CONTRACTS_VERSION = "0.0.0";
```

### Task 0b.2: `apps/api` — NestJS con `GET /health`

**Files:**
- Create: `apps/api/` (vía generador) + ajustar a `GET /health` con test

- [ ] **Step 1: Generar el esqueleto NestJS (no interactivo, sin git/install)**

```bash
pnpm dlx @nestjs/cli new api --directory apps/api --package-manager pnpm --skip-git --skip-install --strict
```

Expected: crea `apps/api/` con `src/main.ts`, `src/app.module.ts`, `src/app.controller.ts`, `src/app.service.ts`, `src/app.controller.spec.ts`, `package.json`, `tsconfig*.json`, `nest-cli.json`.

- [ ] **Step 2: Declarar la dependencia del workspace en `apps/api/package.json`**

Añadir en `dependencies`:

```json
"@lt/contracts": "workspace:*"
```

- [ ] **Step 3: Reemplazar `apps/api/src/app.controller.ts` por un endpoint `GET /health`**

```ts
import { Controller, Get } from '@nestjs/common';

@Controller()
export class AppController {
  @Get('health')
  health(): { status: string } {
    return { status: 'ok' };
  }
}
```

- [ ] **Step 4: Simplificar `apps/api/src/app.module.ts` (quitar AppService no usado)**

```ts
import { Module } from '@nestjs/common';
import { AppController } from './app.controller';

@Module({
  imports: [],
  controllers: [AppController],
  providers: [],
})
export class AppModule {}
```

Borrar `apps/api/src/app.service.ts`.

- [ ] **Step 5: Reescribir el test `apps/api/src/app.controller.spec.ts`**

```ts
import { Test, TestingModule } from '@nestjs/testing';
import { AppController } from './app.controller';

describe('AppController', () => {
  let controller: AppController;

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      controllers: [AppController],
    }).compile();
    controller = module.get<AppController>(AppController);
  });

  it('GET /health returns ok', () => {
    expect(controller.health()).toEqual({ status: 'ok' });
  });
});
```

- [ ] **Step 6: Instalar el workspace y correr el test**

```bash
cd ../..        # raíz del repo
pnpm install
pnpm --filter api test
```

Expected: `pnpm install` resuelve `@lt/contracts` vía `workspace:*`. El test `GET /health returns ok` pasa.

- [ ] **Step 7: Verificar boot del servidor**

```bash
pnpm --filter api start
```

Expected: Nest arranca; `curl http://localhost:3000/health` → `{"status":"ok"}`. Cortar tras confirmar.

### Task 0b.3: `apps/web` — Next.js App Router

**Files:**
- Create: `apps/web/` (vía generador)

- [ ] **Step 1: Generar el esqueleto Next.js (no interactivo)**

```bash
pnpm dlx create-next-app@latest apps/web --ts --app --eslint --tailwind --no-src-dir --import-alias "@/*" --use-pnpm --skip-install --disable-git
```

Expected: crea `apps/web/` con `app/`, `package.json`, `tsconfig.json`, config de Tailwind/ESLint.

- [ ] **Step 2: Instalar el workspace y verificar render**

```bash
cd ../.. && pnpm install
pnpm --filter web dev
```

Expected: Next arranca; `http://localhost:3000` (o el puerto que indique) renderiza la página por defecto. Cortar tras confirmar.

> Nota: api y web usan ambos 3000 por defecto. Para dev simultáneo, fijar el puerto de `web` (p. ej. `next dev -p 3001`) en su script `dev` de `apps/web/package.json`.

- [ ] **Step 3: Fijar el puerto de web a 3001 en `apps/web/package.json`**

Cambiar el script `dev` a:

```json
"dev": "next dev -p 3001"
```

### Task 0b.4: Postgres de dev + `.env.example`

**Files:**
- Create: `docker-compose.yml`
- Create: `.env.example`

- [ ] **Step 1: `docker-compose.yml` (Postgres 16 con healthcheck)**

```yaml
services:
  postgres:
    image: postgres:16
    container_name: laboraltracker-db
    restart: unless-stopped
    environment:
      POSTGRES_USER: laboraltracker
      POSTGRES_PASSWORD: laboraltracker
      POSTGRES_DB: laboraltracker
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U laboraltracker"]
      interval: 5s
      timeout: 5s
      retries: 5

volumes:
  pgdata:
```

- [ ] **Step 2: `.env.example`**

```dotenv
# Plataforma SaaS (apps/api). Copiar a .env y ajustar.
DATABASE_URL="postgresql://laboraltracker:laboraltracker@localhost:5432/laboraltracker?schema=public"
```

- [ ] **Step 3: Verificar que Postgres levanta sano**

```bash
docker compose up -d postgres
docker compose ps
```

Expected: el servicio `postgres` aparece `healthy` (esperar unos segundos). Luego `docker compose down` para limpiar.

### Task 0b.5: CI con path filters

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: `.github/workflows/ci.yml`**

```yaml
name: CI

on:
  push:
    branches: [master]
  pull_request:

jobs:
  changes:
    runs-on: ubuntu-latest
    outputs:
      agent: ${{ steps.filter.outputs.agent }}
      platform: ${{ steps.filter.outputs.platform }}
    steps:
      - uses: actions/checkout@v4
      - uses: dorny/paths-filter@v3
        id: filter
        with:
          filters: |
            agent:
              - 'apps/agent/**'
            platform:
              - 'apps/api/**'
              - 'apps/web/**'
              - 'packages/**'
              - 'pnpm-workspace.yaml'
              - 'package.json'

  agent:
    needs: changes
    if: ${{ needs.changes.outputs.agent == 'true' }}
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install Tauri system deps
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
      - name: cargo test
        run: cargo test --manifest-path apps/agent/src-tauri/Cargo.toml

  platform:
    needs: changes
    if: ${{ needs.changes.outputs.platform == 'true' }}
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm
      - run: pnpm install --frozen-lockfile
      - run: pnpm --filter api test
      - run: pnpm --filter api build
      - run: pnpm --filter web build
```

- [ ] **Step 2: Validar el YAML localmente**

```bash
cat .github/workflows/ci.yml
```

Expected: YAML bien formado (indentación consistente). La validación real corre al abrir el PR.

### Task 0b.6: Verificación integral + commit/PR

- [ ] **Step 1: Confirmar que el lockfile y el workspace están sanos**

```bash
pnpm install
git status --short
```

Expected: existe `pnpm-lock.yaml` en la raíz; no aparecen `node_modules/`, `.next/`, `dist/`.

- [ ] **Step 2: Correr la verificación de plataforma de punta a punta**

```bash
pnpm --filter api test
pnpm --filter api build
pnpm --filter web build
```

Expected: test verde; ambos builds exitosos.

- [ ] **Step 3: Actualizar CHANGELOG**

Añadir bajo `## [Unreleased] → Added`:

```markdown
- **Platform scaffold** (Phase 0b): `packages/contracts` (`@lt/contracts`
  placeholder), `apps/api` (NestJS with `GET /health` + test), `apps/web`
  (Next.js App Router, dev port 3001), `docker-compose.yml` (Postgres 16 dev),
  `.env.example`, and path-filtered GitHub Actions CI (Rust job vs platform job). /
  **Scaffold de plataforma** (Fase 0b): `packages/contracts`, `apps/api` (NestJS
  con `GET /health` + test), `apps/web` (Next.js), Postgres de dev, `.env.example`
  y CI con path filters.
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(platform): scaffold api, web, contracts + dev Postgres + CI (Phase 0b)"
```

- [ ] **Step 5: Push y PR**

```bash
git push -u origin feature/monorepo-0b
gh pr create --title "feat(platform): scaffold api/web/contracts + Postgres + CI (Phase 0b)" --body "Adds platform skeletons: @lt/contracts, NestJS api (GET /health + test), Next.js web, dev Postgres compose, and path-filtered CI. Additive — does not touch apps/agent."
```

> Al terminar: superpowers:finishing-a-development-branch.

---

## Verificación final de la Fase 0 (tras mergear 0a y 0b)

- [ ] El agente compila y arranca desde `apps/agent` (`cargo test`, `cargo tauri dev`).
- [ ] `pnpm install` en la raíz resuelve el workspace.
- [ ] `apps/api` responde `GET /health` → `{"status":"ok"}`.
- [ ] `apps/web` renderiza.
- [ ] `docker compose up -d postgres` queda `healthy`.
- [ ] CI verde en ambos PRs.
