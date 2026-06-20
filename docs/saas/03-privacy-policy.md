# LaboralTracker SaaS — Privacidad y monitoreo (producto + legal)

> Documento **3** del set de arquitectura. Surge de la revisión técnica del doc 01.
> Se apoya en [`00-architecture-overview.md`](00-architecture-overview.md),
> [`01-data-model-prisma.md`](01-data-model-prisma.md) (modelo
> `CompanyTrackingSettings`, `retentionDays`, soft-delete) y
> [`02-api-and-security.md`](02-api-and-security.md) (quién puede ver qué).
>
> **Por qué existe:** LaboralTracker captura **pantalla** y **actividad de
> teclado/mouse**. Eso es dato personal y monitoreo laboral. No es un anexo legal:
> es una **decisión de producto** que el sistema debe **hacer cumplir por diseño**,
> no solo prometer. Privacidad por diseño (SWEBOK §Seguridad; principio de
> minimización de datos).
>
> **Aviso:** este documento define el comportamiento del producto. **No es
> asesoría legal.** La conformidad concreta (GDPR/UE, leyes laborales por país,
> consentimiento) debe revisarla un profesional según la jurisdicción de cada
> empresa cliente.

---

## 1. Principio rector: minimización y transparencia

Dos reglas que gobiernan todo lo demás:

1. **Capturar lo mínimo necesario** para el propósito declarado (medir tiempo y
   productividad laboral), nunca "todo por si acaso".
2. **El empleado siempre sabe** qué se captura, cuándo y quién lo ve. Monitoreo
   oculto = fuera de alcance, por diseño.

---

## 2. Qué se captura — y qué NO

### Se captura (si la empresa lo habilita)

| Dato | Detalle | Modelo |
|---|---|---|
| **Tiempo de trabajo** | Sesiones inicio/fin por tarea | `TimeEntry` |
| **Nivel de actividad** | **Conteos agregados** de teclado/mouse por ventana, % actividad, inactividad | `ActivityLog` |
| **Capturas de pantalla** | Imagen por intervalo configurable, opcionalmente difuminada | `Screenshot` |
| **Metadatos de equipo** | Plataforma, versión del agente, última conexión | `Device` |

### NO se captura (límite duro del producto)

- **NO keylogging**: se cuentan pulsaciones (cuántas), **no se registra qué teclas
  ni el texto escrito**. `ActivityLog` guarda `keyboardCount`, no contenido.
- **NO contenido de portapapeles, archivos, mensajes ni navegación detallada.**
- **NO captura fuera del horario/sesión** cuando el timer está parado (la captura
  vive atada a una sesión activa).
- **NO audio ni cámara.**

Este límite es **de diseño**: el agente no tiene esas capacidades, no es solo una
política. (Si alguna vez se propusiera, sería un cambio de producto con su propio
ADR y revisión legal — no un *flag* más.)

---

## 3. Configuración por empresa (tipada, no JSON suelto)

Cada empresa decide su política vía `CompanyTrackingSettings` (doc 01) — campos
**tipados** justamente porque tienen peso legal:

| Campo | Qué controla | Default |
|---|---|---|
| `screenshotsEnabled` | Si se capturan pantallas | `false` (opt-in consciente) |
| `screenshotIntervalMin` | Frecuencia de captura | `10` min |
| `blurScreenshots` | Difuminado obligatorio (reduce exposición de contenido sensible) | `false` |
| `activityTracking` | Conteos de teclado/mouse | `true` |
| `idleThresholdMin` | A partir de cuánto se marca inactividad | `5` min |
| `retentionDays` | A cuántos días se purga lo histórico | `90` |

**Decisión de producto:** las capturas vienen **desactivadas por defecto**
(`false`). Habilitarlas es una acción consciente del `COMPANY_ADMIN`, no un estado
silencioso. El agente **lee esta política** y se comporta en consecuencia (doc 00:
la plataforma administra, el agente captura).

---

## 4. Quién ve qué (acotado por RBAC — doc 00 §2 y doc 02 §5)

La visibilidad NO es libre: la imponen los guards `Tenant` + `Permission` +
`Ownership`.

| Rol | Tiempo | Actividad | Capturas |
|---|---|---|---|
| **Employee** | Solo el **propio** (`time.read.self`) | El propio | Las **propias** |
| **Manager** | Su **equipo** | Su equipo | Su equipo (`screenshot.read`) |
| **Company Admin** | Toda su empresa | Toda su empresa | Toda su empresa |
| **Client Viewer** | Reportes acotados (horas) | **No** ve actividad cruda | **No** ve capturas |
| **Super Admin** | **No** ve datos de negocio salvo soporte explícito y **auditado** | idem | idem |

- El **Client Viewer** (cliente final de la empresa) ve **horas/reportes**, nunca
  capturas ni actividad cruda — para no exponer al empleado ante un tercero.
- El **Super Admin** (operador del SaaS) **no fisgonea**: cualquier acceso de
  soporte a datos de una empresa queda en `AuditLog` (doc 01).

---

## 5. Retención y borrado

- **Retención**: lo histórico (capturas, actividad) se purga a los `retentionDays`
  de la empresa. Un **job programado** elimina lo vencido (físico del bucket +
  registro), con su propia traza.
- **Borrado lógico primero** (doc 01 §0.4): dar de baja una empresa/usuario marca
  `deletedAt` y **no** evapora historial en cascada (obligación legal/laboral de
  conservar registros de tiempo). El purgado físico es deliberado, no un efecto
  colateral.
- **Capturas**: `deletedAt` lógico → el binario en object storage se borra en el job
  de purga (no queda huérfano).
- **Derecho de acceso/eliminación del titular** (si la jurisdicción lo exige):
  soportado por export de reportes + borrado dirigido; el endpoint concreto se
  define en el doc 04. Toda eliminación dirigida → `AuditLog`.

---

## 6. Cómo se le informa al empleado (transparencia operativa)

No alcanza con una cláusula enterrada. El producto **muestra** el monitoreo:

1. **Aviso al instalar/activar el agente**: qué se captura según la política de su
   empresa (capturas sí/no, intervalo, actividad sí/no), antes de empezar a medir.
2. **Indicador visible en el agente**: cuando el timer corre y hay captura activa,
   el agente lo refleja (no monitoreo a escondidas).
3. **Acceso del empleado a lo propio**: puede ver su tiempo, su actividad y sus
   capturas (`*.read.self`) — si lo ve él, no es vigilancia opaca.
4. **Autorización de dispositivo**: un equipo nuevo queda `authorizedAt = NULL`
   hasta aprobación; revocar un `Device` corta la captura (doc 01/02).

---

## 7. Cómo el sistema lo HACE CUMPLIR (no solo lo promete)

Cierre con la trazabilidad técnica — privacidad por diseño, verificable:

- **Configuración tipada** (`CompanyTrackingSettings`) → estados válidos, no JSON
  ambiguo.
- **RBAC + Tenant + Ownership** (doc 02) → la visibilidad de §4 es código, no
  confianza.
- **`AuditLog` inmutable** → quién vio/exportó/borró qué queda registrado.
- **Retención por job** → el `retentionDays` se ejecuta, no es una promesa.
- **Límite de captura en el agente** → el "NO keylogging / NO cámara" es ausencia
  de capacidad, no un *flag* que alguien pueda invertir sin un cambio auditado.

---

## Próximo documento

**`04-backend-nestjs-structure.md`** — la organización del backend NestJS que
implementa todo lo anterior: módulos (`controller`/`service`/`repository`/`dto`/
`guards`), dónde viven los 4 guards, el `SyncModule`, el job de retención, el
`ExceptionFilter` global y el mapeo a la forma de error estándar. *(brief §5-7)*
