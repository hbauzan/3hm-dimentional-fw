# CÓMO USAR `transparent-dev-protocol` (instalación y portabilidad)

Guía de uso de la skill: cómo encajan los archivos, cómo instalarla donde la herramienta descubre skills (Claude Code, Cursor) y cómo apuntarla desde un archivo de reglas cuando la herramienta no tiene skills nativas.

> ⚠️ **Distinción clave**: el contenido es portable y stack-agnóstico; el mecanismo de carga varía por herramienta. Claude Code y Cursor descubren una skill y pueden leer un módulo solo cuando hace falta. El resto usa un archivo de reglas que apunta a `SKILL.md`. Si esa herramienta inlinea todo lo que referenciás, apuntá **solo** a `SKILL.md` y dejá que el agente abra los módulos bajo demanda.

---

## 1. Cómo encajan las piezas (entre ellas)

```
transparent-dev-protocol/
├─ SKILL.md          ← ENTRADA. Router liviano. Se lee SIEMPRE primero.
│                      (rol, estilo, §3 toolchain canónico, §0 flujo idea→entrega, higiene, bootstrap)
├─ code-design.md    ← módulos profundos + vertical slices (tracer bullets) + TDD
├─ debugging.md      ← loop estructurado de 6 fases
├─ qa-review.md      ← review de dos ejes (Standards vs Spec) + issues durables
├─ git-workflow.md   ← commits, hooks de higiene, entrega con approval gate
├─ documentation.md  ← doc-sync condicional (manifest/spec/CONTEXT)
├─ epistemic.md      ← auditoría explícita (/analisis, /audit, …)
│    estos seis se leen SOLO cuando la tarea lo pide (rutas relativas desde SKILL.md)
├─ templates/        ← copy-to-root: .pre-commit-config.yaml, .env.example
└─ USAGE.md          ← este archivo
```

- **`SKILL.md` es el único archivo "siempre cargado".** Es un índice/router: no duplica el contenido de los módulos, los referencia. Eso es lo que ahorra tokens y mantiene limpia la smart-zone.
- **Los módulos son auto-contenidos** y se cruzan entre sí con rutas relativas (`./debugging.md`, etc.). No usan rutas absolutas → la carpeta funciona en cualquier repo y sistema operativo.
- **Punteros en la raíz de la app** (`CLAUDE.md`, `AGENTS.md`, `GEMINI.md`) referencian `.agents/skills/transparent-dev-protocol/SKILL.md`. Son finos, van versionados y no copian el protocolo.

---

## 2. Claude Code y Cursor (skill nativa)

### Cómo se invoca
- **Auto (description)**: el frontmatter dispara la skill cuando la tarea involucra arquitectura, lógica de alta densidad o componentes de modelos/LLM en cualquier stack.
- **Explícito**: `/transparent-dev-protocol` (requiere symlink en `.claude/skills/`).
- **Garantía sin symlink**: `CLAUDE.md` (versionado, siempre cargado) referencia `SKILL.md`, así el protocolo se aplica aunque el symlink no exista todavía en ese clon.
- **Frase canónica** (en cualquier herramienta):  
  `Usando transparent-dev-protocol, <qué hacer / mejorar / arreglar>`
- **Cursor**: el symlink en `.cursor/skills/` la hace descubrible.

### Instalar — opción A: per-repo (recomendada)
La copia versionada vive en `.agents/skills/transparent-dev-protocol/`. El symlink de descubrimiento no viaja si `.claude/skills/` está gitignored: recrealo una vez por clon.
```bash
# desde la raíz de la app, una vez por clon
mkdir -p .claude/skills .cursor/skills
ln -sfn ../../.agents/skills/transparent-dev-protocol .claude/skills/transparent-dev-protocol
ln -sfn ../../.agents/skills/transparent-dev-protocol .cursor/skills/transparent-dev-protocol
```

### Instalar — opción B: global (disponible en TODOS tus proyectos)
```bash
cp -R /ruta/a/transparent-dev-protocol ~/.claude/skills/transparent-dev-protocol   # Claude Code
cp -R /ruta/a/transparent-dev-protocol ~/.cursor/skills/transparent-dev-protocol   # Cursor
```

---

## 3. Reglas de contexto (cuando no hay skill nativa, o además de ella)

La estrategia es siempre la misma en dos pasos:
1. **Una carpeta** `transparent-dev-protocol/` en el repo. Convención: `.agents/skills/transparent-dev-protocol/`.
2. **Apuntá el archivo de reglas** a ese `SKILL.md` y pedí carga bajo demanda.

| Herramienta | Dónde | Qué poner |
| :--- | :--- | :--- |
| **Cursor** | Skill: el symlink de la opción A. Regla opcional: `.cursor/rules/transparent-dev-protocol.mdc` | La plantilla de abajo, con la ruta real. |
| **Gemini CLI** | `GEMINI.md` (raíz; soporta jerárquicos) | La plantilla de abajo. |
| **OpenCode** | `AGENTS.md` (o `instructions` en `opencode.json`) | La plantilla de abajo. |
| **GitHub Copilot** | `.github/copilot-instructions.md` | La plantilla de abajo. |
| **Windsurf** | `.windsurfrules` (o `.windsurf/rules/`) | La plantilla de abajo. |
| **Genérico / multi-tool** | `AGENTS.md` ([agents.md](https://agents.md)) | La plantilla de abajo. Lo leen Codex, OpenCode y otros. |

### Plantilla de regla (pegá esto en el archivo de la herramienta)
```markdown
# Protocolo de desarrollo
Para CUALQUIER tarea de arquitectura, implementación, bug, review o entrega en este repo,
seguí el protocolo en `.agents/skills/transparent-dev-protocol/SKILL.md`.
- Leé `SKILL.md` primero (rol, estilo, toolchain canónico, flujo idea→entrega con approval gate).
- Leé los módulos SOLO cuando la tarea lo pida:
  diseño/TDD vertical → code-design.md · bug → debugging.md · review → qa-review.md ·
  git/entrega → git-workflow.md · docs → documentation.md ·
  auditoría explícita (/analisis, /audit, /grounding) → epistemic.md.
- Regla dura de toolchain: usar exclusivamente el gestor canónico del stack (Cargo, uv, Go, etc.; sin herramientas legacy ni mezclas).
- No hagas push/merge sin OK explícito del usuario (approval gate de git-workflow.md §3).
  Base protegida o repo con PRs: `gh pr create`, no push a la base.
```

---

## 4. Higiene y una sola fuente de verdad

- Mantené **una** copia de `transparent-dev-protocol/` por repo y que todos los archivos de reglas la referencien en lugar de duplicarla.
- Si una app tiene un frontend y un backend en lenguajes distintos, cada submódulo respeta su propio manifest de verdad, gobernado bajo los mismos principios unificados de diseño y entrega de este protocolo.
