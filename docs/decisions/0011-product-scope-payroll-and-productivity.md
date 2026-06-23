# 0011 — Product scope: payroll-defensible AND productivity tool

- **Status:** Accepted
- **Date:** 2026-06-22

## 🇬🇧 Context
A gap analysis (raised mid-Phase-1) argued LaboralTracker is "a timer with
projects" and far from a Time Doctor-class product, naming four missing
capabilities: activity monitoring, sync, payroll workflow, and anti-tamper.
Verified against the repo:

- **Activity monitoring** — designed, not coded: modules `Activity`/`Screenshots`
  (00-architecture-overview §3), Phase 5 of the roadmap, and a dedicated privacy
  doc (03-privacy-policy). The "not written anywhere" claim was false.
- **Sync** — designed (00/02), not coded: Phase 3. Correct.
- **Payroll workflow** — genuinely undesigned: the roadmap had `Billing`
  (hour/seat billing, Phase 6) but no manual time editing with audit, manager
  approval, paid/unpaid break policies, overtime, per-employee rates, timesheet
  export, PTO, or attendance-vs-schedule. Real gap.
- **Anti-tamper** — partial: the agent rejects backward clocks and flags >12 h
  sessions (02-time-policy), but does NOT detect large **forward** clock jumps,
  has no local-DB integrity (checksum/signature), and no agent event log
  (start/stop/crash/clock-change). Real gaps.

The decision the analysis could not make — and the owner did — is **scope**:
the product targets **both** a *payroll-defensible* system (a company runs
payroll off its data) **and** a *visibility/productivity* tool. "Both" is the
most demanding answer: payroll-defensible means the data must survive a labor
audit. This product constraint, not the comparison to Time Doctor, governs.

## 🇪🇸 Contexto
Un análisis de gaps (planteado a mitad de la Fase 1) sostuvo que LaboralTracker
es "un cronómetro con proyectos" y está lejos de un producto tipo Time Doctor,
nombrando cuatro capacidades faltantes: monitoreo de actividad, sync, workflow de
payroll y anti-tamper. Verificado contra el repo:

- **Monitoreo de actividad** — diseñado, no codeado: módulos `Activity`/`Screenshots`
  (00-architecture-overview §3), Fase 5 del roadmap, y un doc de privacidad dedicado
  (03-privacy-policy). La afirmación "no está escrito en ningún lado" era falsa.
- **Sync** — diseñado (00/02), no codeado: Fase 3. Correcto.
- **Workflow de payroll** — genuinamente sin diseñar: el roadmap tenía `Billing`
  (facturación por horas/usuarios, Fase 6) pero no edición manual de tiempo con
  auditoría, aprobación de manager, políticas de pausas pagas/no pagas, horas extra,
  rates por empleado, export de timesheet, PTO, ni asistencia vs. horario. Gap real.
- **Anti-tamper** — parcial: el agente rechaza reloj retrocedido y marca sesiones
  >12 h (02-time-policy), pero NO detecta saltos grandes de reloj **hacia adelante**,
  no tiene integridad de la BD local (checksum/firma), ni log de eventos del agente
  (start/stop/crash/cambio-de-reloj). Gaps reales.

La decisión que el análisis no podía tomar — y el dueño sí — es el **scope**: el
producto apunta a **ambos**: un sistema *payroll-defensible* (la empresa liquida
sueldos con sus datos) **y** una herramienta de *visibilidad/productividad*.
"Ambos" es la respuesta más exigente: payroll-defensible significa que los datos
deben sobrevivir una auditoría laboral. Esa restricción de producto, no la
comparación con Time Doctor, es la que manda.

## 🇬🇧 Decision / 🇪🇸 Decisión
**Scope = both.** LaboralTracker is built so a company can both *see* how time is
spent (productivity) and *pay* people from that data (payroll-defensible). Two
consequences are adopted as governing principles:

1. **Integrity is a foundation property, designed in — not bolted on.** A
   payroll-defensible record must be tamper-evident *from the moment it is
   captured*, and every sensitive mutation auditable *from the first one*. A
   phase-by-phase review (this turn) pinned the exact landing points:
   - **`AuditLog` lands in Phase 2 (domain CRUD)** — not Phase 6 — because
     Phase 2 ships the first auditable mutations (role assignment, member
     suspension, project/task deletion). CRUD without an audit trail leaves those
     mutations permanently unauditable. The model + doc-02 §6 action list already
     exist; only the phasing was wrong. The audit **UI** stays in Phase 6.
   - **Agent integrity lands in Phase 3 (with the sync client)**: forward
     clock-jump detection + tamper-evident agent event log + local-DB integrity.
     Once the agent syncs, what it uploads becomes the legal substrate. The
     platform **persists the agent's integrity verdict, it does not recompute it**
     (`isSuspect` semantics expand, or an explicit integrity reason is added).
   - **`TimeEntry` must be born audit-aware in Phase 3**: a manual edit of
     agent-captured time (the most abuse-prone payroll action) must preserve the
     original value + editor + timestamp + reason. The seam is respected when
     `TimeEntry` is built, even though payroll is Phase 7 — otherwise Phase 3/4
     ship a mutable entry whose history Phase 7 cannot reconstruct.

   Integrity and audit cannot be retrofitted onto data that already exists.
2. **Payroll workflow is an explicit roadmap phase, not an afterthought.** Manual
   time editing (audited), manager approval, break/overtime policy, per-employee
   rates, and timesheet export get their own spec→plan cycle. Build order still
   follows dependencies (it sits on top of CRUD + sync + reports), but it is now
   named and owned rather than absent.

**Build order is unchanged** — the dependency graph (timer → {activity, sync} →
reports → payroll) still holds; "both" raises the *bar* and adds a *phase*, it
does not let any layer jump its dependencies. /
**Scope = ambos.** LaboralTracker se construye para que una empresa pueda tanto
*ver* en qué se va el tiempo (productividad) como *pagar* sueldos con esos datos
(payroll-defensible). Se adoptan dos principios rectores:

1. **La integridad es propiedad de los cimientos, se diseña — no se atornilla.**
   Un registro payroll-defensible debe ser tamper-evident *desde que se captura*, y
   toda mutación sensible auditable *desde la primera*. Un review fase por fase
   (este turno) fijó los puntos de aterrizaje exactos:
   - **El `AuditLog` aterriza en la Fase 2 (CRUD de dominio)** — no en la Fase 6 —
     porque la Fase 2 trae las primeras mutaciones auditables (asignar rol,
     suspender member, borrar proyecto/tarea). CRUD sin trazas deja esas mutaciones
     sin auditar para siempre. El modelo + la lista de acciones del doc 02 §6 ya
     existen; solo el faseado estaba mal. La **UI** de auditoría se queda en Fase 6.
   - **La integridad del agente aterriza en la Fase 3 (con el cliente de sync)**:
     detección de salto de reloj hacia adelante + log de eventos tamper-evident +
     integridad de la BD local. Una vez que el agente sincroniza, lo que sube es el
     sustrato legal. La plataforma **persiste el veredicto de integridad del agente,
     no lo recalcula** (se expande la semántica de `isSuspect`, o se suma un motivo
     de integridad explícito).
   - **El `TimeEntry` debe nacer audit-aware en la Fase 3**: una edición manual de
     tiempo capturado por el agente (la acción más abusable de payroll) debe
     preservar el valor original + quién + cuándo + por qué. El seam se respeta al
     construir `TimeEntry`, aunque payroll sea Fase 7 — o si no Fase 3/4 shippean una
     entrada mutable cuyo historial la Fase 7 no puede reconstruir.

   La integridad y la auditoría no se retrofitean sobre datos que ya existen.
2. **El workflow de payroll es una fase explícita del roadmap, no un agregado.**
   Edición manual de tiempo (auditada), aprobación de manager, política de
   pausas/horas extra, rates por empleado y export de timesheet tienen su propio
   ciclo spec→plan. El orden de build sigue las dependencias (va encima de CRUD +
   sync + reportes), pero ahora está nombrado y con dueño en vez de ausente.

**El orden de build no cambia** — el grafo de dependencias (timer → {actividad,
sync} → reportes → payroll) sigue valiendo; "ambos" sube la *vara* y agrega una
*fase*, no le permite a ninguna capa saltarse sus dependencias.

## 🇬🇧 Consequences / 🇪🇸 Consecuencias
- `docs/saas/06-roadmap.md` updated: payroll workflow added as an explicit phase;
  a "defensibility thread" note pulls `AuditLog` + agent integrity forward to
  Phase 3. / `docs/saas/06-roadmap.md` actualizado: workflow de payroll agregado
  como fase explícita; una nota de "hilo de defensibilidad" adelanta `AuditLog` +
  integridad del agente a la Fase 3.
- **Agent integrity** joins the agent hardening backlog as P1 (forward clock-jump
  detection, tamper-evident event log, local-DB integrity), to be specced before
  the agent's sync client (Phase 3 counterpart). / **La integridad del agente** se
  suma al backlog de hardening como P1 (detección de salto hacia adelante, log de
  eventos tamper-evident, integridad de la BD local), a especificar antes del
  cliente de sync del agente (contraparte de la Fase 3).
- **Does not block Phase 2.** Domain CRUD (companies, members, teams, projects,
  tasks) is unaffected and proceeds next; this ADR records the scope so the gaps
  are not lost. / **No bloquea la Fase 2.** El CRUD de dominio no se ve afectado y
  sigue a continuación; este ADR registra el scope para no perder los gaps.
- **Legal/privacy coupling:** payroll-defensibility and the existing privacy
  policy (03-privacy-policy) must stay coherent — defensible records vs. employee
  privacy is a real tension to design through, not assume away. / **Acoplamiento
  legal/privacidad:** la defensibilidad de payroll y la política de privacidad
  existente (03-privacy-policy) deben mantenerse coherentes — registros defendibles
  vs. privacidad del empleado es una tensión real a diseñar, no a asumir resuelta.
