# Architecture Decision Records (ADR) — LaboralTracker

> 🇬🇧 **EN** · 🇪🇸 **ES** — every record is bilingual / cada registro es bilingüe.

## 🇬🇧 What is this folder?

An **ADR** documents one significant decision: the **context** (why it came up),
the **decision** taken, and its **consequences** (trade-offs, what it enables,
what it costs). It answers *"why is it like this, and what is it for?"* — the
knowledge that is normally lost between people's heads and git history.

- One file per decision: `NNNN-short-title.md` (sequential, never renumbered).
- A decision is **immutable** once `Accepted`. To change it, write a **new** ADR
  that supersedes the old one (and mark the old one `Superseded by NNNN`).
- For the chronological *"what changed"* log, see the root
  [`CHANGELOG.md`](../../CHANGELOG.md). ADR = *why*; CHANGELOG = *what/when*.

## 🇪🇸 ¿Qué es esta carpeta?

Un **ADR** documenta una decisión importante: el **contexto** (por qué surgió),
la **decisión** tomada y sus **consecuencias** (trade-offs, qué habilita, qué
cuesta). Responde *"¿por qué es así y para qué sirve?"* — el conocimiento que
normalmente se pierde entre las cabezas del equipo y el historial de git.

- Un archivo por decisión: `NNNN-titulo-corto.md` (secuencial, nunca se renumera).
- Una decisión es **inmutable** una vez `Aceptada`. Para cambiarla, se escribe un
  **nuevo** ADR que la reemplaza (y el viejo se marca `Reemplazado por NNNN`).
- Para el registro cronológico de *"qué cambió"*, ver
  [`CHANGELOG.md`](../../CHANGELOG.md) en la raíz. ADR = *por qué*; CHANGELOG = *qué/cuándo*.

## Estados / Status

`Proposed` · `Accepted` · `Superseded by NNNN` · `Deprecated`

## Índice / Index

| # | Título / Title | Estado / Status |
|---|----------------|-----------------|
| [0001](0001-stack-tauri-rust-svelte.md) | Stack: Tauri (Rust + Svelte/TS) | Accepted |
| [0002](0002-hexagonal-architecture-solid.md) | Arquitectura hexagonal + SOLID (didáctica) | Accepted |
| [0003](0003-time-and-schema-foundations.md) | Fundaciones de tiempo y esquema SQLite | Accepted |
| [0004](0004-docs-conventions-vs-decision-record.md) | Convenciones vs. registro de decisión | Accepted |
| [0005](0005-id-strategy-local-to-cloud.md) | Estrategia de IDs (local → nube) | Proposed |

## Plantilla / Template

```md
# NNNN — <título / title>
- **Status:** Proposed | Accepted | Superseded by … | Deprecated
- **Date:** YYYY-MM-DD

## 🇬🇧 Context / 🇪🇸 Contexto
## 🇬🇧 Decision / 🇪🇸 Decisión
## 🇬🇧 Consequences / 🇪🇸 Consecuencias
```
