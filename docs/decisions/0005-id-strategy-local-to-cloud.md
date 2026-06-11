# 0005 — Estrategia de IDs (local → nube)

- **Status:** Accepted (Opción A — ULID texto)
- **Date:** 2026-06-11

## 🇬🇧 Context
The schema uses `INTEGER PRIMARY KEY` (autoincrement rowid) for `project`, `task`,
`time_session`. The whole spec (§6/§11) is honest that the local→team/cloud jump is
"the hard part." Sequential integer IDs are precisely what makes that jump painful:
two offline clients both mint `id = 1`, so on sync you get collisions and must
rewrite every row and foreign key in an ID-remapping migration. This is a
cheap-now / expensive-later decision that the cloud-honesty narrative currently
leaves silent. The PK type lives in migration `0001`, so it must be decided before
Plan 1 freezes that migration.

## 🇪🇸 Contexto
El esquema usa `INTEGER PRIMARY KEY` (rowid autoincremental) en `project`, `task`,
`time_session`. Todo el spec (§6/§11) es honesto en que el salto local→equipo/nube
es "lo difícil". Los IDs enteros secuenciales son justo lo que vuelve ese salto
doloroso: dos clientes offline generan `id = 1`, y al sincronizar hay colisión y
hay que reescribir cada fila y clave foránea en una migración de remapeo de IDs. Es
una decisión barata-ahora / cara-después que el discurso de la nube hoy deja en
silencio. El tipo de PK vive en la migración `0001`, así que debe decidirse antes de
que Plan 1 la congele.

## 🇬🇧 Options
- **A) ULID/UUID text PK now (recommended).** PKs become `TEXT` holding a ULID
  (sortable, time-prefixed) generated in Rust (`ulid` crate). Globally unique → no
  collision on future sync; no ID-remapping migration ever. Cost: text PKs (slightly
  larger indexes), generate IDs in code instead of relying on rowid.
- **B) Keep `INTEGER` autoincrement, defer.** Simplest now. Accept that the cloud
  sub-project will require an ID-remapping migration (rewrite all PKs/FKs) or a
  parallel `uuid` column added later. Documented debt, not silent.
- **C) Hybrid: keep rowid + add a `uuid TEXT UNIQUE` column now.** Local code uses
  rowid; sync uses the uuid. Middle cost; two identities to keep consistent.

## 🇪🇸 Opciones
- **A) PK de texto ULID/UUID ahora (recomendada).** Las PK pasan a `TEXT` con un
  ULID (ordenable, con prefijo temporal) generado en Rust (crate `ulid`).
  Globalmente único → sin colisión en la sync futura; nunca una migración de remapeo.
  Coste: PK de texto (índices algo mayores), generar IDs en código en vez de rowid.
- **B) Mantener `INTEGER` autoincremental, diferir.** Lo más simple hoy. Se acepta
  que el sub-proyecto de nube exigirá una migración de remapeo (reescribir PK/FK) o
  una columna `uuid` paralela añadida después. Deuda documentada, no silenciosa.
- **C) Híbrido: rowid + columna `uuid TEXT UNIQUE` ahora.** El código local usa
  rowid; la sync usa el uuid. Coste medio; dos identidades que mantener coherentes.

## 🇬🇧 Decision / 🇪🇸 Decisión
**Option A — ULID text PK.** PKs are `TEXT` holding a ULID generated in the Rust
domain via the `ulid` crate; foreign keys are `TEXT`. Chosen because the project's
explicit goal is team/cloud and A removes a known, expensive future ID-remapping
migration at near-zero cost today. / **Opción A — PK ULID de texto.** Las PK son
`TEXT` con un ULID generado en el dominio Rust con el crate `ulid`; las claves
foráneas son `TEXT`. Elegida porque la meta declarada es equipo/nube y A elimina una
migración futura de remapeo, conocida y cara, a coste casi nulo hoy.

## 🇬🇧 Consequences / 🇪🇸 Consecuencias
- If A: update `05-data-schema.md` (PKs → `TEXT`, IDs generated in domain via `ulid`),
  and Plan 1's migration/tests before building. / Si A: actualizar
  `05-data-schema.md` (PK → `TEXT`, IDs generados en dominio con `ulid`) y la
  migración/tests de Plan 1 antes de construir.
- If B/C: record the deferred migration explicitly so it is not a surprise later. /
  Si B/C: registrar la migración diferida explícitamente para que no sorprenda luego.
