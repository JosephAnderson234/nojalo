//! Barra lateral: ciclos con sus cursos y el promedio acumulado.

use leptos::prelude::*;
use nojalo_core::*;

use crate::componentes::BotonImportar;
use crate::estado::{nuevo_id, Global};
use crate::formato::{corto, fmt, nombre_ciclo};
use crate::iconos::Icono;

#[component]
pub fn Barra() -> impl IntoView {
    let app = expect_context::<Global>();

    let grupos = move || {
        app.cursos.with(|v| {
            let mut lista = ciclos(v);
            // Un ciclo recién creado no tiene cursos todavía: se muestra mientras está abierto.
            if let Some(abierto) = app.ciclo.get() {
                lista.push(abierto);
                ordenar_ciclos(&mut lista);
            }
            lista
                .into_iter()
                .map(|p| {
                    let r = resumen_ciclo(cursos_del_ciclo(v, &p));
                    (p, r)
                })
                .collect::<Vec<_>>()
        })
    };
    let acumulado = Memo::new(move |_| app.cursos.with(|v| resumen_ciclo(v.iter())));

    view! {
        <aside class="barra" aria-label="Cursos y ciclos">
            <div class="marca">
                <span class="logo">
                    <Icono nombre="logo" tam=20 />
                </span>
                "Nojalo"
            </div>
            <div class="fila-botones">
                <button
                    class="primario"
                    on:click=move |_| app.crear(Curso::nuevo(nuevo_id(), "Nuevo curso", &app.ciclo_destino()))
                >
                    <Icono nombre="mas" />
                    "Curso"
                </button>
                <button
                    class="icono-solo"
                    title="Agregar un curso de ejemplo con todas las reglas"
                    aria-label="Agregar un curso de ejemplo"
                    on:click=move |_| app.crear(Curso::ejemplo(nuevo_id(), &app.ciclo_destino()))
                >
                    <Icono nombre="libro" />
                </button>
                <BotonImportar
                    clase="icono-solo"
                    titulo="Importar esquemas de calificación (.json)"
                    etiqueta="Importar esquemas"
                    al_leer=Callback::new(move |t: String| app.importar(&t, app.ciclo_destino()))
                >
                    <Icono nombre="importar" />
                </BotonImportar>
            </div>
            <div class="ciclos-titulo">
                <span>"Ciclos"</span>
                <button class="mini-btn" title="Crear un ciclo nuevo" on:click=move |_| app.nuevo_ciclo()>
                    <Icono nombre="mas" tam=12 />
                    "Ciclo"
                </button>
            </div>
            <nav class="ciclos" aria-label="Ciclos">
                {move || {
                    grupos()
                        .into_iter()
                        .map(|(p, r)| {
                            let p_activo = p.clone();
                            let p_click = p.clone();
                            let ciclo_activo = move || app.ciclo.get().as_deref() == Some(p_activo.as_str());
                            view! {
                                <section class="periodo">
                                    <button
                                        class="ciclo-cabecera"
                                        class:activo=ciclo_activo.clone()
                                        aria-current=move || ciclo_activo().then_some("page")
                                        title="Ver el promedio ponderado del ciclo"
                                        on:click=move |_| app.abrir_ciclo(p_click.clone())
                                    >
                                        <Icono nombre="ciclo" tam=13 />
                                        <span class="ciclo-nombre">{nombre_ciclo(&p)}</span>
                                        <span class="pond">
                                            {r.ponderado.map(|x| fmt(x, 2)).unwrap_or_else(|| "—".into())}
                                        </span>
                                    </button>
                                    {r
                                        .cursos
                                        .into_iter()
                                        .map(|c| {
                                            let id = c.id.clone();
                                            let id_click = c.id.clone();
                                            let (clase, texto_estado) = match c.estado {
                                                Estado::Aprobado => ("ok", "aprobado"),
                                                Estado::Desaprobado => ("mal", "jalado"),
                                                Estado::EnJuego => ("juego", "en juego"),
                                            };
                                            let activo = move || {
                                                app.ciclo.get().is_none() && app.sel.get().as_deref() == Some(id.as_str())
                                            };
                                            view! {
                                                <button
                                                    class="curso-item"
                                                    class:activo=activo.clone()
                                                    aria-current=move || activo().then_some("page")
                                                    title=format!("{} ({texto_estado})", c.nombre)
                                                    on:click=move |_| app.abrir_curso(id_click.clone())
                                                >
                                                    {mini_anillo(c.nota.map_or(0.0, |x| x / c.escala_max), clase)}
                                                    <span class="sr">{format!("{texto_estado}: ")}</span>
                                                    <span class="nombre">{c.nombre.clone()}</span>
                                                    <span class="nota">
                                                        {c.nota.map(|x| fmt(x, c.decimales)).unwrap_or_else(|| "—".into())}
                                                    </span>
                                                </button>
                                            }
                                        })
                                        .collect_view()}
                                </section>
                            }
                        })
                        .collect_view()
                }}
            </nav>
            <Show when=move || acumulado.with(|r| !r.cursos.is_empty())>
                <div class="acumulado" title="Σ(nota × créditos) / Σ créditos de todos tus ciclos">
                    <span class="rotulo">"Promedio acumulado"</span>
                    <span class="cifra-acum">
                        {move || acumulado.with(|r| r.ponderado.map(|x| fmt(x, 2)).unwrap_or_else(|| "—".into()))}
                    </span>
                    <span class="sub">
                        {move || {
                            acumulado
                                .with(|r| {
                                    format!(
                                        "{} créditos con nota · {} asegurados",
                                        corto(r.creditos_con_nota),
                                        corto(r.creditos_aprobados),
                                    )
                                })
                        }}
                    </span>
                </div>
            </Show>
        </aside>
    }
}

/// Anillo pequeño para la barra lateral.
pub fn mini_anillo(frac: f64, clase: &'static str) -> impl IntoView {
    view! {
        <svg class=format!("mini {clase}") viewBox="0 0 24 24" width="22" height="22" aria-hidden="true">
            <circle class="pista" cx="12" cy="12" r="9" pathLength="100"></circle>
            <circle
                class="valor"
                cx="12"
                cy="12"
                r="9"
                pathLength="100"
                stroke-dasharray=format!("{:.2} 100", frac.clamp(0.0, 1.0) * 100.0)
                transform="rotate(-90 12 12)"
            ></circle>
        </svg>
    }
}
