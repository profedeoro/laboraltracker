# LaboralTracker SaaS — Visión, roles, módulos y arquitectura

> Documento **fundacional** de la plataforma (las dos mitades). Cubre los puntos
> 1-4 del brief de arquitectura. Los siguientes documentos (modelo de datos, API,
> seguridad, estructura backend/frontend, roadmap) se construyen sobre éste.
>
> **Regla del producto:** inspirado en Time Doctor, **sin** copiar código, marca,
> diseño visual ni elementos propietarios. Construido desde cero con buenas
> prácticas.

---

## 0. Decisión de arquitectura: DOS MITADES

LaboralTracker no es una sola app: son **dos componentes que se necesitan** e
integran vía **API REST**.

```txt
┌─────────────────────────────┐         ┌──────────────────────────────────────┐
│  AGENTE DE ESCRITORIO        │  REST   │  PLATAFORMA WEB SaaS                  │
│  (Tauri 2 + Rust + Svelte)   │ ◄─────► │  NestJS (API) + Next.js (dashboard)  │
│  — YA CONSTRUIDO (Plans 1-4) │  JWT    │  PostgreSQL · Prisma · multi-tenant  │
│                              │  sync   │                                      │
│  • timer offline-first       │         │  • usuarios/empresas/equipos         │
│  • captura de pantalla*      │         │  • RBAC, reportes, admin, billing    │
│  • actividad teclado/mouse*  │         │  • recibe y agrega lo que el agente  │
│  • SQLite local              │         │    sincroniza                        │
└─────────────────────────────┘         └──────────────────────────────────────┘
        * Plan 5/6+ del agente
```

**Por qué dos mitades (no es capricho):** el navegador, por *sandbox* de seguridad,
**no puede** capturar la pantalla del sistema operativo ni leer el teclado/mouse
global. Eso **obliga** a un agente nativo. La web no lo reemplaza; lo administra.

**El agente ya construido encaja exacto:** el cronómetro, la sesión única, el
modelo de tiempo y los IDs fueron diseñados *desde el día 1* pensando en esta nube:

- **IDs = ULID texto** (ADR 0005): el id que genera el agente local **es** el id en
  la nube. Sin remapeo en la sincronización; sirve además como clave de
  **idempotencia** para evitar duplicados al sincronizar.
- **Tiempo = epoch-millis UTC** (02-time-policy): sin ambigüedad de zona al
  consolidar datos de agentes en distintas zonas horarias.
- **Sesión única + `is_suspect`**: datos limpios antes de subir; los reportes de la
  plataforma excluyen sesiones sospechosas.

---

## 1. Qué es una aplicación tipo Time Doctor

### Problema que resuelve
Equipos remotos/híbridos necesitan saber, **con datos y no con suposiciones**,
en qué se va el tiempo: cuánto se trabaja, en qué proyecto/tarea, con qué nivel de
actividad, y poder facturar y reportar sobre eso. Resuelve la **falta de
visibilidad** del trabajo distribuido y la **fricción del registro manual** (el
agente mide solo).

### Usuarios
- **Empresas** que gestionan equipos (el cliente que paga el SaaS).
- **Managers** que supervisan productividad y proyectos.
- **Empleados/colaboradores** cuyo tiempo se mide (corren el agente).
- **Clientes finales** de esas empresas, que ven reportes acotados (opcional).

### Módulos principales (resumen — detalle en §3)
Autenticación · Usuarios · Empresas · Equipos · Proyectos · Tareas · Registro de
tiempo · Actividad · Capturas · Reportes · Dashboard · Configuración ·
Notificaciones · Facturación · Auditoría.

### Funcionalidades mínimas (MVP del producto)
Login/roles · alta de empresa/usuarios · proyectos y tareas · **registro de tiempo
por tarea** (lo que el agente ya hace) · sincronización agente→plataforma ·
reporte básico de horas por usuario/proyecto.

### Funcionalidades avanzadas
Capturas por intervalo configurable · nivel de actividad (teclado/mouse) ·
detección de inactividad · reportes de productividad · facturación por horas ·
notificaciones · auditoría completa · integraciones (export, webhooks).

---

## 2. Roles del sistema (RBAC)

Cinco roles. La autorización combina **rol** + **tenant (empresa)** + **ownership
del recurso** — autenticar ≠ autorizar.

| Rol | Alcance | Puede | NO puede |
|-----|---------|-------|----------|
| **Super Admin** | Toda la instancia SaaS | Gestionar empresas, planes, métricas globales, soporte | Ver datos de negocio detallados de una empresa salvo soporte explícito/auditado |
| **Company Admin** | Su empresa (tenant) | Gestionar usuarios, equipos, proyectos, settings, facturación, ver todos los reportes de su empresa | Tocar datos de otra empresa; cambiar el plan global |
| **Manager** | Sus equipos/proyectos | Ver tiempo/actividad/reportes de su equipo, asignar tareas, gestionar sus proyectos | Gestionar usuarios fuera de su equipo; settings de empresa; facturación |
| **Employee** | Sí mismo | Correr el agente, ver su propio tiempo y tareas, iniciar/parar timer | Ver datos de otros; configurar empresa; ver reportes ajenos |
| **Client Viewer** | Reportes acotados (read-only) | Ver reportes/horas de los proyectos que le compartieron | Cualquier escritura; ver capturas/actividad cruda; ver otros proyectos |

**Permisos como datos** (no `if rol === 'X'` desperdigado): tabla `Permission`
+ `Role` con relación, evaluados por *guards* centralizados (ver doc de seguridad).
Esto cumple OCP: agregar un permiso/rol no obliga a tocar cada controlador.

### Módulos visibles por rol (alto nivel)
- Super Admin → panel de instancia (empresas, planes, salud).
- Company Admin → todo lo de su empresa.
- Manager → equipos/proyectos/tareas/reportes de su ámbito.
- Employee → su time tracker, sus tareas, su perfil.
- Client Viewer → solo reportes compartidos.

---

## 3. Módulos de la aplicación

Cada módulo es una unidad cohesiva (en NestJS = un módulo; en el agente, lo que
aplique). Responsabilidad y por qué existe:

| Módulo | Responsabilidad | Vive en |
|--------|-----------------|---------|
| **Auth** | Login, JWT, refresh tokens, sesiones, RBAC | Plataforma (agente consume) |
| **Users** | CRUD de usuarios, perfil, asignación a equipos | Plataforma |
| **Companies** | Tenant raíz; settings de empresa (política de capturas, etc.) | Plataforma |
| **Teams** | Agrupación de usuarios bajo managers | Plataforma |
| **Projects** | Proyectos de la empresa | Ambos (agente ya lo tiene local) |
| **Tasks** | Tareas dentro de un proyecto | Ambos (agente ya lo tiene local) |
| **Time tracking** | Sesiones de tiempo (start/stop), consolidación | Agente mide · Plataforma consolida |
| **Activity** | Nivel de actividad (teclado/mouse, %), inactividad | Agente captura · Plataforma guarda |
| **Screenshots** | Capturas por intervalo, subida, almacenamiento, privacidad | Agente captura · Plataforma guarda/sirve |
| **Reports** | Agregaciones (horas por user/proyecto/tarea, productividad) | Plataforma |
| **Dashboard** | Vistas resumen por rol | Plataforma (web) |
| **Settings** | Configuración por empresa/usuario | Plataforma |
| **Notifications** | Avisos (sesión larga, inactividad, límites) | Plataforma |
| **Billing** | Planes, facturación por horas/usuarios (si aplica) | Plataforma |
| **Audit** | Registro inmutable de acciones sensibles | Plataforma |

---

## 4. Arquitectura general del sistema

### Backend (plataforma) — NestJS
Arquitectura por capas + módulos, alineada con `~/.claude/rules/nestjs-backend.md`:
`Controller → DTO/validación → Service (lógica) → Repository/Prisma → PostgreSQL`,
con `Guards` (auth + RBAC + tenant), `Interceptors` (respuesta/audit), `Pipes`
(validación). Dominio separado de infraestructura (misma disciplina hexagonal que
el agente, adaptada a NestJS).

### Frontend (plataforma) — Next.js
Modular por features, componentes reutilizables (container/presentational), estado
global acotado (auth/rol/permisos/empresa), formularios con React Hook Form + Zod,
HttpService centralizado con manejo de token/refresh.

### Base de datos — PostgreSQL + Prisma
Relacional, **multi-tenant por `companyId`** en las entidades de negocio, con
índices por las consultas reales (reportes por usuario/fecha/proyecto), migraciones
versionadas y auditoría.

### Comunicación frontend ↔ backend
REST/JSON, JWT en header (o cookie httpOnly segura), contrato tipado (Zod en el
front; DTOs validados en el back). WebSockets solo si un caso lo justifica
(p. ej. estado de agente en vivo).

### Comunicación AGENTE ↔ plataforma (la integración clave)
- El agente autentica con un **token de dispositivo/usuario** (JWT + refresh).
- **Offline-first:** el agente mide y guarda en su SQLite local (ya implementado);
  cuando hay red, **sincroniza por lotes** las sesiones/actividad/capturas.
- **Idempotencia:** cada registro lleva su **ULID** (generado en el agente). El
  endpoint de sync hace *upsert* por ULID → reenviar el mismo lote no duplica.
- La plataforma es la **fuente de verdad consolidada**; el agente es la **fuente de
  captura**.

### Flujo de autenticación
1. Usuario hace login (web o agente) → API valida (hash bcrypt/argon2) → emite
   **access token** (corto) + **refresh token** (largo, rotado/revocable).
2. El cliente guarda tokens (web: cookie httpOnly; agente: almacenamiento seguro
   del SO).
3. Cada request lleva el access token; al expirar, se rota con el refresh.
4. Logout/revocación invalida la sesión (tabla `Session`).

### Flujo de permisos
Request → `AuthGuard` (¿token válido?) → `RolesGuard` (¿rol permite la acción?) →
chequeo de **tenant** (¿el recurso es de su empresa?) → chequeo de **ownership**
cuando aplica → Service. Falla cualquiera → 401/403 controlado, sin filtrar
internals.

### Flujo de registro de tiempo (agente → plataforma)
1. Empleado inicia timer en el agente (tarea X) → sesión local abierta (sesión
   única garantizada por el índice parcial — ya hecho).
2. Para/cambia → sesión cerrada localmente con tiempo canónico del backend del
   agente (`clock.now()` UTC).
3. El agente sincroniza el lote a `POST /sync/time-entries` (idempotente por ULID).
4. La plataforma *upserta*, valida (no solapamiento imposible, `is_suspect`
   respetado) y consolida por empresa/usuario/proyecto.

### Flujo de generación de reportes
Las consultas de reporte corren **en la plataforma** sobre datos ya consolidados
(no en el agente): agregaciones por usuario/proyecto/tarea/fecha, reusando la
lógica de **solapamiento por día** definida en `02-time-policy.md` (que el agente
usará para su vista "hoy" en Plan 5, y la plataforma para reportes globales).

---

## Próximos documentos (este set, en orden)

1. **`01-data-model-prisma.md`** — schema Prisma: User, Company, Team, Project,
   Task, TimeEntry, ActivityLog, Screenshot, Report, Role, Permission, Session,
   AuditLog (campos, relaciones, índices, reglas de negocio). *(brief §8)*
2. **`02-api-and-security.md`** — endpoints REST + JWT/refresh/RBAC/rate-limit/
   validación/auditoría. *(brief §9-10)*
3. **`03-backend-nestjs-structure.md`** — organización por módulos
   (controller/service/repo/DTO/guards). *(brief §5-7)*
4. **`04-frontend-nextjs-structure.md`** — carpetas, componentes, estado, forms.
   *(brief §14-17)*
5. **`05-roadmap.md`** — fases de construcción → de ahí salen los planes
   incrementales (spec→plan→build por slices, como el agente). *(brief §21)*

## Decisión pendiente (al pasar a construir, no ahora)
**Estructura de repos:** ¿la plataforma vive en un **repo nuevo separado**
(recomendado: toolchains muy distintas — Rust/Cargo vs Node/npm — y despliegues
distintos) o en un **monorepo** junto al agente? Se decide al scaffoldear código;
la documentación de arquitectura vive en este repo (hogar del producto).
