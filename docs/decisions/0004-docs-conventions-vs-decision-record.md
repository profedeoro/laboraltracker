# 0004 — Convenciones vs. registro de decisión (organización de docs)

- **Status:** Accepted
- **Date:** 2026-06-11

## 🇬🇧 Context
The single design spec grew long. Concern: token cost per session and drift /
hallucination from a large monolithic document loaded wholesale. Content mixed two
natures: one-time decision rationale vs. living rules needed during every coding
session.

## 🇪🇸 Contexto
El spec de diseño único creció. Preocupación: coste de tokens por sesión y deriva /
alucinación por un documento monolítico grande cargado entero. El contenido
mezclaba dos naturalezas: rationale de decisión de una sola vez vs. reglas vivas
necesarias en cada sesión de código.

## 🇬🇧 Decision
Split by **usage type**, not by section count:
- **Living rules** → `docs/conventions/` (one topic per file, loaded on demand).
- **Decision rationale** → the design spec (kept as a record, links to conventions).
- **Invariants (one line each) + links** → root `CLAUDE.md` (auto-loaded, cheap).
- **Why each decision** → `docs/decisions/` (ADRs, bilingual).
- **What changed/when** → root `CHANGELOG.md` (bilingual).
Rule: **single source of truth per concept** — link, never duplicate.

## 🇪🇸 Decisión
Dividir por **tipo de uso**, no por número de secciones:
- **Reglas vivas** → `docs/conventions/` (un tema por archivo, carga bajo demanda).
- **Rationale de decisión** → el spec de diseño (queda como registro, enlaza convenciones).
- **Invariantes (una línea c/u) + enlaces** → `CLAUDE.md` raíz (auto-carga, barato).
- **Por qué de cada decisión** → `docs/decisions/` (ADRs, bilingües).
- **Qué cambió/cuándo** → `CHANGELOG.md` raíz (bilingüe).
Regla: **una sola fuente de verdad por concepto** — enlazar, nunca duplicar.

## 🇬🇧 Consequences
- (+) Implementation tasks load only the relevant slice → fewer tokens, less noise.
- (+) No duplication → no drift between documents.
- (−) More files to navigate; requires discipline to keep links valid.

## 🇪🇸 Consecuencias
- (+) Las tareas de implementación cargan solo la porción relevante → menos tokens, menos ruido.
- (+) Sin duplicación → sin deriva entre documentos.
- (−) Más archivos que navegar; exige disciplina para mantener los enlaces válidos.
