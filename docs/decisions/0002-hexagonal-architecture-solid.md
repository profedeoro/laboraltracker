# 0002 — Arquitectura hexagonal + SOLID (didáctica)

- **Status:** Accepted
- **Date:** 2026-06-10

## 🇬🇧 Context
The raw MVP A logic is three rules (non-empty name, duration = end − start, one
active session). A code review flagged that a full 4-layer hexagonal architecture
is over-engineering for that. But the user explicitly requested SOLID architecture
and chose the learning path.

## 🇪🇸 Contexto
La lógica cruda del MVP A son tres reglas (nombre no vacío, duración = fin − inicio,
una sola sesión activa). Una revisión de código señaló que una arquitectura
hexagonal de 4 capas es sobreingeniería para eso. Pero el usuario pidió
explícitamente arquitectura SOLID y eligió el camino de aprendizaje.

## 🇬🇧 Decision
Adopt **Clean/Hexagonal** (domain / application / infrastructure / presentation)
with ports & adapters and a composition root, validated against **SOLID**. Keep it
**despite** being oversized for the current functionality, and **name that
honestly** as a deliberate didactic choice. Use the "one sub-project at a time"
process to also **prune** abstractions, not only add them.

## 🇪🇸 Decisión
Adoptar **Clean/Hexagonal** (dominio / aplicación / infraestructura / presentación)
con puertos y adaptadores y un *composition root*, validada contra **SOLID**.
Mantenerla **aunque** esté sobredimensionada para la funcionalidad actual, y
**nombrarlo con honestidad** como decisión didáctica deliberada. Usar el proceso
"un sub-proyecto a la vez" también para **podar** abstracciones, no solo sumarlas.

## 🇬🇧 Consequences
- (+) Real learning of layering, DIP, ports/adapters, test doubles.
- (+) Domain stays pure → 100% unit-testable; future persistence swap is isolated.
- (−) More upfront code (use-case structs, traits, in-memory + SQLite impls).
- Detail lives in [`docs/conventions/01-architecture-solid.md`](../conventions/01-architecture-solid.md).

## 🇪🇸 Consecuencias
- (+) Aprendizaje real de capas, DIP, puertos/adaptadores, dobles de prueba.
- (+) Dominio puro → 100% testeable; el cambio futuro de persistencia queda aislado.
- (−) Más código inicial (structs de caso de uso, traits, impls in-memory + SQLite).
- El detalle vive en [`docs/conventions/01-architecture-solid.md`](../conventions/01-architecture-solid.md).
