# 0009 — Two-name seam: agent `TimeSession` ⇄ platform `TimeEntry`

- **Status:** Accepted
- **Date:** 2026-06-19

## 🇬🇧 Context
The same logical row is named differently on each half of the product: the desktop
**agent** (shipped, Plans 1-4) calls it `TimeSession` (Rust domain) / `time_session`
(SQLite table); the **platform** (design stage) names the Prisma model `TimeEntry`.
A technical review flagged this as an inconsistency — "it isn't an *entry*, it's a
*session*" — and asked whether to rename so both halves speak one noun.

Investigating the schema (not just the prose) shows the two names describe two
genuinely different concepts in two bounded contexts:

- On the **agent**, every `time_session` row **is** a measured stopwatch run. SQLite
  enforces it: single global active session (partial unique index), clock-backwards
  guard, heartbeat anchor, `is_suspect`. `TimeSession` is true there.
- On the **platform**, the same row carries `source EntrySource @default(AGENT)` with
  `AGENT | MANUAL` (doc 01). A `MANUAL` row has **no stopwatch behind it** — a human
  typed `startedAt`/`endedAt` on a form: no heartbeat, no device, `isSuspect`
  meaningless. The single-active-session invariant does **not** exist on the platform
  (no partial unique index on the Prisma model). The platform row is a **superset**.

So "session" would *lie* about a `MANUAL` row, and `EntrySource` is clearly built to
grow (`IMPORTED`/`EDITED` tomorrow). `TimeEntry` is the industry-standard term too
(Toggl/Harvest/Clockify expose `time_entries`, not "sessions").

The tension is real: the monorepo's decisive justification (doc 06) is **one shared
agent↔platform sync contract**, and a two-name seam seems to undercut it. The
resolution turns on what actually *binds* the contract.

## 🇪🇸 Contexto
La misma fila lógica se llama distinto en cada mitad del producto: el **agente** de
escritorio (ya construido, Plans 1-4) la llama `TimeSession` (dominio Rust) /
`time_session` (tabla SQLite); la **plataforma** (en diseño) nombra al modelo Prisma
`TimeEntry`. Una revisión técnica lo marcó como inconsistencia — "no es una *entrada*,
es una *sesión*" — y preguntó si renombrar para que ambas mitades usen un solo
sustantivo.

Investigar el schema (no solo la prosa) muestra que los dos nombres describen dos
conceptos genuinamente distintos en dos contextos acotados:

- En el **agente**, cada fila `time_session` **es** una corrida de cronómetro medida.
  SQLite lo hace cumplir: una sola sesión activa global (índice único parcial), guarda
  contra reloj retrocedido, ancla de heartbeat, `is_suspect`. Ahí `TimeSession` es
  verdadero.
- En la **plataforma**, la misma fila lleva `source EntrySource @default(AGENT)` con
  `AGENT | MANUAL` (doc 01). Una fila `MANUAL` **no tiene cronómetro detrás** — un
  humano tipeó `startedAt`/`endedAt` en un form: sin heartbeat, sin device,
  `isSuspect` sin sentido. El invariante de sesión única **no existe** en la
  plataforma (no hay índice único parcial en el modelo Prisma). La fila de plataforma
  es un **superconjunto**.

Entonces "session" *mentiría* sobre una fila `MANUAL`, y `EntrySource` está hecho para
crecer (`IMPORTED`/`EDITED` mañana). `TimeEntry` es además el término estándar de la
industria (Toggl/Harvest/Clockify exponen `time_entries`, no "sessions").

La tensión es real: la justificación decisiva del monorepo (doc 06) es **un contrato
de sync compartido** agente↔plataforma, y un seam de dos nombres parece socavarlo. La
resolución depende de qué *ata* realmente el contrato.

## 🇬🇧 Decision / 🇪🇸 Decisión
**Keep the two names on purpose. The agent stays `TimeSession`/`time_session`; the
platform stays `TimeEntry`. No rename, and no third name (`WorkInterval` would desync
*both* halves to fix a mismatch that exists on only one).** Each half names its concept
honestly within its own bounded context. / **Mantener los dos nombres a propósito. El
agente queda `TimeSession`/`time_session`; la plataforma queda `TimeEntry`. Sin
renombrar, y sin un tercer nombre (`WorkInterval` desincronizaría *ambas* mitades para
arreglar un mismatch que existe en una sola).** Cada mitad nombra su concepto
honestamente dentro de su propio contexto acotado.

The decisive reason: what binds the sync contract is the **shared ULID** — ADR 0005
makes the id both the identity and the idempotency key (`upsert` by id). The join
syncs **by id, never by type name**, so divergent names cost nothing in correctness.
Bending `MANUAL` into a "degenerate session" to force one name would bend the *data*
to fit the *name*; the correct direction is the opposite. / La razón decisiva: lo que
ata el contrato de sync es el **ULID compartido** — el ADR 0005 hace del id a la vez la
identidad y la idempotency key (`upsert` por id). El join sincroniza **por id, nunca
por el nombre del tipo**, así que nombres divergentes no cuestan nada en correctitud.
Doblar `MANUAL` en una "sesión degenerada" para forzar un solo nombre doblaría el
*dato* para que entre en el *nombre*; la dirección correcta es la inversa.

**Condition of the decision:** a deliberate seam that is not documented is
indistinguishable from accidental drift. So the seam **must be made explicit** (see
Consequences). / **Condición de la decisión:** un seam deliberado que no se documenta
es indistinguible de un drift accidental. Por eso el seam **debe hacerse explícito**
(ver Consecuencias).

## 🇬🇧 Consequences / 🇪🇸 Consecuencias
- **Zero code migration.** The agent (shipped: `time_session`, `TimeSession`,
  generated `TimeSessionDto.ts`) is untouched; the platform keeps `TimeEntry` and the
  `POST /sync/time-entries` route. The shared ULID / sync contract is unaffected. /
  **Cero migración de código.** El agente (construido: `time_session`, `TimeSession`,
  `TimeSessionDto.ts` generado) queda intacto; la plataforma mantiene `TimeEntry` y la
  ruta `POST /sync/time-entries`. El ULID compartido / contrato de sync no se afecta.
- **Mandatory: make the seam explicit** (the docs currently leak ambiguity by glossing
  `TimeEntry` as "el `time_session` del agente"). Done as part of this decision:
  - doc 01 (`TimeEntry` docstring): a consolidated agent `time_session` is **one
    source** (`source=AGENT`) of a `TimeEntry`; the platform also holds `source=MANUAL`
    rows with no agent session behind them.
  - doc 00 (sync flow): `agent.time_session (source=AGENT) ⇒ platform.TimeEntry`, upsert
    by shared ULID (ADR 0005). **The id is the contract; the type name is not.**
  / **Obligatorio: hacer explícito el seam** (los docs hoy filtran ambigüedad al
  glosar `TimeEntry` como "el `time_session` del agente"). Hecho como parte de esta
  decisión:
  - doc 01 (docstring de `TimeEntry`): un `time_session` del agente consolidado es
    **una fuente** (`source=AGENT`) de un `TimeEntry`; la plataforma también guarda
    filas `source=MANUAL` sin sesión del agente detrás.
  - doc 00 (flujo de sync): `agent.time_session (source=AGENT) ⇒ platform.TimeEntry`,
    upsert por ULID compartido (ADR 0005). **El id es el contrato; el nombre del tipo
    no.**
- **Recommended:** name the wire DTO for what it is — the sync payload is a
  `TimeSessionSyncDto` that *upserts* a `TimeEntry` — so the boundary is named, not
  hidden. Land it when the backend scaffold defines the sync DTOs (doc 04). /
  **Recomendado:** nombrar el DTO de transporte por lo que es — el payload de sync es
  un `TimeSessionSyncDto` que *upserta* un `TimeEntry` — para que el borde quede
  nombrado, no escondido. Aterrizarlo cuando el scaffold del backend defina los DTOs de
  sync (doc 04).
- **Cost accepted:** a permanent cognitive seam at the most-trafficked row — a reader
  tracing a ULID from agent SQLite to the Postgres row remaps the name once. Bounded,
  and justified: the name marks a real context boundary (measurement device vs
  consolidated, multi-source ledger). / **Costo aceptado:** un seam cognitivo
  permanente en la fila más transitada — quien siga un ULID del SQLite del agente a la
  fila Postgres remapea el nombre una vez. Acotado y justificado: el nombre marca un
  límite de contexto real (dispositivo de medición vs libro consolidado multi-fuente).
- Closes the naming question (review point 5a) deferred from the second-review ADR
  batch (0006-0008). / Cierra la cuestión de naming (punto 5a de la revisión) diferida
  del lote de ADRs de la segunda revisión (0006-0008).
