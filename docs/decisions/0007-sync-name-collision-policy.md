# 0007 — Política de colisión de nombres en la sincronización

- **Status:** Accepted
- **Date:** 2026-06-19

## 🇬🇧 Context
`Project` and `Task` carry a business-unique name: `@@unique([companyId, name])`
and `@@unique([projectId, name])` (doc 01). They sync agent→platform **by ULID
(`id`)**, never by name — the id the agent mints locally *is* the id in the cloud
(ADR 0005), so sync is an idempotent `upsert` by `id`.

A name collision therefore arises only when **two distinct ULIDs share the same
name** within the same scope. Concretely: an employee creates "Proyecto X" offline
in the agent while a manager creates "Proyecto X" on the web — two different ids,
same name, same company. On sync, the second one violates the unique index.

The agent's local SQLite **currently has no such unique constraint**
(`conventions/05-data-schema.md`), so today it cannot detect the clash before
pushing. This was flagged in doc 01 §3.8 and needs a defined, reversible policy:
`Project`/`Task` have attached `TimeEntry` history, so the resolution must never
silently corrupt or irreversibly mix data.

## 🇪🇸 Contexto
`Project` y `Task` tienen un nombre único de negocio: `@@unique([companyId, name])`
y `@@unique([projectId, name])` (doc 01). Sincronizan agente→plataforma **por ULID
(`id`)**, nunca por nombre — el id que el agente genera local *es* el id en la nube
(ADR 0005), así que la sync es un `upsert` idempotente por `id`.

Una colisión de nombre solo aparece cuando **dos ULIDs distintos comparten el mismo
nombre** en el mismo alcance. Concreto: un empleado crea "Proyecto X" offline en el
agente mientras un manager crea "Proyecto X" en la web — dos ids distintos, mismo
nombre, misma empresa. Al sincronizar, el segundo viola el índice único.

El SQLite local del agente **hoy no tiene esa restricción única**
(`conventions/05-data-schema.md`), así que no puede detectar el choque antes de
empujar. Se marcó en doc 01 §3.8 y necesita una política definida y reversible:
`Project`/`Task` tienen historial de `TimeEntry` asociado, así que la resolución
nunca debe corromper en silencio ni mezclar datos de forma irreversible.

## 🇬🇧 Options / 🇪🇸 Opciones
- **A) Reject at item level (recommended).** The conflicting item comes back in
  `conflicts[]` with `reason: "NAME_TAKEN"` (HTTP 409 semantics); the rest of the
  batch is accepted. The user renames — a **human** decision. The agent mirrors the
  unique index to catch the clash locally before syncing. / **Rechazo a nivel de
  ítem (recomendada).** El ítem en conflicto vuelve en `conflicts[]` con `reason:
  "NAME_TAKEN"` (semántica HTTP 409); el resto del lote se acepta. El usuario
  renombra — decisión **humana**. El agente espeja el índice único para detectar el
  choque localmente antes de sincronizar.
- **B) Auto-rename ("Proyecto X (2)").** Silent and surprising: it corrupts the name
  the user chose without anyone asking. / **Auto-renombrar ("Proyecto X (2)").**
  Silencioso y sorpresivo: corrompe el nombre que eligió el usuario sin que nadie lo
  pida.
- **C) Auto-merge by name.** Two distinct ids may each own different `TimeEntry`
  rows; merging them mixes time history irreversibly. Unacceptable in a labor app. /
  **Auto-mergear por nombre.** Dos ids distintos pueden tener `TimeEntry` distintos;
  fusionarlos mezcla historial de tiempo de forma irreversible. Inaceptable en una
  app laboral.

## 🇬🇧 Decision / 🇪🇸 Decisión
**Option A — reject at item level, no rename, no merge.** The conflict surfaces to
the human who owns the name. One bad item never sinks the batch: the sync endpoint
returns a partial result (`accepted` / `conflicts` / `errors`, doc 02 §3.3). The
agent must **mirror** the uniqueness as a **partial unique index** (over
non-archived / non-deleted rows) in its SQLite, so the clash is detected and
resolved locally *before* the push. / **Opción A — rechazo a nivel de ítem, sin
renombrar, sin mergear.** El conflicto se eleva al humano dueño del nombre. Un ítem
malo nunca tumba el lote: el endpoint de sync devuelve resultado parcial (`accepted`
/ `conflicts` / `errors`, doc 02 §3.3). El agente debe **espejar** la unicidad como
**índice único parcial** (sobre filas no archivadas / no borradas) en su SQLite,
para detectar y resolver el choque localmente *antes* de empujar.

**Direction of truth (future):** projects/tasks will naturally be **born on the
platform** (manager) and **pulled** by the agent, shrinking collisions to the
offline-creation edge case — which is exactly what policy A covers. / **Dirección de
verdad (futuro):** lo natural es que proyectos/tareas **nazcan en la plataforma**
(manager) y el agente los **baje** (pull), reduciendo las colisiones al caso de
borde de creación offline — justo lo que cubre la política A.

## 🇬🇧 Consequences / 🇪🇸 Consecuencias
- The sync endpoint returns a per-item partial result; conflicts use `NAME_TAKEN`
  with HTTP 409 in the error taxonomy (doc 02 §3.3, §7). / El endpoint de sync
  devuelve resultado parcial por ítem; los conflictos usan `NAME_TAKEN` con HTTP 409
  en la taxonomía de errores (doc 02 §3.3, §7).
- **Agent backlog (not in Plans 1-4):** add the partial unique index on the agent's
  SQLite so it detects clashes before sync. Tracked alongside the deferred agent
  hardening items. / **Backlog del agente (fuera de Plans 1-4):** agregar el índice
  único parcial en el SQLite del agente para detectar choques antes de sincronizar.
  Se rastrea junto a los pendientes diferidos de hardening del agente.
- Trade-off accepted: the user must take an action (rename) to resolve a conflict.
  We choose that friction over silent corruption or irreversible history mixing. /
  Trade-off aceptado: el usuario debe actuar (renombrar) para resolver el conflicto.
  Elegimos esa fricción antes que la corrupción silenciosa o la mezcla irreversible
  de historial.
- This ADR formalizes the decision already drafted in doc 02 §3.4 and closes the
  flag in doc 01 §3.8. / Este ADR formaliza la decisión ya esbozada en doc 02 §3.4 y
  cierra el flag de doc 01 §3.8.
