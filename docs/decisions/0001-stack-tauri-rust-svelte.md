# 0001 — Stack: Tauri (Rust + Svelte/TypeScript)

- **Status:** Accepted
- **Date:** 2026-06-10

## 🇬🇧 Context
We are building LaboralTracker, a Time Doctor–style desktop time tracker. The
hard part of such a tool is OS integration (active window, idle time, screenshots
later), not the UI. The team works mostly in Python and has a mature `python.md`
ruleset. Candidates: (A) Python + PySide6, (C) Tauri (Rust core + web frontend).

## 🇪🇸 Contexto
Construimos LaboralTracker, un *time tracker* de escritorio estilo Time Doctor. Lo
difícil es la integración con el SO (ventana activa, *idle*, capturas a futuro), no
la UI. El equipo trabaja sobre todo en Python y tiene reglas `python.md` maduras.
Candidatos: (A) Python + PySide6, (C) Tauri (núcleo Rust + frontend web).

## 🇬🇧 Decision
Use **Tauri 2.x**: business core in **Rust**, UI in **Svelte + TypeScript**,
persistence in **SQLite** (`rusqlite`). Chosen over Python/PySide6 **prioritising
learning the modern stack and architectural discipline**, accepting that no
`rust.md`/`ts.md` ruleset exists yet (we write our own conventions).

## 🇪🇸 Decisión
Usar **Tauri 2.x**: núcleo de negocio en **Rust**, UI en **Svelte + TypeScript**,
persistencia en **SQLite** (`rusqlite`). Elegido sobre Python/PySide6
**priorizando el aprendizaje del stack moderno y la disciplina arquitectónica**,
asumiendo que no existe aún un ruleset `rust.md`/`ts.md` (escribimos el nuestro).

## 🇬🇧 Consequences
- (+) One ecosystem learned deeply; pure, testable Rust domain.
- (+) Lightweight app (system WebView, no bundled Chromium like Electron).
- (−) Three worlds to maintain: Rust + TypeScript + web tooling.
- (−) We must author Rust/TS conventions from scratch (done in `docs/conventions/`).
- We write **idiomatic Rust**, not transliterated Python.

## 🇪🇸 Consecuencias
- (+) Un ecosistema aprendido a fondo; dominio Rust puro y testeable.
- (+) App ligera (WebView del sistema, sin Chromium embebido como Electron).
- (−) Tres mundos que mantener: Rust + TypeScript + web tooling.
- (−) Redactamos las convenciones Rust/TS desde cero (hecho en `docs/conventions/`).
- Escribimos **Rust idiomático**, no Python transliterado.
