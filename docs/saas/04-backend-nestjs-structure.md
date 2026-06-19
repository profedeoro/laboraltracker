# LaboralTracker SaaS — Estructura del backend (NestJS)

> Documento **4** del set de arquitectura. Implementa lo definido en los docs
> [00](00-architecture-overview.md) (arquitectura), [01](01-data-model-prisma.md)
> (modelo), [02](02-api-and-security.md) (API + guards) y
> [03](03-privacy-policy.md) (privacidad). Cubre los puntos 5-7 del brief.
>
> Rige `~/.claude/rules/nestjs-backend.md`. **Alcance:** diseño de organización, no
> código. El scaffold real va cuando arranquemos a construir (decisión de repo
> pendiente, doc 00).

---

## 1. Principio de organización

**Modular por feature, en capas por responsabilidad.** Cada módulo de negocio es
una unidad cohesiva; lo transversal vive en `common`; la persistencia se aísla en
`database` + repositorios. La regla de oro (misma disciplina hexagonal que el
agente, doc 00):

```txt
Controller → DTO/validación → Service (lógica) → Repository (Prisma) → PostgreSQL
            ↑ Guards / Interceptors / Pipes (transversal)
```

- El **controller** recibe y delega. No tiene lógica de negocio.
- El **service** tiene la lógica y las **reglas que el schema no expresa** (doc 01
  §3): coherencia de tenant, idempotencia, `isSuspect`, filtro `deletedAt IS NULL`.
- El **repository** habla con Prisma. Aísla la persistencia del dominio.
- Los **guards/pipes/interceptors** son transversales y se declaran, no se repiten.

---

## 2. Estructura de carpetas

```txt
backend/
├── src/
│   ├── main.ts                      # bootstrap: ValidationPipe global, Helmet, CORS
│   ├── app.module.ts
│   │
│   ├── config/                      # configuración centralizada (no os.env suelto)
│   │   ├── env.config.ts            # validación de env con Zod/Joi al arrancar
│   │   ├── jwt.config.ts
│   │   └── storage.config.ts        # bucket de capturas (S3/R2)
│   │
│   ├── common/                      # TRANSVERSAL — el corazón de la seguridad
│   │   ├── guards/
│   │   │   ├── auth.guard.ts        # 1) ¿token válido y no revocado?
│   │   │   ├── permission.guard.ts  # 2) ¿el rol tiene el Permission?
│   │   │   ├── tenant.guard.ts      # 3) ¿recurso de la empresa + membresía activa?
│   │   │   └── ownership.guard.ts   # 4) ¿el recurso es del propio user? (self)
│   │   ├── decorators/
│   │   │   ├── require-permission.decorator.ts   # @RequirePermission('time.read')
│   │   │   ├── current-user.decorator.ts         # inyecta {userId, companyId, roleId}
│   │   │   └── public.decorator.ts               # marca rutas sin AuthGuard (login)
│   │   ├── filters/
│   │   │   └── all-exceptions.filter.ts          # forma de error estándar (doc 02 §7)
│   │   ├── interceptors/
│   │   │   ├── response.interceptor.ts           # envuelve en { success, data }
│   │   │   └── audit.interceptor.ts              # escribe AuditLog en acciones sensibles
│   │   └── pipes/                                 # validación/transform a medida si hace falta
│   │
│   ├── database/
│   │   ├── prisma.service.ts        # PrismaClient como provider inyectable
│   │   ├── prisma.module.ts         # global
│   │   └── seed.ts                  # 5 roles + permisos + RolePermission (doc 01 §5)
│   │
│   ├── modules/                     # FEATURES (uno por módulo del doc 00 §3)
│   │   ├── auth/                    # login, refresh (rotación), logout, switch-company
│   │   ├── users/                   # identidad global
│   │   ├── companies/               # tenant + tracking-settings (tipado)
│   │   ├── members/                 # CompanyMember (rol por empresa)
│   │   ├── teams/
│   │   ├── projects/
│   │   ├── tasks/
│   │   ├── time-entries/
│   │   ├── activity/
│   │   ├── screenshots/             # metadata + URL firmada del bucket
│   │   ├── devices/                 # autorizar/revocar equipos del agente
│   │   ├── sync/                    # EL endpoint de sync (doc 02 §3)
│   │   ├── reports/                 # agregaciones (excluye isSuspect)
│   │   └── audit/                   # lectura de AuditLog (append vive en el interceptor)
│   │
│   └── jobs/
│       └── retention.job.ts         # purga histórico vencido (retentionDays, doc 03 §5)
│
├── prisma/
│   ├── schema.prisma                # el modelo del doc 01
│   └── migrations/
├── test/
│   ├── unit/                        # reglas de negocio (services)
│   ├── integration/                 # repos contra Postgres de prueba
│   └── e2e/                         # flujos: login, sync, reportes
└── .env.example
```

---

## 3. Anatomía de un módulo de feature

Cada módulo sigue la misma forma (ejemplo `time-entries/`):

```txt
modules/time-entries/
├── time-entries.controller.ts   # rutas; declara guards y @RequirePermission
├── time-entries.service.ts      # lógica + reglas (ownership self, filtro deletedAt)
├── time-entries.repository.ts   # acceso Prisma aislado
├── time-entries.module.ts
└── dto/
    ├── list-time-entries.dto.ts # query: rango, filtros (whitelist), paginación
    └── time-entry.response.dto.ts
```

Ejemplo conceptual del controller (declara seguridad, delega todo):

```ts
@Controller('time-entries')
@UseGuards(AuthGuard, PermissionGuard, TenantGuard)
export class TimeEntriesController {
  constructor(private readonly service: TimeEntriesService) {}

  @Get()
  @RequirePermission('time.read')          // el guard lo resuelve contra el rol
  list(@CurrentUser() user: AuthCtx, @Query() q: ListTimeEntriesDto) {
    return this.service.list(user, q);     // service aplica ownership self vs equipo
  }
}
```

El **service** es quien decide `time.read` (equipo) vs `time.read.self` (propio)
sin ensuciar el controller — ahí vive la regla, no en la ruta.

---

## 4. Dónde viven los 4 guards (y por qué transversales)

Los guards del doc 02 §5 viven en `common/guards` y se aplican por módulo/ruta en
**orden**: `Auth → Permission → Tenant → Ownership`.

- **Permisos como datos** (doc 01): `PermissionGuard` lee el `Permission` requerido
  (del `@RequirePermission`) y lo compara contra los permisos del rol de la
  `CompanyMember` del contexto. **Agregar un permiso no toca ningún controller**
  (OCP real, no eslogan).
- **`TenantGuard`** resuelve `companyId` del token, verifica membresía activa, y deja
  el filtro de empresa disponible para el repositorio (toda query lleva `companyId`).
- **`OwnershipGuard`** solo donde "lo propio vs lo ajeno" importa (capturas,
  actividad, tiempo del employee).

Centralizarlos cumple DRY y, sobre todo, evita el peor bug de un SaaS multi-tenant:
**que un endpoint se olvide de filtrar por empresa**. Si la seguridad estuviera
repartida en cada controller, ese olvido sería cuestión de tiempo.

---

## 5. Módulos especiales

### 5.1 `auth/`
Login (verifica hash argon2/bcrypt), `refresh` con **rotación** (revoca el viejo,
detecta reuso → revoca cadena), `logout`, `switch-company` (cambia el contexto de
empresa activa para usuarios multi-empresa, doc 02 §4). Guarda `Session` con el
refresh **hasheado**, atado a `Device`.

### 5.2 `sync/`
El endpoint del doc 02 §3. Su service:
1. Valida `Device` autorizado y no revocado → si no, 403.
2. Abre/actualiza `SyncBatch` (`RECEIVED → PROCESSING → DONE/FAILED`).
3. Hace `upsert` por ULID (idempotente) dentro de una **transacción** Prisma.
4. Devuelve **respuesta parcial** (`accepted`/`conflicts`/`errors`), aplicando la
   política de colisión de nombres (rechazo por ítem, sin renombrar/mergear).

### 5.3 `reports/`
Agregaciones sobre datos consolidados; **siempre** filtra `isSuspect = false` y
`deletedAt IS NULL`. Para reportes pesados, consultas SQL crudas vía Prisma
`$queryRaw` parametrizado (nunca interpolando input del cliente).

### 5.4 `jobs/retention.job.ts`
Tarea programada (`@nestjs/schedule`) que purga capturas/actividad vencidas según
`CompanyTrackingSettings.retentionDays` (doc 03 §5): borra el binario del bucket +
el registro, con traza en `AuditLog`. El soft-delete es lógico; este job hace el
físico, deliberado.

---

## 6. Transversales en `main.ts` (bordes del sistema)

- **`ValidationPipe` global** con `whitelist: true`, `forbidNonWhitelisted: true`,
  `transform: true` → descarta campos no declarados (defensa contra mass-assignment).
- **Helmet** (headers de seguridad) + **CORS** por whitelist (no `*`).
- **Rate limiting** (`@nestjs/throttler`) estricto en `auth/*`, razonable en
  `sync/*`.
- **`AllExceptionsFilter`** global → mapea todo a la forma de error estándar
  (doc 02 §7), loguea el detalle **del lado servidor** (sin secretos ni PII), nunca
  filtra stack traces ni errores crudos de Prisma al cliente.
- **`ResponseInterceptor`** → envuelve respuestas en `{ success, data }`.

---

## 7. Validación de configuración al arrancar

`config/env.config.ts` valida las variables de entorno **al boot** (Zod/Joi): si
falta `DATABASE_URL`, `JWT_SECRET`, credenciales del bucket, etc., el server **no
arranca** (falla rápido y claro, no a mitad de un request). Nada de leer
`process.env` suelto por el código (regla `nestjs-backend.md`).

---

## 8. Testing (qué se prueba en cada nivel)

Alineado con el flujo del agente (TDD por capas) y `nestjs-backend.md`:

- **Unit** (`test/unit`): reglas de negocio en los **services** — ownership self vs
  equipo, idempotencia del sync, exclusión de `isSuspect`, filtro `deletedAt`.
- **Integration** (`test/integration`): **repositorios** contra un Postgres de
  prueba — que las queries y los índices hagan lo esperado.
- **E2E** (`test/e2e`): flujos completos — login+refresh+rotación, sync idempotente
  (reenviar el mismo lote no duplica), un reporte de horas, y **denegación de
  acceso cruzado entre empresas** (el test que protege el invariante multi-tenant).

> El test de aislamiento entre tenants es **obligatorio**: es la prueba que
> garantiza que una empresa nunca ve datos de otra.

---

## Próximo documento

**`05-frontend-nextjs-structure.md`** — la organización del dashboard Next.js:
carpetas por feature, componentes (container/presentational), estado global acotado
(auth/rol/empresa activa), formularios con React Hook Form + Zod, y el cliente HTTP
con manejo de token/refresh. *(brief §14-17)*
