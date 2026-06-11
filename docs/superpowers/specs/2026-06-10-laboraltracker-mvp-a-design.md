# LaboralTracker — Diseño MVP A (Cronómetro con proyectos)

- **Fecha:** 2026-06-10
- **Estado:** Aprobado para implementación
- **Sub-proyecto:** Núcleo local de *time tracking* (primer incremento de un clon estilo Time Doctor)

---

## 1. Contexto y visión

LaboralTracker será, a futuro, una herramienta de seguimiento de tiempo y
productividad para **equipos/empresa** (estilo Time Doctor). Se construye de
forma **incremental**: un sub-proyecto bien hecho a la vez, cada uno con su
propio ciclo spec → plan → código → test → commit.

Este documento define el **primer sub-proyecto: el núcleo local de time
tracking**. Es el cimiento del que colgarán después el monitoreo automático de
actividad, captura de apps/URLs, capturas de pantalla, clasificación de
productividad, reportes y la capa de nube/equipo.

### Marco ético/legal (no negociable)

El producto final es monitoreo **consentido y transparente**: el usuario sabe
que se le rastrea, puede pausar y ve sus propios datos. No se construirá
vigilancia encubierta (stalkerware). En este MVP local esto es trivial (el
usuario rastrea su propio tiempo manualmente), pero la decisión queda registrada
para los sub-proyectos futuros.

---

## 2. Stack tecnológico

| Capa            | Tecnología                                   |
|-----------------|----------------------------------------------|
| Empaquetado     | **Tauri 2.x** (app de escritorio)            |
| Núcleo/dominio  | **Rust** (lógica de negocio pura, testeable) |
| Frontend/UI     | **Svelte + TypeScript**                      |
| Persistencia    | **SQLite local** vía `rusqlite`              |
| Errores (Rust)  | `thiserror`                                  |
| Tests Rust      | `cargo test`                                 |
| Tests frontend  | `vitest`                                     |

**Decisión registrada:** se eligió Tauri (Rust + TS) sobre Python/PySide6
priorizando aprendizaje del stack moderno. Como no existe un `rust.md`/`ts.md`
en las reglas globales, se traduce el *espíritu* de `~/.claude/rules/python.md`
(capas, tipado estricto, validación en bordes, errores explícitos, tests,
seguridad) a convenciones propias de Rust/TS, documentadas en el `CLAUDE.md` del
proyecto.

**Decisión:** se usa `rusqlite` directamente en Rust, **no** `tauri-plugin-sql`,
para no filtrar SQL al frontend y mantener la separación de capas.

---

## 3. Alcance del MVP A

### Dentro de alcance
- Crear / listar / archivar **proyectos**.
- Crear / listar tareas dentro de un proyecto.
- Iniciar y parar un **cronómetro** sobre una tarea.
- Regla: **una sola sesión corriendo a la vez** (iniciar una nueva detiene la activa).
- Ver el **tiempo registrado hoy** (por tarea/proyecto).
- Persistencia local en SQLite.

### Fuera de alcance (YAGNI — sub-proyectos posteriores)
- Monitoreo automático de apps/ventana activa e *idle*.
- Capturas de pantalla.
- Clasificación productivo/improductivo.
- Reportes avanzados, exportación.
- Nube, cuentas, login, roles, multi-usuario.

---

## 4. Arquitectura — Clean / Hexagonal (puertos y adaptadores)

Cuatro capas en `src-tauri/` (Rust) con la **regla de dependencia** apuntando
hacia el dominio (DIP):

```txt
Frontend (Svelte/TS)                 ← Presentación (UI)
   │  invoke()  ── comandos Tauri
src-tauri/ (Rust)
 ├─ presentation/   handlers de comandos Tauri (finos) + DTOs
 ├─ application/    casos de uso (StartTimer, StopTimer, CreateProject…)
 ├─ domain/         entidades, value objects, invariantes, PUERTOS (traits)
 └─ infrastructure/ adaptadores: repositorios SQLite (implementan los traits)
```

- **`domain`**: no depende de nada externo (ni SQLite ni Tauri). Reglas e
  invariantes puras → 100% testeable.
- **`application`**: depende solo de los *traits* del dominio (puertos).
- **`infrastructure`**: implementa esos traits con `rusqlite`.
- **`presentation`**: traduce comando Tauri ↔ caso de uso. Sin lógica de negocio.
- **`lib.rs`** (*composition root*): único lugar que conoce las implementaciones
  concretas; arma la inyección de dependencias y registra los comandos.

```txt
presentation ──► application ──► domain (traits/puertos) ◄── infrastructure
                                   ▲
                  las flechas apuntan a lo ABSTRACTO (DIP)
   composition root (lib.rs) inyecta los adaptadores concretos
```

### Validación SOLID (guía viva)

- **SRP:** una razón de cambio por pieza. Cada caso de uso = un struct con un
  método (`StartTimerUseCase`, `StopTimerUseCase`…). Entidades solo sus
  invariantes; repos solo persistencia; handlers solo traducción.
- **OCP:** añadir un `PostgresSessionRepository` (futura nube) no toca dominio ni
  casos de uso; nuevos casos de uso se agregan, no se edita un servicio gigante.
- **LSP:** `SqliteSessionRepository` e `InMemorySessionRepository` (tests) son
  intercambiables bajo el mismo contrato del trait.
- **ISP:** puertos pequeños y específicos (`ProjectRepository`,
  `TaskRepository`, `SessionRepository`, `Clock`), no un repositorio-Dios.
- **DIP:** `application` depende de traits, no de `rusqlite`; solo el
  composition root conoce los detalles concretos.

---

## 5. Modelo de dominio

```txt
Project (id, name, color?, created_at, archived)
   └── Task (id, project_id, name, created_at, completed)
          └── TimeSession (id, task_id, started_at, ended_at?)
```

- **Project** — agrupa tareas. Invariante: `name` no vacío.
- **Task** — pertenece a un proyecto. Invariante: `name` no vacío.
- **TimeSession** — intervalo de tiempo. `ended_at = None` ⇒ **corriendo**.
  Duración **derivada** (`ended_at − started_at`), no se almacena redundante.
- **Invariante clave:** solo una `TimeSession` corriendo a la vez. Vive en un
  servicio de dominio (`TimeTracker`), no en la UI.

### Puertos (traits del dominio)

```txt
ProjectRepository   add / list / find_by_id / archive
TaskRepository      add / list_by_project / find_by_id
SessionRepository   add / save / find_running_session / list_for_day
Clock               now() -> timestamp   (inyectable para tests)
```

---

## 6. Flujo de datos (ejemplo: iniciar cronómetro)

```txt
[UI] clic "Start" en una tarea
  → invoke('start_timer', { taskId })
  → [presentation] start_timer handler
  → [application] StartTimerUseCase.execute(taskId)
        · repo.find_running_session() → si existe, la cierra (ended_at = clock.now())
        · crea TimeSession(started_at = clock.now())
        · repo.add(session)
  → devuelve DTO { sessionId, startedAt }
  → [UI] store de Svelte actualiza estado → cronómetro corriendo
```

El "ahora" se inyecta vía `Clock` para poder testear sin reloj real.

---

## 7. Manejo de errores

- Errores de dominio/aplicación como **enums Rust** con `thiserror`
  (`ProjectNameEmpty`, `TaskNotFound`, `SessionNotFound`, `NoRunningSession`…).
- Casos de uso devuelven `Result<T, AppError>`. **Prohibido** `unwrap()`/`panic!`
  en flujo normal.
- Los comandos Tauri convierten el error a un tipo **serializable** para el
  frontend, sin filtrar detalles internos. El frontend lo muestra de forma
  controlada.

---

## 8. Estrategia de pruebas (TDD)

- **Dominio (unit, `cargo test`):** invariantes — una sola sesión activa,
  cálculo de duración, rechazo de nombres vacíos.
- **Aplicación:** casos de uso contra `InMemory*Repository` (test doubles que
  implementan los traits) → sin tocar SQLite.
- **Infraestructura:** repos contra una SQLite **temporal** (archivo en dir
  temporal); verifica el contrato real del trait.
- **Frontend:** `vitest` ligero para stores/lógica de UI.
- Todo bug corregido ⇒ prueba de regresión (falla antes, pasa después).

---

## 9. Estructura de carpetas

```txt
laboraltracker/
├── src/                       # Frontend Svelte
│   └── lib/
│       ├── api/               # wrappers tipados de invoke()
│       ├── stores/            # estado (timer, proyectos, tareas)
│       └── components/
├── src-tauri/
│   ├── src/
│   │   ├── domain/            # entidades, value objects, puertos (traits), errores
│   │   ├── application/       # casos de uso
│   │   ├── infrastructure/    # repos SQLite, migraciones
│   │   ├── presentation/      # comandos Tauri + DTOs
│   │   ├── lib.rs             # composition root (DI + registro de comandos)
│   │   └── main.rs
│   ├── migrations/            # esquema SQLite versionado
│   ├── Cargo.toml
│   └── tauri.conf.json
├── CLAUDE.md                  # convenciones Rust + TS (espíritu de python.md)
├── README.md
└── .gitignore
```

---

## 10. Git y proceso

- Repositorio git desde el commit 0.
- **Conventional commits** (`feat:`, `fix:`, `test:`, `refactor:`, `docs:`,
  `chore:`). Incrementos pequeños y revisables.
- Ramas: `main` + `feature/…`.
- **Primer entregable antes de codificar:** redactar el `CLAUDE.md` del proyecto
  con las convenciones Rust/TS y la tabla SOLID como guía viva.

---

## 11. Criterios de aceptación del MVP A

- [ ] Se puede crear un proyecto y una tarea dentro de él.
- [ ] Iniciar el cronómetro crea una `TimeSession` con `ended_at = None`.
- [ ] Iniciar un cronómetro nuevo detiene automáticamente el anterior.
- [ ] Parar el cronómetro fija `ended_at` y la duración se calcula correctamente.
- [ ] Los datos persisten al cerrar y reabrir la app (SQLite local).
- [ ] La vista "hoy" muestra el tiempo acumulado por tarea/proyecto.
- [ ] `cargo test` y `vitest` en verde; sin `unwrap()`/`panic!` en flujo normal.
- [ ] El dominio no depende de `rusqlite` ni de Tauri (verificable por imports).
```