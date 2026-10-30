# RIGOR EPISTÉMICO

Modo de auditoría. No es el ciclo de implementación.

Leé este archivo solo si el usuario pide separar lo comprobado de lo inferido. Un bug sigue [debugging.md](./debugging.md). Una review de diff sigue [qa-review.md](./qa-review.md). Este modo no las reemplaza: puede rotular sus afirmaciones, nada más.

## Cuándo entra

Entrá solo con un trigger explícito:

- **Comandos directos**: `/analisis`, `/audit`, `/grounding`, `/certeza`, `/factcheck`
- **Frases coloquiales de grounding / pedido de certeza (multi-palabra)**:
  - "a ver, decime bien"
  - "pará pará" / "pará un poco, que no me queda claro"
  - "revisá bien" / "miralo con lupa"
  - "separame los tantos" / "qué es posta y qué estás deduciendo"
  - "bajame a tierra qué sabemos y qué no"
  - "fact-check yourself" / "hold on, verify this"

**Regla de exclusión**: NO entres por verbos o palabras sueltas ("revisá", "miralo", "pará", "hacelo bien") dentro de una implementación estándar. Esos verbos forman parte del trabajo cotidiano de desarrollo e indican prestar atención al flujo o preguntar si hay ambigüedad sin congelar el ciclo ni generar etiquetas epistémicas.

## Qué hacer

1. Frená el código y el cierre de diseño hasta terminar el mapa. Si después piden el parche o la implementación, salí de este modo y volvé al flujo normal.
2. Partí el problema en afirmaciones atómicas. Cada una lleva **una** etiqueta y la evidencia al lado: `archivo:línea`, comando corrido con su salida, o la cita textual de una frase.
3. Si la etiqueta no cierra con evidencia incontrovertible, es `[SIN VERIFICAR]`. No la subas de rango para completar el mapa.
4. Cerrá con la matriz de tres columnas. Sin preámbulo ni conclusiones genéricas.

En una auditoría citá `archivo:línea`. Eso no se traslada a issues durables: ahí manda [qa-review.md](./qa-review.md) §1 (comportamiento, sin paths ni líneas frágiles).

## Las 8 etiquetas

| Tag | Es | No es |
| :--- | :--- | :--- |
| `[HECHO]` | Invariante estático que se puede abrir y señalar (path, línea, versión, texto). | Una política, una intención o un comportamiento dinámico de runtime. |
| `[VERIFICADO]` | Lo corriste y viste la salida (test en verde, comando ejecutado, request real). | "Lo leí en un markdown" o "el usuario lo decidió". Leer un archivo produce `[HECHO]` sobre el texto, no verificación de que la regla sea correcta. |
| `[INFERENCIA]` | Deducción lógica basada en evidencia. La cadena causal va escrita en la misma afirmación. | Una corazonada sin cadena explicativa. |
| `[SIN VERIFICAR]` | Afirmación plausible que todavía no contrastaste empíricamente. | Un permiso para seguir construyendo como si fuera cierta. |
| `[HIPÓTESIS]` | Predicción falsable: *"Si X es la causa, intervenir Y producirá Z"*. | Una inferencia o una asunción sin intervención predicha. |
| `[ASUNCIÓN]` | Premisa operativa de trabajo por omisión. Se declara explícitamente para que se pueda refutar. | Un hecho que preferís no chequear. |
| `[DECISIÓN]` | Trade-off deliberado adoptado (ej. latencia vs. memoria, merge directo vs. PR, pure Rust vs. bindings). | Algo verificado. Una decisión no se etiqueta `[VERIFICADO]`. |
| `[RIESGO]` | Efecto colateral, punto ciego, acoplamiento o riesgo de regresión/drift. | Una tarea ya hecha. |

## Matriz de cierre

| Sabemos | Deducimos | Sigue abierto |
| :--- | :--- | :--- |
| `[HECHO]`, `[VERIFICADO]` | `[INFERENCIA]`, `[DECISIÓN]` | `[SIN VERIFICAR]`, `[HIPÓTESIS]`, `[ASUNCIÓN]`, `[RIESGO]` |

Una fila por afirmación, con la evidencia en la celda. Si una columna queda vacía, dejala vacía.
