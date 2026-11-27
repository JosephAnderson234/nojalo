<p align="center">
  <img src="assets/logo.svg" alt="Logo de Nojalo: un medidor de nota que pasa la marca aprobatoria, con un check" width="128" height="128">
</p>

<h1 align="center">Nojalo</h1>

<p align="center"><em>¿Cuánto necesito en el final para no jalar el curso?</em></p>

App de escritorio para registrar calificaciones, escrita **toda en Rust**:

- **`core/`**: el motor de notas. Es Rust puro, sin dependencias de UI y con tests.
- **`src/`**: la UI en [Leptos](https://leptos.dev) (reactividad fina), compilada a WebAssembly con [Trunk](https://trunkrs.dev).
  - `estado.rs`: estado global (señales) y las operaciones que lo cambian y persisten.
  - `api/`: persistencia; `tauri.rs` (SQLite vía comandos) y `navegador.rs` (localStorage).
  - `vistas/`: barra lateral, bienvenida, ciclo y editor de curso (`curso/`: resumen y árbol).
  - `componentes/`: piezas reutilizables (campo numérico, diálogo, avisos, importar).
- **`src-tauri/`**: el shell de escritorio en [Tauri 2](https://tauri.app). Guarda los cursos en SQLite.

El mismo crate `nojalo-core` corre en WASM (UI) y en nativo (backend/tests).

## Descarga

Instaladores para Windows y Linux en [Releases](https://github.com/JosephAnderson234/nojalo/releases/latest). No están firmados: en Windows elige *Más información → Ejecutar de todas formas*.

Versión web: [josephanderson234.github.io/nojalo](https://josephanderson234.github.io/nojalo/). Se publica sola con cada push a `main` (workflow `pages.yml`). En la web los datos se guardan en el navegador (localStorage), no se sincronizan con la app de escritorio.

Para publicar una versión: sube `version` en `src-tauri/tauri.conf.json` y empuja un tag `vX.Y.Z`; el workflow `release.yml` compila y publica los instaladores.

## Modelo

Un curso es un **árbol**. Cada nodo es una *nota* o un *grupo* (los grupos pueden contener otros grupos).

| Regla | Cómo funciona |
|---|---|
| **Peso `auto`** | Reparte en partes iguales lo que dejan libre los hermanos con peso explícito. |
| **Peso `% grupo`** | Porcentaje relativo al grupo padre. |
| **Peso `% final`** | Porcentaje relativo a la nota final del curso. Se convierte solo a peso local dividiendo por el peso efectivo del padre. Se puede mezclar con `% grupo` en el mismo árbol. |
| **Eliminar menores** | Un grupo descarta sus *n* notas más bajas. Mientras haya notas pendientes en el grupo, solo se descartan las que con seguridad saldrán (una pendiente podría terminar siendo la eliminada). |
| **Candado (nota mínima)** | Si un nodo queda por debajo de `mínimo`, la nota de su grupo o del curso completo se limita a `tope`. Por ejemplo: «si el examen final < 10.5, la nota final es como máximo 10». |

Para cada curso se calcula:

- **Nota actual**: solo con lo rendido, con los pesos renormalizados.
- **Rango posible**: el peor caso (lo pendiente vale 0) y el mejor caso (lo pendiente vale el máximo).
- **Nota necesaria**: el promedio que hace falta en todo lo pendiente para aprobar. Se obtiene por bisección, porque la nota final es monótona.
- **Simulador**: «si saco X en lo que falta, termino con Y».
## Ciclos

Los cursos se agrupan por **ciclo** (por ejemplo `2026-2`). Al hacer clic en un ciclo de la barra lateral se abre su resumen:

- **Promedio ponderado**: `Σ(nota × créditos) / Σ créditos`. Solo cuentan los cursos que ya tienen alguna nota, con su nota actual. Los cursos de 0 créditos no pesan.
- **Proyección**: el ponderado en el peor y en el mejor caso, contando todos los cursos.
- Créditos asegurados, jalados y en juego. Los créditos se pueden editar desde la tabla.
- Para renombrar el ciclo se edita su título; el cambio se aplica a todos sus cursos.

Al pie de la barra lateral se muestra el **promedio acumulado** de todos los ciclos.

## Esquemas (exportar e importar)

Un **esquema** es la estructura de un curso **sin notas**: evaluaciones, grupos, pesos, candados, eliminación de menores, créditos, escala y nota aprobatoria. Sirve para compartir con compañeros cómo se califica un curso.

| Acción | Dónde |
|---|---|
| Exportar el esquema de un curso | Botón *Exportar esquema* en el curso (`<curso>.esquema.json`) |
| Exportar todos los esquemas de un ciclo | Botón *Exportar esquemas* en la vista del ciclo (`ciclo-<x>.esquemas.json`) |
| Crear cursos desde un archivo | Botón de importar de la barra lateral, de la vista del ciclo o de la pantalla de bienvenida |
| Reemplazar la estructura de un curso | *Aplicar esquema* en el curso (pide confirmación: se borran las notas) |

Formato: JSON con `"formato": "nojalo/esquema"` (un curso) o `"nojalo/paquete"` (varios), y `"version": 1`. Al importar también se acepta el JSON interno de un curso: se le quitan las notas. Antes de crear nada, los archivos se validan: escala, nota aprobatoria, pesos y candados.

## Requisitos

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk tauri-cli --locked
```

En Windows funcionan las toolchains `-gnu` (MinGW/MSYS2) y `-msvc` (requiere VS Build Tools). WebView2 ya viene con Windows 11.

## Uso

```sh
cargo tauri dev          # app de escritorio con recarga en caliente
trunk serve              # solo la UI en http://localhost:1420 (guarda en localStorage)
cargo test -p nojalo-core
cargo tauri build        # instalador
```

Los datos se guardan en `%APPDATA%/app.nojalo/nojalo.db` en Windows (en Linux, en `~/.local/share/app.nojalo/`).
