# Changelog — LaboralTracker

> 🇬🇧 All notable changes to this project, bilingual. Format based on
> [Keep a Changelog](https://keepachangelog.com/); versioning will follow
> [SemVer](https://semver.org/) once the first release ships.
> The *why* of each decision lives in [`docs/decisions/`](docs/decisions/) (ADRs).
>
> 🇪🇸 Todos los cambios notables del proyecto, bilingüe. Formato basado en
> [Keep a Changelog](https://keepachangelog.com/); el versionado seguirá
> [SemVer](https://semver.org/) desde el primer release.
> El *porqué* de cada decisión vive en [`docs/decisions/`](docs/decisions/) (ADRs).

## [Unreleased]

### 🇬🇧 Added / 🇪🇸 Añadido
- **Design spec for MVP A** (local time-tracking core). /
  **Spec de diseño del MVP A** (núcleo local de *time tracking*).
  → `docs/superpowers/specs/2026-06-10-laboraltracker-mvp-a-design.md`
- **Living conventions** (architecture/SOLID, time policy, concurrency, errors,
  SQLite schema), one topic per file, loaded on demand. /
  **Convenciones vivas** (arquitectura/SOLID, política de tiempo, concurrencia,
  errores, esquema SQLite), un tema por archivo, carga bajo demanda.
  → `docs/conventions/`
- **Thin root `CLAUDE.md`**: project invariants (one line each) + links. /
  **`CLAUDE.md` raíz delgado**: invariantes del proyecto (una línea c/u) + enlaces.
- **ADR folder** with retroactive records 0001–0004 (bilingual). /
  **Carpeta de ADRs** con registros retroactivos 0001–0004 (bilingüe).
  → `docs/decisions/`
- **This bilingual CHANGELOG.** / **Este CHANGELOG bilingüe.**

### 🇬🇧 Changed / 🇪🇸 Cambiado
- **Spec foundations hardened** after technical review: UTC epoch-millis time
  policy, midnight overlap, orphan-session recovery, partial unique index for the
  single-active-session invariant, `Mutex<Connection>` concurrency model, honest
  cloud-migration limit, typed FE↔BE contract generated from Rust. /
  **Fundaciones del spec endurecidas** tras revisión técnica: política de tiempo
  epoch-millis UTC, solapamiento en medianoche, recuperación de sesión huérfana,
  índice único parcial para el invariante de sesión única, modelo de concurrencia
  `Mutex<Connection>`, límite honesto del salto a la nube, contrato FE↔BE tipado
  generado desde Rust.
  → ADR [0003](docs/decisions/0003-time-and-schema-foundations.md)
- **Documentation reorganised** from one monolithic spec into decision-record +
  on-demand conventions, to cut token cost and document drift. /
  **Documentación reorganizada** de un spec monolítico a registro-de-decisión +
  convenciones bajo demanda, para reducir coste de tokens y deriva.
  → ADR [0004](docs/decisions/0004-docs-conventions-vs-decision-record.md)

### 🇬🇧 Decided / 🇪🇸 Decidido
- **Stack:** Tauri (Rust + Svelte/TS) over Python/PySide6. /
  **Stack:** Tauri (Rust + Svelte/TS) sobre Python/PySide6.
  → ADR [0001](docs/decisions/0001-stack-tauri-rust-svelte.md)
- **Architecture:** Clean/Hexagonal + SOLID (didactic, deliberately oversized). /
  **Arquitectura:** Clean/Hexagonal + SOLID (didáctica, sobredimensionada a propósito).
  → ADR [0002](docs/decisions/0002-hexagonal-architecture-solid.md)

---

### 🇬🇧 How to use / 🇪🇸 Cómo se usa
- Add entries under **[Unreleased]** as work happens; group by Added / Changed /
  Fixed / Removed / Decided. On release, rename the section to the version + date. /
  Añade entradas bajo **[Unreleased]** según avanza el trabajo; agrupa por
  Añadido / Cambiado / Corregido / Eliminado / Decidido. Al hacer release, renombra
  la sección a la versión + fecha.
- Each entry stays one line of *what*; the *why* goes to an ADR and is linked. /
  Cada entrada es una línea de *qué*; el *porqué* va a un ADR y se enlaza.
