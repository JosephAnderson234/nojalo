# Nojalo

> ¿Cuánto necesito en el final para no jalar el curso?

App de escritorio para registrar calificaciones, escrita **toda en Rust**:

- **`core/`**: el motor de notas. Es Rust puro, sin dependencias de UI y con tests.
- **`src/`**: la UI en [Leptos](https://leptos.dev) (reactividad fina), compilada a WebAssembly con [Trunk](https://trunkrs.dev).
- **`src-tauri/`**: el shell de escritorio en [Tauri 2](https://tauri.app). Guarda los cursos en SQLite.

El mismo crate `nojalo-core` corre en WASM (UI) y en nativo (backend/tests).

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
- **Ponderado por créditos** de cada periodo.

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

Los datos se guardan en `%APPDATA%/app.nojalo/nojalo.db` en Windows (en macOS y Linux, en la carpeta de datos equivalente).
