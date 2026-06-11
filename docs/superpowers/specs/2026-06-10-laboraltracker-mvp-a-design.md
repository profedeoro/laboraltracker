# LaboralTracker — Diseño MVP A (Cronómetro con proyectos)

- **Fecha:** 2026-06-10 (rev. 2026-06-11 tras revisión técnica)
- **Estado:** Aprobado para implementación — fundaciones (tiempo + concurrencia) cerradas.
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

### Naturaleza del proyecto (honestidad de marco)

Este es un proyecto **didáctico además de funcional**: se eligió el stack
Tauri (Rust + TS) y una arquitectura hexagonal completa **priorizando el
aprendizaje del stack moderno y de la disciplina arquitectónica**, no la
velocidad de entrega. Para la funcionalidad cruda de un cronómetro, la
arquitectura está deliberadamente **sobredimensionada** (ver §4.1). Es una
decisión consciente y aprobada, no un descuido. Si el objetivo fuera entregar
producto al mínimo coste, se colapsarían capas; aquí el andamiaje **es** parte
del objetivo de aprendizaje.

### Marco ético/legal (no negociable)

El producto final es monitoreo **consentido y transparente**: el usuario sabe
que se le rastrea, puede pausar y ve sus propios datos. No se construirá
vigilancia encubierta (stalkerware). En este MVP local esto es trivial (el
usuario rastrea su propio tiempo manualmente), pero la decisión queda registrada
para los sub-proyectos futuros.

---

## 2. Stack tecnológico

| Capa            | Tecnología                                          |
|-----------------|-----------------------------------------------------|
| Empaquetado     | **Tauri 2.x** (app de escritorio)                   |
| Núcleo/dominio  | **Rust** (lógica de negocio pura, testeable)        |
| Frontend/UI     | **Svelte + TypeScript**                             |
| Persistencia    | **SQLite local** vía `rusqlite`                     |
| Migraciones     | `rusqlite_migration` (o `PRAGMA user_version` manual)|
| Contrato FE↔BE  | tipos TS **generados desde Rust** (`ts-rs`)         |
| Errores (Rust)  | `thiserror`                                          |
| Tests Rust      | `cargo test` (+ `proptest` para aritmética de tiempo)|
| Tests frontend  | `vitest`                                             |

**Decisión registrada:** se eligió Tauri (Rust + TS) sobre Python/PySide6
priorizando aprendizaje del stack moderno. Como no existe un `rust.md`/`ts.md`
en las reglas globales, se traduce el *espíritu* de `~/.claude/rules/python.md`
(capas, validación en bordes, errores explícitos, tests, seguridad) a
convenciones propias de Rust/TS. **Importante:** se escribe Rust **idiomático**
(el sistema de tipos y `Result` ya dan garantías que en Python se logran con
capas); no se transliteran patrones de Python uno a uno.

**Decisión:** se usa `rusqlite` directamente en Rust, **no** `tauri-plugin-sql`,
para no filtrar SQL al frontend y mantener la separación de capas.

---

## 3. Alcance del MVP A

### Dentro de alcance
- Crear / listar / archivar **proyectos**.
- Crear / listar tareas dentro de un proyecto.
- Iniciar y parar un **cronómetro** sobre una tarea.
- Regla: **una sola sesión corriendo a la vez** (iniciar una nueva detiene la activa).
- Ver el **tiempo registrado hoy** (por tarea/proyecto), con política de medianoche.
- Recuperación de **sesión huérfana** al arrancar la app.
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
   │  invoke()  ── comandos Tauri (DTOs tipados generados desde Rust)
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

### 4.1 Nota de proporción (decisión consciente)

Para la funcionalidad real del MVP (tres reglas: nombre no vacío, duración =
fin − inicio, una sesión activa) esta arquitectura es **sobredimensionada**: se
escriben varios structs de caso de uso, traits + impl SQLite + impl in-memory +
DTOs + handlers antes de que el cronómetro marque un segundo. **Se acepta a
propósito** como ejercicio de aprendizaje de arquitectura (ver §1). En cada
sub-proyecto futuro se revisará si una abstracción gana su sitio o sobra; el
proceso "un sub-proyecto a la vez" sirve también para **podar**, no solo sumar.

### 4.2 Validación SOLID (guía viva)

- **SRP:** una razón de cambio por pieza. Cada caso de uso = un struct con un
  método. Entidades solo sus invariantes; repos solo persistencia; handlers solo
  traducción.
- **OCP:** añadir un adaptador de persistencia nuevo no toca dominio ni casos de
  uso. **Límite honesto (ver §11):** esto aísla del *motor* de persistencia, no
  de los problemas de sistemas distribuidos del salto a la nube.
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
          └── TimeSession (id, task_id, started_at, ended_at?, last_heartbeat_at?, is_suspect)
```

- **Project** — agrupa tareas. Invariante: `name` no vacío.
- **Task** — pertenece a un proyecto. Invariante: `name` no vacío.
- **TimeSession** — intervalo de tiempo. `ended_at = None` ⇒ **corriendo**.
  Duración **derivada** (`ended_at − started_at`), no se almacena redundante.
- **Invariante clave:** solo una `TimeSession` corriendo a la vez.

### 5.1 Dueño único del invariante (corrige contradicción previa)

El invariante "una sola sesión activa" y la transición "parar la activa al
iniciar otra" son **orquestación**, así que viven **exclusivamente en el caso de
uso `StartTimerUseCase`** — no en un servicio de dominio paralelo. (La revisión
anterior tenía esta responsabilidad duplicada entre §5 y §6; queda en un solo
dueño.) **Defensa en profundidad:** la BD lo garantiza además con un índice
único parcial (§7). El `Mutex<Connection>` (§8) serializa la ejecución del caso
de uso, así que no hay carrera viva real; el índice es la red de seguridad.

### 5.2 Puertos (traits del dominio)

```txt
ProjectRepository   add / list / find_by_id / archive
TaskRepository      add / list_by_project / find_by_id
SessionRepository   add(new) / save(existing) / find_running / list_overlapping(range)
Clock               now() -> epoch_millis_utc   (inyectable para tests)
```

Semántica `add` vs `save`: **`add` = INSERT** de sesión nueva; **`save` =
UPDATE** de `ended_at`/`is_suspect` de una existente. (Alternativa aceptable:
colapsar en un único `save`/`upsert`; se decide en el plan.)

---

## 6. Política de tiempo (no negociable — el tiempo es el dominio)

- **Almacenamiento:** todos los instantes se guardan como `INTEGER` = epoch en
  **milisegundos UTC**. Nada de strings locales, nada de TZ en la columna. UTC
  elimina la ambigüedad de DST y cambios de zona.
- **Duración:** derivada (`ended_at − started_at`); nunca se persiste.
- **Zona horaria = presentación, no dominio.** El "día" (vista "hoy") se calcula
  en la zona local del usuario y se traduce a un rango `[day_start, day_end)` en
  epoch-millis UTC antes de tocar SQL. Si el usuario viaja, "hoy" sigue su zona
  actual.
- **Reloj de pared, no monotónico:** los instantes persistidos vienen del
  `Clock` (reloj de pared). El reloj de pared *salta* (NTP, DST, ajuste manual,
  suspensión) → por eso las salvaguardas siguientes.
- **Sesión cruzando medianoche:** NO se atribuye al día de inicio; se reparte
  por **solapamiento** del intervalo con el rango de cada día (23:30→01:00 suma
  30 min a un día y 60 al siguiente).
- **Sesión huérfana** (`ended_at = NULL` al arrancar la app): no se confía en su
  duración.
    1. Al iniciar la app se detecta cualquier sesión con `ended_at = NULL`.
    2. Se cierra en `last_heartbeat_at` (si existe) o en `started_at`.
    3. Se marca `is_suspect = 1` para no contaminar reportes en silencio.
  Mientras una sesión corre, se persiste `last_heartbeat_at` cada ~30 s como
  ancla de recuperación si el proceso muere.
- **Cap de duración:** sesión cuya duración supere un máximo configurable
  (default 12 h) se marca `is_suspect = 1`.
- **Dos relojes (UI vs BD):** la UI muestra el transcurrido calculado en JS desde
  `startedAt` (solo presentación); el **dato canónico al parar lo fija el
  backend** con `clock.now()`. La UI nunca es fuente de verdad del tiempo.

---

## 7. Esquema SQLite (`migrations/0001_init.sql`)

```sql
PRAGMA foreign_keys = ON;

CREATE TABLE project (
    id          INTEGER PRIMARY KEY,
    name        TEXT    NOT NULL CHECK (length(trim(name)) > 0),
    color       TEXT,
    created_at  INTEGER NOT NULL,            -- epoch millis UTC
    archived    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE task (
    id          INTEGER PRIMARY KEY,
    project_id  INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    name        TEXT    NOT NULL CHECK (length(trim(name)) > 0),
    created_at  INTEGER NOT NULL,
    completed   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX ix_task_project ON task(project_id);

CREATE TABLE time_session (
    id                INTEGER PRIMARY KEY,
    task_id           INTEGER NOT NULL REFERENCES task(id) ON DELETE CASCADE,
    started_at        INTEGER NOT NULL,           -- epoch millis UTC
    ended_at          INTEGER,                    -- NULL = corriendo
    last_heartbeat_at INTEGER,                    -- ancla de recuperación
    is_suspect        INTEGER NOT NULL DEFAULT 0,
    CHECK (ended_at IS NULL OR ended_at >= started_at)  -- sin duraciones negativas
);
CREATE INDEX ix_session_task    ON time_session(task_id);
CREATE INDEX ix_session_started ON time_session(started_at);

-- INVARIANTE GARANTIZADO POR LA BD: como máximo UNA sesión corriendo.
-- La expresión vale 1 para toda fila con ended_at IS NULL; UNIQUE ⇒ una sola.
CREATE UNIQUE INDEX ux_one_running_session
    ON time_session( (ended_at IS NULL) )
    WHERE ended_at IS NULL;
```

- `CHECK (ended_at >= started_at)` es la red contra el salto de reloj de pared:
  si NTP retrocede el reloj entre start y stop, el UPDATE **falla** en vez de
  guardar una duración negativa silenciosa.
- `ux_one_running_session` da defensa en profundidad real (alternativa más
  legible: columna generada `is_running` + `UNIQUE INDEX`).
- SQLite no aplica FK por defecto: el `ON DELETE CASCADE` solo funciona con
  `PRAGMA foreign_keys = ON` **en cada conexión** (ver §8).

### 7.1 Vista "hoy" por solapamiento

```sql
SELECT t.project_id, s.task_id,
       SUM( MIN(COALESCE(s.ended_at, :now), :day_end)
          - MAX(s.started_at, :day_start) ) AS ms
FROM time_session s
JOIN task t ON t.id = s.task_id
WHERE s.is_suspect = 0
  AND s.started_at < :day_end
  AND COALESCE(s.ended_at, :now) > :day_start
GROUP BY t.project_id, s.task_id;
```

`:day_start`/`:day_end` se calculan en Rust (medianoche local → epoch UTC).
Excluir `is_suspect` evita que una sesión huérfana infle el día.

---

## 8. Concurrencia y conexión

- `rusqlite::Connection` es `Send` pero `!Sync`. Vive tras un
  `Mutex<Connection>` en el `State` de Tauri (suficiente para mono-usuario; un
  pool solo se justifica con lecturas concurrentes reales, que aquí no hay).
- **Comandos síncronos:** el I/O de SQLite local es sub-milisegundo; no
  justifica `async`. Regla: **nunca** mantener un `std::sync::Mutex` a través de
  un `.await`. Si en el futuro un comando hiciera trabajo largo, envolver el
  bloqueante en `tauri::async_runtime::spawn_blocking`.
- **Pragmas por conexión** (`foreign_keys`, `busy_timeout` no persisten en el
  archivo), centralizados en `infrastructure::db::open`:

```rust
// infrastructure/db.rs
pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(conn)
}

pub struct Db(pub Mutex<Connection>);   // Tauri .manage(Db(...))
```

El handler toma el lock síncronamente y construye los repos sobre `&conn`.

---

## 9. Flujo de datos (iniciar cronómetro — corregido)

```txt
StartTimerUseCase.execute(task_id):
  · let now = clock.now();                       // epoch-millis UTC
  · if let Some(running) = repo.find_running():  // cierra la activa
        running.stop(now)?;                        //   (CHECK valida >= started_at)
        repo.save(running);
  · let session = TimeSession::start(task_id, now);
  · repo.add(session);                            // ux_one_running_session
                                                  //   rechaza una 2ª abierta
  · return SessionDto { id, started_at };
```

El "ahora" se inyecta vía `Clock`. La UI recibe `{ id, started_at }` y tickea en
JS solo para mostrar; el backend es la fuente de verdad al parar.

---

## 10. Manejo de errores

- Errores de dominio/aplicación como **enums Rust** con `thiserror`
  (`ProjectNameEmpty`, `TaskNotFound`, `SessionNotFound`, `NoRunningSession`,
  `ClockWentBackwards`…). Prohibido `unwrap()`/`panic!` en flujo normal.
- **App local mono-usuario:** se **loguea el error completo** (el usuario es el
  dueño de la máquina; ocultárselo solo dificulta el debug) y se devuelve al
  frontend un error **serializable y controlado**. La ocultación estricta de
  detalle se reserva para cuando exista una frontera de confianza real (nube).
- Los DTOs y el tipo de error se **generan a TS desde Rust** (`ts-rs`) para que
  el contrato `invoke` no pueda divergir en silencio.

---

## 11. Límite honesto del salto a la nube (corrige promesa OCP ingenua)

La arquitectura aísla del **motor de persistencia**, **no** de los problemas de
sistemas distribuidos. El salto de local mono-usuario a equipo/nube **no** es un
mero cambio de adaptador: introduce identidad de usuario, organizaciones,
multi-tenancy, propiedad de datos, autenticación, latencia/fallos de red,
sincronización offline y resolución de conflictos — y todo eso **cambia el
dominio**, no solo la infraestructura. El adaptador Postgres será la parte fácil;
el modelo identidad/tenant/sync será la difícil y llegará al dominio. No se
invierte hoy esperando blindar ese futuro.

---

## 12. Riesgo de secuenciación (registrado, no bloqueante)

El MVP A construye el 20% fácil (cronómetro local) y difiere el 80% difícil y
específico de plataforma: monitoreo de apps/ventana, captura de pantalla,
permisos de SO (grabación en macOS, accesibilidad), UX de consentimiento, sync
seguro multi-tenant. **Acción:** mantener el MVP A, pero **agendar pronto un
spike técnico** del sub-proyecto de monitoreo + permisos de SO en un SO objetivo
(Windows primero) para validar viabilidad antes de pulir en exceso el núcleo.

---

## 13. Estrategia de pruebas (TDD)

- **Dominio (unit, `cargo test`):** invariantes — una sola sesión activa,
  duración, rechazo de nombres vacíos, `stop` con reloj retrocedido.
- **Aritmética de tiempo:** `proptest` sobre cálculo de duración y solapamiento
  con medianoche.
- **Aplicación:** casos de uso contra `InMemory*Repository` (test doubles).
- **Infraestructura:** repos contra SQLite **temporal**; verifica el contrato del
  trait y que el índice único parcial rechaza una segunda sesión abierta.
- **Costura FE↔BE:** al menos un test delgado que ejerza un comando real con
  DTOs/errores serializados; tipos TS generados desde Rust como contrato.
- **Frontend:** `vitest` ligero para stores/lógica de UI.
- Todo bug corregido ⇒ prueba de regresión (falla antes, pasa después).

---

## 14. Estructura de carpetas

```txt
laboraltracker/
├── src/                       # Frontend Svelte
│   └── lib/
│       ├── api/               # wrappers tipados de invoke() + tipos generados
│       ├── stores/            # estado (timer, proyectos, tareas)
│       └── components/
├── src-tauri/
│   ├── src/
│   │   ├── domain/            # entidades, value objects, puertos (traits), errores
│   │   ├── application/       # casos de uso
│   │   ├── infrastructure/    # db.rs, repos SQLite, migraciones
│   │   ├── presentation/      # comandos Tauri + DTOs
│   │   ├── lib.rs             # composition root (DI + registro de comandos)
│   │   └── main.rs
│   ├── migrations/            # 0001_init.sql (esquema versionado)
│   ├── Cargo.toml
│   └── tauri.conf.json
├── CLAUDE.md                  # convenciones Rust + TS, política de tiempo, concurrencia
├── README.md
└── .gitignore
```

---

## 15. Git y proceso

- Repositorio git desde el commit 0. **Conventional commits**. Incrementos
  pequeños y revisables. Ramas: `main` + `feature/…`.
- **Primer entregable antes de codificar:** redactar el `CLAUDE.md` del proyecto
  con: convenciones Rust/TS idiomáticas, tabla SOLID, **política de tiempo (§6)**,
  **modelo de concurrencia (§8)** y semántica de errores (§10).

---

## 16. Criterios de aceptación del MVP A

- [ ] Crear un proyecto y una tarea dentro de él.
- [ ] Iniciar el cronómetro crea una `TimeSession` con `ended_at = NULL`.
- [ ] Iniciar un cronómetro nuevo detiene automáticamente el anterior.
- [ ] La BD rechaza una segunda sesión abierta (índice único parcial).
- [ ] Parar fija `ended_at`; la duración se calcula correctamente.
- [ ] Un reloj retrocedido entre start y stop es rechazado (CHECK), no guardado.
- [ ] Al reabrir tras un cierre sucio, la sesión huérfana se cierra y se marca
      `is_suspect`.
- [ ] Una sesión que cruza medianoche se reparte correctamente entre días.
- [ ] Los datos persisten al cerrar y reabrir (SQLite local).
- [ ] La vista "hoy" muestra el tiempo acumulado por tarea/proyecto.
- [ ] Todos los instantes persistidos son epoch-millis UTC.
- [ ] `cargo test` (incl. `proptest`) y `vitest` en verde; sin `unwrap()`/`panic!`
      en flujo normal.
- [ ] El dominio no depende de `rusqlite` ni de Tauri (verificable por imports).
- [ ] Los tipos TS de DTOs/errores se generan desde Rust (no duplicados a mano).
```