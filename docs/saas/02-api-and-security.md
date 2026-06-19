# LaboralTracker SaaS — API REST y seguridad

> Documento **2** del set de arquitectura. Se construye sobre
> [`00-architecture-overview.md`](00-architecture-overview.md) y
> [`01-data-model-prisma.md`](01-data-model-prisma.md). Cubre los puntos 9-10 del
> brief: contrato de la API, **el endpoint de sync** agente→plataforma,
> autenticación (JWT + refresh), autorización (RBAC + tenant + ownership) y el
> *hardening* transversal.
>
> **Alcance:** diseño, no código. Los endpoints reales se escriben al scaffoldear
> el backend NestJS (doc 04).

---

## 1. Convenciones REST

Alineado con `~/.claude/rules/backend.md` y `~/.claude/rules/nestjs-backend.md`.

- **Versionado**: prefijo `/api/v1`. Romper contrato → `/api/v2`, no mutar v1.
- **Recursos en plural**, kebab-case: `/api/v1/projects`, `/api/v1/time-entries`.
- **Métodos**: `GET` (leer), `POST` (crear), `PATCH` (actualizar parcial),
  `DELETE` (baja **lógica** — marca `deletedAt`, no borra; ver doc 01 §0.4).
- **Acciones de negocio** que no son CRUD → sub-recurso `POST`:
  `POST /api/v1/devices/:id/revoke`, `POST /api/v1/sync/time-entries`.
- **Paginación** en listados: `?page=1&limit=20` → respuesta con `meta`
  (`page/limit/total/totalPages`). Límite máximo duro (p. ej. 100) en el servidor.
- **Filtros/orden** controlados por whitelist (nunca interpolar campos del cliente
  en SQL/ORDER BY — riesgo de inyección).
- **Respuesta consistente**:

```jsonc
// éxito
{ "success": true, "data": { /* ... */ } }
// éxito paginado
{ "success": true, "data": [ /* ... */ ], "meta": { "page": 1, "limit": 20, "total": 137, "totalPages": 7 } }
```

---

## 2. Endpoints por módulo (resumen)

> Todos bajo `/api/v1`. Columna **Permiso** = clave de `Permission` (doc 01); el
> `Roles/PermissionsGuard` la exige. Columna **Tenant** = el `TenantGuard` filtra
> por `companyId` del contexto. "Ownership" = además se valida que el recurso sea
> del solicitante cuando aplica.

| Método y ruta | Qué hace | Permiso | Notas |
|---|---|---|---|
| `POST /auth/login` | Login → access + refresh | — (público) | rate-limit estricto |
| `POST /auth/refresh` | Rota el refresh token | — (cookie/refresh) | rotación + revocación |
| `POST /auth/logout` | Revoca la sesión | autenticado | invalida `Session` |
| `GET  /me` | Perfil + membresías del usuario | autenticado | identidad global |
| `POST /companies` | Crea empresa (onboarding) | `company.create` | super admin / signup |
| `GET  /companies/:id` | Datos de empresa | `company.read` | tenant |
| `PATCH /companies/:id/tracking-settings` | Política de captura | `company.manage` | tipado (doc 01) |
| `GET/POST /users` · `PATCH /users/:id` | Gestión de personas | `user.manage` | tenant |
| `POST /members` · `PATCH /members/:id` | Alta/edición de membresía+rol | `member.manage` | tenant |
| `GET/POST /teams` · `PATCH /teams/:id` | Equipos | `team.manage` | tenant |
| `GET/POST /projects` · `PATCH /projects/:id` | Proyectos | `project.manage` / `project.read` | tenant |
| `GET/POST /tasks` · `PATCH /tasks/:id` | Tareas | `task.manage` / `task.read` | tenant |
| `GET /time-entries` | Listar tiempo (filtros) | `time.read` (propio: `time.read.self`) | tenant + ownership |
| `GET /activity` · `GET /screenshots` | Actividad / capturas | `activity.read` / `screenshot.read` | tenant + ownership |
| `GET /reports` · `POST /reports` | Reportes/export | `report.view` | tenant |
| `GET/POST /devices` · `POST /devices/:id/revoke` | Equipos del agente | `device.manage` (propio: `device.read.self`) | tenant + ownership |
| `POST /sync/time-entries` | **Sync del agente** (ver §3) | `sync.write` (rol agente/employee) | tenant + device auth |

El detalle de DTOs por endpoint va en el doc 04 (estructura backend); acá fijamos
el contrato y la seguridad.

---

## 3. El endpoint de sync (corazón de la integración)

`POST /api/v1/sync/time-entries` — el agente, offline-first, sube en **lotes** sus
sesiones/actividad/capturas cuando hay red.

### 3.1 Request

```jsonc
{
  "batchId": "01J...",        // ULID generado por el agente = idempotency key del lote
  "deviceId": "01J...",       // equipo que sincroniza
  "timeEntries": [
    { "id": "01J...", "taskId": "01J...", "startedAt": 1718800000000,
      "endedAt": 1718803600000, "lastHeartbeatAt": 1718803590000, "isSuspect": false }
  ],
  "activityLogs": [ /* ... */ ],
  "screenshots":  [ /* metadata; el binario se sube aparte (URL firmada) */ ]
}
```

### 3.2 Garantías (las hace cumplir el servicio, doc 01 §3)

1. **Idempotencia por ULID**: `upsert` por `id` de cada registro. Reenviar el mismo
   lote no duplica; campos mutables (`endedAt`, `isSuspect`, `lastHeartbeatAt`) se
   actualizan. El `batchId` (también ULID) deduplica el lote completo.
2. **Autorización de dispositivo**: el `Device` debe tener `authorizedAt != NULL` y
   `revokedAt == NULL`. Equipo no autorizado o revocado → **403**, no sincroniza.
3. **Tenant**: cada registro se escribe con el `companyId` del contexto del token;
   nunca el que mande el cliente.
4. **Auditoría**: cada llamada crea/actualiza un `SyncBatch`
   (`RECEIVED → PROCESSING → DONE/FAILED`, `itemCount`, `error`). Responder "no
   aparecen mis horas" deja de ser adivinanza.

### 3.3 Respuesta parcial (clave: un ítem malo no tumba el lote)

```jsonc
{
  "success": true,
  "data": {
    "batchId": "01J...",
    "accepted": 142,
    "conflicts": [ { "id": "01J...", "reason": "NAME_TAKEN", "field": "name" } ],
    "errors":   [ { "id": "01J...", "reason": "TASK_NOT_FOUND" } ]
  }
}
```

El agente reintenta lo fallido y/o se lo muestra al usuario; lo aceptado queda
firme.

### 3.4 Decisión: colisión de nombres en la sync (cierra doc 01 §3.8)

`Project`/`Task` sincronizan **por ULID (`id`)**, no por nombre. Una colisión solo
ocurre si **dos ids distintos** comparten `name` en la misma empresa, chocando con
`@@unique([companyId, name])`.

**Política elegida: rechazo a nivel de ítem, sin renombrar ni mergear.**

- **Por qué no auto-renombrar**: corromper el nombre que eligió el usuario en
  silencio es una sorpresa fea (aparece "Proyecto (2)" sin que nadie lo pidiera).
- **Por qué no auto-mergear**: dos ids distintos pueden tener `TimeEntry` distintos;
  fusionarlos mezcla historial de tiempo de forma irreversible. Inaceptable en una
  app laboral.
- **Qué hace**: el ítem en conflicto vuelve en `conflicts[]` con `reason:
  "NAME_TAKEN"`; el resto del lote se acepta. El agente le pide al usuario que
  renombre (decisión humana, no del sistema).
- **Consistencia con el agente**: el agente debe **espejar** la restricción única
  (índice único parcial sobre no-archivados/no-borrados en su SQLite) para detectar
  el choque **antes** de sincronizar y resolverlo localmente. Esto entra en el
  backlog del agente (no en Plans 1-4).

> Nota de dirección de verdad: a futuro, lo natural es que **proyectos/tareas nazcan
> en la plataforma** (manager) y el agente los **baje** (pull), reduciendo
> colisiones a casos de creación offline. La política de arriba cubre justamente ese
> caso de borde.

---

## 4. Autenticación (JWT + refresh)

### 4.1 Tokens

- **Access token** (JWT, corto: ~15 min): `sub` (userId), `email`, y el **contexto
  de empresa activa** (`companyId` + `roleId` resueltos de la `CompanyMember`). Como
  un usuario puede estar en varias empresas, el contexto se fija al login/refresh
  (o vía `POST /auth/switch-company`).
- **Refresh token** (largo: ~7-30 días): **opaco**, no JWT. Se guarda **hasheado**
  en `Session.refreshTokenHash` (nunca el token en claro). Atado a un `Device`.

### 4.2 Almacenamiento por cliente

- **Web (Next.js)**: refresh en **cookie `httpOnly` + `Secure` + `SameSite`**;
  access en memoria. Mitiga XSS (JS no lee la cookie).
- **Agente (Tauri)**: tokens en el **almacenamiento seguro del SO** (keychain /
  credential manager), no en texto plano.

### 4.3 Rotación y revocación

- Cada `POST /auth/refresh` **rota**: emite refresh nuevo, marca el viejo
  `revokedAt`. Reuso de un refresh ya rotado → **revoca toda la cadena** (señal de
  robo de token) y fuerza re-login.
- `logout` y "revocar dispositivo" marcan `revokedAt`. Token caducado o revocado →
  401.

### 4.4 Flujo

```txt
login (email+pass) ─→ valida hash (bcrypt/argon2) ─→ access(15m) + refresh(rotable)
   request con access ─→ (expira) ─→ /auth/refresh con refresh ─→ access nuevo + refresh nuevo
   logout / revoke ─→ Session.revokedAt ─→ 401 en el siguiente intento
```

---

## 5. Autorización: cuatro guards en orden

Autenticar ≠ autorizar. Cada request protegido pasa, **en este orden**, por:

```txt
Request
  → 1. AuthGuard      ¿token válido y no revocado?          falla → 401
  → 2. PermissionGuard ¿el rol tiene el Permission requerido? falla → 403
  → 3. TenantGuard    ¿el recurso es de la empresa del token
                       y el user tiene CompanyMember activa?  falla → 403/404
  → 4. Ownership      (cuando aplica) ¿el recurso es del
                       propio user? p. ej. ver MIS capturas    falla → 403
  → Controller → Service
```

- **Permisos como datos** (doc 01): el `PermissionGuard` lee el `Permission`
  requerido (declarado con un decorador `@RequirePermission('time.read')`) y lo
  compara contra los permisos del rol de la membresía. Agregar un permiso/rol **no
  obliga a tocar cada controlador** (OCP).
- **`time.read` vs `time.read.self`**: un `MANAGER` ve el tiempo de su equipo;
  un `EMPLOYEE` solo el propio. El guard de ownership distingue ambos casos sin
  duplicar lógica en cada endpoint.
- **404 vs 403**: para recursos de otra empresa se prefiere **404** (no revelar
  existencia) salvo cuando el 403 es informativo y seguro.

---

## 6. Hardening transversal

- **Validación en el borde**: todo body/query/param pasa por DTO validado
  (`class-validator` + `ValidationPipe` con `whitelist` + `forbidNonWhitelisted` +
  `transform`). El front además valida con **Zod** (UX), pero **el backend revalida
  siempre** (integridad).
- **Rate limiting**: estricto en `/auth/login` y `/auth/refresh` (anti
  fuerza-bruta); razonable en `/sync/*` (un agente legítimo sincroniza por lotes,
  no en ráfaga).
- **CORS**: whitelist de orígenes (dashboard web + esquema del agente), no `*`.
- **Headers de seguridad**: Helmet (CSP, HSTS, etc.).
- **Secretos**: solo por variables de entorno / secrets manager; nunca en el repo ni
  en logs.
- **Errores sin fugas**: nunca devolver stack traces ni errores crudos del ORM al
  cliente. Un `ExceptionFilter` global mapea a la forma estándar (§7) y **loguea el
  detalle del lado servidor** (sin secretos ni PII sensible).
- **Auditoría**: acciones sensibles (borrado lógico, export de reporte, revocar
  dispositivo, cambiar política de captura) escriben en `AuditLog`.

---

## 7. Formato de error estándar

```jsonc
{
  "success": false,
  "error": {
    "code": "FORBIDDEN",            // enum estable, no el mensaje
    "message": "No tenés permiso para ver este recurso",
    "details": [ /* errores de validación campo a campo, si aplica */ ]
  },
  "timestamp": "2026-06-19T12:00:00.000Z",
  "path": "/api/v1/time-entries"
}
```

Códigos HTTP: `400` validación · `401` no autenticado · `403` sin permiso · `404`
no existe (o tenant ajeno) · `409` conflicto (p. ej. `NAME_TAKEN`) · `422` regla de
negocio · `429` rate-limit · `500` interno (sin detalle al cliente).

---

## Próximo documento

**`03-privacy-policy.md`** — el documento de **privacidad** (producto + legal) que
pidió la revisión: qué se captura y qué no, quién puede verlo, retención
(`retentionDays`), cómo se elimina, cómo se informa al empleado, y las
configuraciones por empresa (`CompanyTrackingSettings`). En una app que captura
pantalla y actividad, esto no es opcional. *(derivado de la revisión del doc 01)*
