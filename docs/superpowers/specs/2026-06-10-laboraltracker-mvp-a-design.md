# LaboralTracker — Diseño MVP A (Cronómetro con proyectos)

- **Fecha:** 2026-06-10 (rev. 2026-06-11: revisión técnica + split de convenciones)
- **Estado:** Aprobado para implementación — fundaciones (tiempo + concurrencia) cerradas.
- **Sub-proyecto:** Núcleo local de *time tracking* (primer incremento de un clon estilo Time Doctor)

> **Naturaleza de este documento:** es un **registro de decisión** (rationale,
> alcance, riesgos). Las **reglas vivas** que gobiernan el código viven en
> [docs/conventions/](../../conventions/00-index.md) y se enlazan desde aquí —
> no se duplican (una sola fuente de verdad por concepto). El `CLAUDE.md` de la
> raíz lleva las invariantes en una línea.

---

## 1. Contexto y visión

LaboralTracker será, a futuro, una herramienta de seguimiento de tiempo y
productividad para **equipos/empresa** (estilo Time Doctor). Se construye de
forma **incremental**: un sub-proyecto bien hecho a la vez, cada uno con su
propio ciclo spec → plan → código → test → commit.

Este documento define el **primer sub-proyecto: el núcleo local de time
tracking**, cimiento del que colgarán después el monitoreo automático, captura de
apps/URLs, capturas de pantalla, clasificación de productividad, reportes y la
capa de nube/equipo.

### Naturaleza del proyecto (honestidad de marco)

Proyecto **didáctico además de funcional**: se eligió Tauri (Rust + TS) y una
arquitectura hexagonal completa **priorizando el aprendizaje del stack y de la
disciplina arquitectónica**, no la velocidad de entrega. Para la funcionalidad
cruda de un cronómetro, la arquitectura está deliberadamente **sobredimensionada**
(decisión consciente, ver [01-architecture-solid.md](../../conventions/01-architecture-solid.md)).

### Marco ético/legal (no negociable)

El producto final es monitoreo **consentido y transparente**: el usuario sabe que
se le rastrea, puede pausar y ve sus propios datos. **No** se construirá
vigilancia encubierta (stalkerware). En este MVP local es trivial (auto-registro
manual), pero la decisión queda registrada para los sub-proyectos futuros.

---

## 2. Stack tecnológico

| Capa            | Tecnología                                          |
|-----------------|-----------------------------------------------------|
| Empaquetado     | **Tauri 2.x**                                       |
| Núcleo/dominio  | **Rust** (lógica pura, testeable)                   |
| Frontend/UI     | **Svelte + TypeScript**                            |
| Persistencia    | **SQLite** vía `rusqlite`                           |
| Migraciones     | `rusqlite_migration` (o `PRAGMA user_version`)      |
| Contrato FE↔BE  | tipos TS **generados desde Rust** (`ts-rs`)         |
| Errores (Rust)  | `thiserror`                                          |
| Tests           | `cargo test` (+ `proptest`) · `vitest`              |

Decisiones registradas: Tauri sobre Python/PySide6 por aprendizaje del stack;
`rusqlite` directo (no `tauri-plugin-sql`) para no filtrar SQL al frontend;
Rust **idiomático**, no Python transliterado.

---

## 3. Alcance del MVP A

### Dentro
- Crear / listar / archivar **proyectos**; crear / listar **tareas** por proyecto.
- Iniciar / parar **cronómetro** sobre una tarea; **una sola sesión a la vez**.
- Ver **tiempo de hoy** (por tarea/proyecto), con política de medianoche.
- Recuperación de **sesión huérfana** al arrancar. Persistencia SQLite local.

### Fuera (YAGNI — sub-proyectos posteriores)
- Monitoreo de apps/ventana e *idle*; capturas; productivo/improductivo;
  reportes avanzados/exportación; nube, cuentas, login, roles, multi-usuario.

---

## 4. Arquitectura y modelo de dominio

Clean / Hexagonal (puertos y adaptadores), dominio en Rust, DIP hacia el dominio.
**Detalle, puertos y tabla SOLID:** [01-architecture-solid.md](../../conventions/01-architecture-solid.md).

```txt
Project (id, name, color?, created_at, archived)
   └── Task (id, project_id, name, created_at, completed)
          └── TimeSession (id, task_id, started_at, ended_at?, last_heartbeat_at?, is_suspect)
```

- `TimeSession` con `ended_at = NULL` ⇒ corriendo. Duración derivada.
- **Invariante:** una sola sesión activa; dueño = `StartTimerUseCase`; garantía en
  BD (índice único parcial). Ver [05-data-schema.md](../../conventions/05-data-schema.md).

### Flujo: iniciar cronómetro

```txt
StartTimerUseCase.execute(task_id):
  · let now = clock.now();                       // epoch-millis UTC
  · if let Some(running) = repo.find_running():  // cierra la activa
        running.stop(now)?; repo.save(running);  //   CHECK valida >= started_at
  · repo.add(TimeSession::start(task_id, now));  // índice rechaza 2ª abierta
  · return SessionDto { id, started_at };
```

UI tickea en JS solo para mostrar; el backend es la fuente de verdad al parar.

---

## 5. Reglas vivas (enlaces — fuente de verdad en convenciones)

| Tema | Documento |
|------|-----------|
| Arquitectura, puertos, SOLID, dueño del invariante | [01-architecture-solid.md](../../conventions/01-architecture-solid.md) |
| Política de tiempo (UTC, medianoche, huérfana, dos relojes) | [02-time-policy.md](../../conventions/02-time-policy.md) |
| Concurrencia (`Mutex<Connection>`, comandos síncronos, pragmas) | [03-concurrency.md](../../conventions/03-concurrency.md) |
| Errores (`thiserror`, logging local, contrato tipado) | [04-error-handling.md](../../conventions/04-error-handling.md) |
| Esquema SQLite (DDL, índice único parcial, "hoy") | [05-data-schema.md](../../conventions/05-data-schema.md) |

---

## 6. Límite honesto del salto a la nube

La arquitectura aísla del **motor de persistencia**, **no** de los problemas de
sistemas distribuidos. El salto a equipo/nube introduce identidad, organizaciones,
multi-tenancy, autenticación, latencia/fallos de red, sync offline y resolución de
conflictos — y eso **cambia el dominio**, no solo la infraestructura. El adaptador
Postgres será la parte fácil; identidad/tenant/sync, la difícil, y llegará al
dominio. No se invierte hoy esperando blindar ese futuro.

---

## 7. Riesgo de secuenciación (registrado, no bloqueante)

El MVP A hace el 20% fácil (cronómetro local) y difiere el 80% difícil y
específico de plataforma (monitoreo, captura, permisos de SO, consentimiento, sync
multi-tenant). **Acción:** mantener el MVP A, pero **agendar pronto un spike** del
sub-proyecto de monitoreo + permisos de SO (Windows primero) para validar
viabilidad antes de pulir en exceso el núcleo.

---

## 8. Pruebas (TDD)

- **Dominio:** invariantes (una sesión activa, duración, nombres vacíos, `stop`
  con reloj retrocedido). **`proptest`** para aritmética de tiempo/solapamiento.
- **Aplicación:** casos de uso contra `InMemory*Repository`.
- **Infraestructura:** SQLite temporal; verifica que el índice único parcial
  rechaza una segunda sesión abierta.
- **Costura FE↔BE:** test delgado de un comando real; tipos TS generados desde Rust.
- **Frontend:** `vitest` ligero. Bug corregido ⇒ prueba de regresión.

---

## 9. Estructura de carpetas

```txt
laboraltracker/
├── src/                       # Svelte: lib/{api, stores, components}
├── src-tauri/
│   ├── src/{domain, application, infrastructure, presentation}/
│   ├── src/{lib.rs, main.rs}  # lib.rs = composition root
│   ├── migrations/            # 0001_init.sql (canónico al scaffold)
│   ├── Cargo.toml
│   └── tauri.conf.json
├── docs/{conventions/, superpowers/specs/}
├── CLAUDE.md                  # índice fino: invariantes + enlaces
├── README.md
└── .gitignore
```

---

## 10. Git y proceso

Git desde el commit 0; conventional commits; incrementos pequeños; ramas `main` +
`feature/…`. **Primer entregable de implementación:** el `CLAUDE.md` + las
convenciones (ya creados) son la base; el plan arranca con el scaffold de Tauri y
las migraciones.

---

## 11. Criterios de aceptación del MVP A

- [ ] Crear proyecto y tarea dentro de él.
- [ ] Iniciar crea `TimeSession` con `ended_at = NULL`; iniciar otra detiene la previa.
- [ ] La BD rechaza una segunda sesión abierta (índice único parcial).
- [ ] Parar fija `ended_at`; duración correcta.
- [ ] Reloj retrocedido entre start y stop es rechazado (CHECK), no guardado.
- [ ] Tras cierre sucio, la sesión huérfana se cierra y se marca `is_suspect`.
- [ ] Sesión que cruza medianoche se reparte entre días.
- [ ] Persistencia al cerrar/reabrir; vista "hoy" por tarea/proyecto.
- [ ] Todos los instantes persistidos = epoch-millis UTC.
- [ ] `cargo test` (incl. `proptest`) y `vitest` en verde; sin `unwrap()`/`panic!`.
- [ ] `domain` no depende de `rusqlite` ni Tauri (verificable por imports).
- [ ] Tipos TS de DTOs/errores generados desde Rust (no duplicados a mano).
```