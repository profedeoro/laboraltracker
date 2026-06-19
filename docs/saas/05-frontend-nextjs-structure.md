# LaboralTracker SaaS — Estructura del frontend (Next.js)

> Documento **5** del set de arquitectura. Es el dashboard web que consume la API
> de los docs [02](02-api-and-security.md) (endpoints, auth) y respeta la
> visibilidad por rol de los docs [00](00-architecture-overview.md) §2 y
> [03](03-privacy-policy.md) §4. Cubre los puntos 14-17 del brief.
>
> **Stack:** Next.js (App Router) + React + TypeScript + Tailwind + React Hook Form
> + Zod + Zustand (estado acotado). **Alcance:** diseño de organización, no código.

---

## 1. Principios de organización

1. **Modular por feature**, no por tipo técnico. Lo que cambia junto, vive junto.
2. **Container / Presentational**: componentes "tontos" (UI pura, reciben props)
   separados de "contenedores" (traen datos, orquestan). El presentacional se prueba
   y reusa sin saber de la API.
3. **El backend manda en seguridad**: la UI **oculta** lo que el rol no puede usar
   (UX), pero **nunca es la frontera real** — esa son los guards (doc 02/04). Ocultar
   un botón ≠ proteger un endpoint.
4. **Contrato tipado de punta a punta**: los tipos de request/response se comparten
   o se generan; Zod valida en el borde (igual filosofía que el agente, que genera
   TS desde Rust).

---

## 2. Estructura de carpetas (App Router)

```txt
web/
├── src/
│   ├── app/                          # rutas (App Router)
│   │   ├── (auth)/                    # grupo público
│   │   │   ├── login/page.tsx
│   │   │   └── layout.tsx
│   │   ├── (dashboard)/               # grupo protegido (layout valida sesión)
│   │   │   ├── layout.tsx             # sidebar + guard de sesión + empresa activa
│   │   │   ├── page.tsx               # dashboard según rol
│   │   │   ├── projects/
│   │   │   ├── tasks/
│   │   │   ├── time/                  # reportes de tiempo
│   │   │   ├── activity/
│   │   │   ├── screenshots/
│   │   │   ├── team/
│   │   │   ├── members/
│   │   │   ├── devices/
│   │   │   ├── reports/
│   │   │   └── settings/              # incl. tracking-settings (admin)
│   │   └── api/                       # route handlers (proxy de cookies si hace falta)
│   │
│   ├── features/                      # LÓGICA por dominio (la carne)
│   │   ├── auth/
│   │   │   ├── components/            # LoginForm (presentational)
│   │   │   ├── hooks/                 # useLogin, useRefresh
│   │   │   └── api/                   # llamadas a /auth/*
│   │   ├── projects/
│   │   │   ├── components/            # ProjectList, ProjectForm
│   │   │   ├── hooks/                 # useProjects, useCreateProject
│   │   │   ├── api/                   # GET/POST /projects
│   │   │   └── schemas/              # Zod: createProjectSchema
│   │   ├── time/ · activity/ · screenshots/ · reports/ · devices/ · members/ ...
│   │
│   ├── components/                    # UI compartida (presentational/atomic)
│   │   ├── ui/                        # Button, Input, Table, Modal, Badge...
│   │   └── layout/                    # Sidebar, Topbar, CompanySwitcher
│   │
│   ├── lib/
│   │   ├── http/                      # cliente HTTP centralizado (ver §5)
│   │   ├── auth/                      # helpers de sesión/rol
│   │   └── format/                    # formateo de tiempo (zona = presentación)
│   │
│   ├── stores/                        # estado global ACOTADO (Zustand)
│   │   ├── auth.store.ts              # user, empresa activa, rol, permisos
│   │   └── ui.store.ts               # sidebar, theme, toasts
│   │
│   └── types/                         # tipos del contrato API (DTOs)
└── .env.example
```

---

## 3. Estado global: acotado, no un cajón de sastre

Zustand con **solo lo verdaderamente global**:

- **`auth.store`**: `user` (identidad global), **empresa activa** (`companyId`),
  `roleKey` y la **lista de permisos** del rol → la UI decide qué mostrar.
- **`ui.store`**: sidebar, theme, toasts.

Todo lo demás (datos de servidor: proyectos, tiempo, reportes) **no va al store
global**: va en hooks de fetching por feature (React Query/SWR o `fetch` en Server
Components), con su caché. Meter datos de servidor en Zustand es el error clásico
que genera estado duplicado y desincronizado.

**Empresa activa** (clave por ser multi-empresa, doc 02 §4): el `CompanySwitcher`
del topbar cambia el `companyId` activo → dispara `POST /auth/switch-company` y
refresca el contexto. Todo lo que se ve cuelga de esa empresa activa.

---

## 4. UI por rol (refleja el RBAC, no lo reemplaza)

La navegación y los controles se **renderizan según permisos** (doc 00 §2, doc 03
§4):

```tsx
// helper de permisos (lee del auth.store)
const { can } = usePermissions();

{can('report.view') && <NavItem href="/reports">Reportes</NavItem>}
{can('member.manage') && <NavItem href="/members">Personas</NavItem>}
```

- **Employee** ve su tiempo/tareas/equipos propios; no ve "Personas" ni capturas
  ajenas.
- **Client Viewer** ve solo reportes; el resto del menú no existe para él.
- **Recordatorio**: esto es **UX**. Si alguien fuerza la URL, el **backend** lo
  frena (404/403). La UI nunca es la última línea de defensa.

---

## 5. Cliente HTTP: token, refresh y errores en un solo lugar

`lib/http` centraliza **todas** las llamadas. Responsabilidades:

1. **Adjunta el access token** (de memoria) en cada request.
2. **Refresh transparente**: ante `401`, intenta `POST /auth/refresh` (con la cookie
   httpOnly del refresh), reintenta **una vez**, y si falla → logout. La rotación
   del refresh la maneja el backend (doc 02 §4); el front solo reacciona.
3. **Mapea el error estándar** (doc 02 §7) a algo accionable (toast / errores de
   formulario), sin tragar el error.
4. **Concurrencia de refresh**: si varias requests caen en 401 a la vez, una sola
   dispara el refresh y las demás esperan (evita tormenta de refresh).

> Ningún componente llama `fetch` crudo. Centralizar acá es lo que hace manejable
> el token/refresh; repartirlo sería un dolor de cabeza garantizado.

---

## 6. Formularios: React Hook Form + Zod

- **Un schema Zod por formulario**, reusado para validar en el cliente.
- El mismo contrato que valida el backend (DTO) → la validación de UX y la de
  integridad **coinciden** (pero el backend revalida siempre, doc 02 §6).

```tsx
const createProjectSchema = z.object({
  name: z.string().trim().min(1, 'El nombre es obligatorio'),
  color: z.string().optional(),
});

const form = useForm({ resolver: zodResolver(createProjectSchema) });
```

Errores de campo del backend (p. ej. `NAME_TAKEN` en `details`) se mapean al campo
correspondiente del formulario, no a un toast genérico.

---

## 7. Tiempo en la UI (zona = presentación)

Coherente con la política del agente (doc 01 §0.2): el backend entrega
**epoch-millis UTC**; el front **convierte a la zona del usuario solo para mostrar**.
Los totales/duración se calculan desde los millis. Un helper en `lib/format`
concentra esto (nada de `new Date()` disperso con zonas implícitas).

---

## 8. Server vs Client Components (App Router)

- **Server Components** para listados/lecturas iniciales (traen datos en el server,
  menos JS al cliente, mejor primer render).
- **Client Components** donde hay interacción/estado (formularios, switcher, timers
  en vivo, toasts).
- El layout protegido `(dashboard)/layout.tsx` valida sesión antes de renderizar;
  sin sesión → redirect a `/login`.

---

## 9. Testing del frontend

- **Componentes presentacionales**: render + interacción (Testing Library) — son
  puros, fáciles de probar.
- **Hooks de feature**: lógica de fetching/estado con mocks del cliente HTTP.
- **E2E** (Playwright): login, crear proyecto, ver reporte, y **que un rol no vea lo
  que no debe** (espejo en UI del test de aislamiento del backend).

---

## Próximo documento

**`06-roadmap.md`** — el cierre del set: convierte todo este análisis en **fases de
construcción** priorizadas (qué se hace primero y por qué), de donde salen los
planes incrementales (spec → plan → slices, la misma disciplina del agente).
Incluye la decisión de **estructura de repos** (monorepo vs separados) que quedó
pendiente desde el doc 00. *(brief §21)*
