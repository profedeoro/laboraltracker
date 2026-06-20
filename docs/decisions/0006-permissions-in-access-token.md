# 0006 — Permisos resueltos en el access token (JWT)

- **Status:** Accepted
- **Date:** 2026-06-19

## 🇬🇧 Context
The platform models authorization as **data**: `Permission` (atomic capability,
e.g. `time.read`) ↔ `RolePermission` ↔ `Role`, and a user's role lives on the
`CompanyMember` (a person can hold different roles in different companies). The
`PermissionGuard` (doc 02 §5) enforces a required permission per protected route.

As designed, resolving "does this request's role hold permission X?" means walking
`token.roleId → RolePermission → Permission` on **every protected request**. The
access token (doc 02 §4.1) already carries `sub`, `email`, `companyId`, `roleId`,
but **not** the resolved permission set.

This is a hot path. The agent is offline-first and syncs in batches, but a busy
tenant (hundreds of employees returning Monday 9 AM) drives a burst of protected
requests; adding ~3 queries before each controller runs is avoidable load that
will not hurt in dev and will hurt in production.

## 🇪🇸 Contexto
La plataforma modela la autorización como **datos**: `Permission` (capacidad
atómica, p. ej. `time.read`) ↔ `RolePermission` ↔ `Role`, y el rol del usuario vive
en la `CompanyMember` (una persona puede tener roles distintos en empresas
distintas). El `PermissionGuard` (doc 02 §5) exige un permiso por ruta protegida.

Tal como está diseñado, resolver "¿el rol de este request tiene el permiso X?"
implica recorrer `token.roleId → RolePermission → Permission` en **cada request
protegido**. El access token (doc 02 §4.1) ya lleva `sub`, `email`, `companyId`,
`roleId`, pero **no** el conjunto de permisos resueltos.

Es un camino caliente. El agente es offline-first y sincroniza por lotes, pero un
tenant cargado (cientos de empleados volviendo un lunes 9 AM) genera una ráfaga de
requests protegidos; sumar ~3 queries antes de cada controlador es carga evitable
que no se nota en desarrollo y sí duele en producción.

## 🇬🇧 Options / 🇪🇸 Opciones
- **A) Resolved permissions in the JWT (recommended).** Compute the permission keys
  (`string[]`) at login / refresh / `switch-company` and embed them as a signed
  claim. `PermissionGuard` becomes an in-memory array membership check. The
  `Permission`/`RolePermission` tables remain the authoritative **catalog** (seed +
  admin UI), no longer queried on the hot path. Cost: bounded staleness (a
  grant/revoke applies on next token refresh, ≤ access-token TTL) and token size
  grows with the permission set. / **Permisos resueltos en el JWT (recomendada).**
  Calcular las claves de permiso (`string[]`) al login / refresh / `switch-company`
  y embeberlas como claim firmado. El `PermissionGuard` pasa a ser una verificación
  de pertenencia a un array en memoria. Las tablas `Permission`/`RolePermission`
  quedan como **catálogo** autoritativo (seed + admin), fuera del camino caliente.
  Coste: staleness acotado (un cambio aplica en el próximo refresh, ≤ TTL del
  access) y el token crece con el set de permisos.
- **B) Per-request resolution + cache (Redis/in-memory, TTL).** Keep resolving but
  cache `roleId → permissions`. Fresher than A, no token bloat, but adds cache
  infrastructure and still a lookup per request. / **Resolución por request + caché
  (Redis/memoria, TTL).** Seguir resolviendo pero cachear `roleId → permisos`. Más
  fresco que A, sin inflar el token, pero suma infraestructura de caché y aún hay
  un lookup por request.
- **C) Per-request DB resolution (status quo).** Simplest, always fresh, but the
  N+1 on the hot path at scale. / **Resolución por request contra la BD (status
  quo).** Lo más simple, siempre fresco, pero el N+1 en el camino caliente a escala.

## 🇬🇧 Decision / 🇪🇸 Decisión
**Option A.** The access token carries a signed `permissions: string[]` claim,
recomputed at login, refresh, and `switch-company`. `PermissionGuard` checks
membership in that array — no DB query. `Permission`/`RolePermission` stay as the
seed-backed catalog and source of truth for the admin UI. We accept bounded
staleness: the ~15-min access-token TTL already caps it, and role/permission edits
are infrequent admin actions. Where **immediate** revocation is required, we revoke
the **`Session`** (kills the token entirely, forcing re-login) rather than chasing
per-permission freshness — that mechanism already exists (doc 02 §4.3). /
**Opción A.** El access token lleva un claim firmado `permissions: string[]`,
recalculado al login, refresh y `switch-company`. El `PermissionGuard` verifica
pertenencia a ese array — sin query a la BD. `Permission`/`RolePermission` quedan
como catálogo con seed y fuente de verdad para el admin. Aceptamos staleness
acotado: el TTL de ~15 min del access ya lo limita, y editar roles/permisos es una
acción de admin poco frecuente. Cuando se necesita revocación **inmediata**, se
revoca la **`Session`** (invalida el token entero y fuerza re-login) en vez de
perseguir frescura por permiso — ese mecanismo ya existe (doc 02 §4.3).

## 🇬🇧 Consequences / 🇪🇸 Consecuencias
- `PermissionGuard` drops to an O(1) array check; ~3 queries removed per protected
  request. The `Permission` table leaves the hot path. / El `PermissionGuard` baja
  a una verificación O(1); se quitan ~3 queries por request protegido. La tabla
  `Permission` sale del camino caliente.
- **Staleness window:** a permission grant/revoke takes effect on the next refresh
  (≤ access TTL). Must be documented in doc 02 §4/§5; for instant effect, revoke the
  Session. / **Ventana de staleness:** un cambio de permiso aplica en el próximo
  refresh (≤ TTL del access). Documentar en doc 02 §4/§5; para efecto inmediato,
  revocar la Session.
- **Token size:** keep permission keys compact; watch the header/cookie ceiling
  (~4–8 KB) if the catalog grows large. Revisit (cache, B) only if a role's permission
  set ever gets big. / **Tamaño del token:** mantener claves compactas; vigilar el
  techo de header/cookie (~4–8 KB) si el catálogo crece. Reconsiderar (caché, B) solo
  si el set de un rol llega a ser grande.
- Recompute permissions on `switch-company` (different membership → different role →
  different set). / Recalcular permisos en `switch-company` (otra membresía → otro
  rol → otro set).
- Permissions are **signed**, not client-supplied → tamper-proof. They do **not**
  replace `TenantGuard` or ownership: a permission says *what action*, the tenant
  guard says *whose data*. Both still run (doc 02 §5). / Los permisos van **firmados**,
  no los manda el cliente → a prueba de manipulación. **No** reemplazan al
  `TenantGuard` ni al ownership: el permiso dice *qué acción*, el tenant guard dice
  *de quién es el dato*. Ambos siguen corriendo (doc 02 §5).
- Update doc 02 §4.1 (token claims) and §5 (guard) to reflect the embedded
  permissions when scaffolding the backend. / Actualizar doc 02 §4.1 (claims del
  token) y §5 (guard) para reflejar los permisos embebidos al scaffoldear el backend.
