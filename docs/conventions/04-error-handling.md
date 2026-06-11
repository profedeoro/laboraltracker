# 04 — Manejo de errores y contrato FE↔BE

> Carga este doc cuando definas o propagues errores.

## Errores en Rust

- Errores de dominio/aplicación como **enums** con `thiserror`
  (`ProjectNameEmpty`, `TaskNotFound`, `SessionNotFound`, `NoRunningSession`,
  `ClockWentBackwards`, `Internal`…).
- Casos de uso devuelven `Result<T, AppError>`.
- **Prohibido** `unwrap()`/`panic!` en flujo normal. `expect` solo en el
  composition root para fallos de arranque irrecuperables (p. ej. no abrir la BD).

## App local mono-usuario: logging vs. ocultación

- El usuario es el dueño de la máquina: **se loguea el error completo** (ocultar
  detalle solo dificulta el debug) y se devuelve al frontend un error
  **serializable y controlado**.
- La ocultación estricta de detalle interno se reserva para cuando exista una
  **frontera de confianza real** (la futura capa nube/multi-tenant), no ahora.
  No aplicar a ciegas patrones de seguridad web a una app local.

## Contrato FE↔BE tipado (anti-deriva)

- Los DTOs y el tipo de error se **generan a TypeScript desde Rust** (`ts-rs`),
  para que el contrato de `invoke` no pueda divergir en silencio. Un campo
  renombrado en Rust debe romper la compilación del front, no el runtime.
- Al menos un test delgado de la **costura** (un comando real con DTOs/errores
  serializados) además de `cargo test` y `vitest`.
