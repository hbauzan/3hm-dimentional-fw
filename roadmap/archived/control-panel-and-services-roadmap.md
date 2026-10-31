# Roadmap: Modernización de setup.sh y Orquestación de Servicios Desacoplados

**Proyecto:** Three-Headed Semantic Firewall (`3hm-dimentional-fw`)  
**Documento:** `roadmap/control-panel-and-services-roadmap.md`  
**Estado:** IMPLEMENTADO / COMPLETADO (Verificado 100% Green)
**Objetivo:** Adaptar `setup.sh` a la arquitectura desacoplada (Gateway `sf-proxy` en `:8088` + SPA `frontend/` en `:5173` + LanceDB seeding) y dotar al usuario de control total de ciclo de vida y guía interactiva de pruebas.

---

## 1. Auditoría de Brechas (Gap Analysis: `setup.sh` vs Estado Real)

| Componente | Estado Actual en `setup.sh` | Realidad del Repo (Post-Fase 2) | Brecha / Fallo Crítico |
| :--- | :--- | :--- | :--- |
| **Backend Primario** | Solo gestiona `sf-ui` (:3000). | Gateway unificado `sf-proxy` (:8088). | `sf-proxy` no puede ser iniciado, monitoreado ni detenido desde el script. |
| **Frontend Primario** | Asume HTML embebido en `sf-ui`. | SPA desacoplada en `frontend/` (:5173). | No hay gestión de `pnpm dev`, builds ni monitoreo de Vite en `:5173`. |
| **Toolchain** | Verifica Homebrew, Rust, Python, Xcode. | `frontend/` requiere Node.js y `pnpm@11`. | Si falta `pnpm`, el script no lo detecta ni lo instala. |
| **Base de Datos** | Ingesta manual interactiva de archivo binario `.f32`. | `data/sf-lancedb` vacío $\implies$ `sf-proxy` crashea al arrancar. | No hay mecanismo "1-click" para hidratar el catálogo canónico (55 locks). |
| **Parada de Servicios** | Opción 10 solo mata puerto :3000 (`sf-ui`). | Hay procesos en :8088 (`sf-proxy`) y :5173 (`vite`). | Opción 10 deja procesos zombies en :8088 y :5173. |
| **Tests & Linters** | Solo ejecuta `cargo test` y `clippy`. | Hay 34 tests vitest, `tsc -b` y `oxlint` en `frontend/`. | Los tests del frontend quedan fuera del quality gate de `setup.sh`. |
| **Experiencia WebGUI** | Abre `http://127.0.0.1:3000`. | La UI moderna está en `http://127.0.0.1:5173`. | El usuario es dirigido a la interfaz anterior en vez de la nueva SPA. |

---

## 2. Plan de Trabajo por Slices (Para el Agente Ejecutor)

### Slice 1: Toolchain & Hidratación Canónica de LanceDB
* **1.1. Detección y Setup de `pnpm` (`ensure_node_pnpm`):**
  - Agregar verificación de `node` (>=18) y `pnpm` (>=9) en `ensure_prerequisites()`.
  - Si falta `pnpm`, ofrecer instalación vía `corepack enable` o `brew install pnpm` / `npm install -g pnpm`.
* **1.2. Hidratación Automática / Seeder Canónico de LanceDB:**
  - Crear un comando/binario rápido (o subcomando en `sf-storage` / helper en Rust o Python) que ejecute `sf_bench::generate_canonical_locks()` y persista los 55 locks en `data/sf-lancedb`.
  - En `setup.sh`, agregar verificación al iniciar el gateway: si `data/sf-lancedb` no contiene la tabla `domain_hypercubes` o está vacía, preguntar o auto-sembrar los 55 locks canónicos sin fricción.

---

### Slice 2: Orquestador Multi-Servicio en `setup.sh`
* **2.1. Puertos y Sondas Idempotentes:**
  - Variables de entorno parametrizadas:
    - `SF_PROXY_PORT="${SF_PORT:-8088}"`
    - `SF_SPA_PORT="${VITE_PORT:-5173}"`
    - `SF_UI_PORT="${SF_UI_PORT:-3000}"` (Legacy Diagnostic Studio)
  - Sondas individuales de salud (Healthchecks):
    - `probe_sf_proxy`: Chequear listener TCP :8088 y respuesta HTTP en `/api/v1/chat` o `/v1/chat/completions`.
    - `probe_sf_spa`: Chequear listener TCP :5173 y respuesta en `/`.
    - `probe_sf_ui`: Chequear listener TCP :3000 (Studio embebido).
* **2.2. Operaciones de Ciclo de Vida (Background y Foreground):**
  - `start_gateway_daemon`: Lanza `cargo run -p sf-proxy --bin sf_proxy` desacoplado a log (`sf_proxy.log`).
  - `start_spa_daemon`: Lanza `pnpm --dir frontend dev --port 5173 --host 127.0.0.1` desacoplado a log (`sf_spa.log`).
  - `deploy_full_stack` (Opción 1 renovada):
    1. Verifica prerrequisitos (Cargo + pnpm + .env).
    2. Comprueba LanceDB (hidrata los 55 locks si está vacío).
    3. Inicia `sf-proxy` en `:8088` y aguarda healthcheck.
    4. Inicia SPA en `:5173` y aguarda healthcheck.
    5. Abre navegador en `http://127.0.0.1:5173`.
  - `stop_all_services` (Opción 10 renovada):
    - Mata limpiamente procesos de `:8088`, `:5173` y `:3000` con `kill` progresivo y `kill -9` de respaldo.
* **2.3. Menú Principal Renovado:**
  - Panel superior mostrando el estado en vivo de los **tres** servicios:
    - `[Gateway sf-proxy :8088]`
    - `[Frontend SPA :5173]`
    - `[Diagnostic Studio :3000]`

---

### Slice 3: Quality Gates y Suite de Tests Unificada
* **3.1. Tests Completos (Opción 3):**
  - Ejecutar `cargo test --workspace` (Rust, 5 crates).
  - Ejecutar `pnpm --dir frontend test` (Vitest, 34 tests).
  - Reportar resumen unificado.
* **3.2. Linters Estrictos (Opción 4):**
  - Rust: `cargo fmt --all -- --check` y `cargo clippy --workspace --all-targets -- -D warnings`.
  - Frontend: `pnpm --dir frontend tsc -b` y `pnpm --dir frontend oxlint`.

---

### Slice 4: Módulo de Guía Interactiva para la WebGUI
* **4.1. Asistente Paso a Paso en Terminal:**
  - Agregar opción en el menú (o flujo guiado post-despliegue): `Guía de Pruebas en WebGUI`.
  - Muestra tarjetas formateadas en terminal con las acciones exactas a ejecutar en `http://127.0.0.1:5173`:
    1. **Prueba PASS:** Clic en chip `Python generator` $\implies$ Observar stream SSE letra por letra.
    2. **Prueba BREACH L1:** Clic en chip `Risotto recipe` $\implies$ Ficha Rayos X: L1 Coseno debajo de umbral ($0.50$). L2 y L3 skipped.
    3. **Prueba BREACH L2:** Clic en chip `Piggybacked clause` $\implies$ Ficha Rayos X: L1 pasa, L2 marca la cláusula culinaria en rojo.
    4. **Prueba BREACH L3:** Clic en chip `Spectral noise` $\implies$ Ficha Rayos X: L1 y L2 pasan, L3 muestra quórum insuficiente ($V < K=103$).
    5. **Ajuste en Caliente:** Activar switch "Apply overrides" y mover slider L1 a $0.30$ $\implies$ El prompt de risotto ahora pasa al upstream.
    6. **Upstream Real:** Cargar clave de DeepSeek / Gemini / OpenAI en el modal de configuración $\implies$ El request se reenvía al proveedor real conservando la clave en `localStorage`.

---

## 3. Criterios de Aceptación (Definición de Terminado)

1. **Autonomía Operativa**: El usuario ejecuta `./setup.sh`, elige opción `1`, y en menos de 10 segundos tiene el Gateway y la SPA corriendo, LanceDB hidratado y la pestaña del navegador abierta en `:5173` lista para chatear.
2. **Cero Zombies**: La opción `10` mata todos los procesos asociados a `:8088`, `:5173` y `:3000` sin dejar puertos tomados.
3. **Idempotencia Absoluta**: Correr la opción `1` cuando los servicios ya están corriendo no duplica procesos ni crashea; simplemente confirma que están saludables y abre el browser.
4. **Toolchain Limpio**: Cero mezclas de gestores (`pnpm` exclusivo en `frontend/`, `cargo` exclusivo en crates Rust).
5. **Alineación con `transparent-dev-protocol`**: Todo cambio pasa por branch dedicado, tests locales en verde, y parada en el approval gate.
