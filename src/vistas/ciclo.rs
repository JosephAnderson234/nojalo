//! Vista de un ciclo: promedio ponderado por créditos y tabla de cursos.

use leptos::prelude::*;
use nojalo_core::*;

use crate::componentes::{BotonImportar, Numero};
use crate::estado::{nuevo_id, Global};
use crate::formato::{corto, fmt, nombre_archivo, nombre_ciclo};
use crate::iconos::Icono;

#[component]
pub fn VistaCiclo(ciclo: String) -> impl IntoView {
    let app = expect_context::<Global>();
    let nombre = StoredValue::new(ciclo.clone());
    let resumen = Memo::new(move |_| {
        let c = nombre.get_value();
        app.cursos.with(|v| resumen_ciclo(cursos_del_ciclo(v, &c)))
    });
    let r = move |f: fn(&ResumenCiclo) -> String| move || resumen.with(f);
    let ponderado = move || resumen.with(|r| r.ponderado.map(|x| fmt(x, 2)).unwrap_or_else(|| "—".into()));

    let exportar_ciclo = move |_| {
        let c = nombre.get_value();
        let json = app.cursos.with_untracked(|v| paquete(&c, cursos_del_ciclo(v, &c)).a_json());
        app.exportar(nombre_archivo(&format!("ciclo {}", nombre_ciclo(&c)), ".esquemas.json"), json);
    };

    view! {
        <div class="editor ciclo-vista">
            <header class="cabecera">
                <div class="linea-titulo">
                    <span class="icono-titulo">
                        <Icono nombre="ciclo" tam=22 />
                    </span>
                    <input
                        class="titulo"
                        aria-label="Nombre del ciclo"
                        placeholder="Sin ciclo"
                        prop:value=ciclo.clone()
                        on:change=move |ev| {
                            let campo: web_sys::HtmlInputElement = event_target(&ev);
                            let nuevo = campo.value().trim().to_string();
                            let viejo = nombre.get_value();
                            if nuevo == viejo {
                                return;
                            }
                            let existe = app.cursos.with_untracked(|v| v.iter().any(|c| c.periodo == nuevo));
                            if existe {
                                campo.set_value(&viejo);
                                let mensaje = format!(
                                    "Ya existe el ciclo «{}». Los cursos de «{}» pasarán a ese ciclo.",
                                    nombre_ciclo(&nuevo),
                                    nombre_ciclo(&viejo),
                                );
                                app.confirmar("¿Juntar ciclos?", mensaje, "Juntar", move || {
                                    app.renombrar_ciclo(viejo.clone(), nuevo.clone())
                                });
                            } else {
                                app.renombrar_ciclo(viejo, nuevo);
                            }
                        }
                    />
                    <div class="acciones-curso">
                        <button on:click=move |_| app.crear(Curso::nuevo(nuevo_id(), "Nuevo curso", &nombre.get_value()))>
                            <Icono nombre="mas" tam=14 />
                            "Curso"
                        </button>
                        <BotonImportar
                            titulo="Crear cursos en este ciclo a partir de esquemas (.json)"
                            al_leer=Callback::new(move |t: String| app.importar(&t, nombre.get_value()))
                        >
                            <Icono nombre="importar" tam=14 />
                            "Importar"
                        </BotonImportar>
                        <button
                            title="Guardar los esquemas (sin notas) de todos los cursos del ciclo en un archivo"
                            disabled=move || resumen.with(|r| r.cursos.is_empty())
                            on:click=exportar_ciclo
                        >
                            <Icono nombre="exportar" tam=14 />
                            "Exportar esquemas"
                        </button>
                    </div>
                </div>
                <p class="meta">
                    {r(|r| {
                        let n = r.cursos.len();
                        format!("{n} curso{} · {} créditos", if n == 1 { "" } else { "s" }, corto(r.creditos))
                    })}
                </p>
            </header>

            <section class="hero ciclo-hero">
                <div class="pond-bloque">
                    <span class="rotulo">"Promedio ponderado"</span>
                    <span class="cifra">{ponderado}</span>
                    <span class="pildora">
                        {r(|r| format!("{} de {} créditos con nota", corto(r.creditos_con_nota), corto(r.creditos)))}
                    </span>
                </div>
                <div class="hero-info">
                    <div class="formula" role="math" aria-label="Promedio ponderado: suma de nota por créditos entre suma de créditos">
                        <div class="fraccion">
                            <span class="num-f">"Σ (nota × créditos)"</span>
                            <span class="den-f">"Σ créditos"</span>
                        </div>
                        <span class="igual">"="</span>
                        <div class="fraccion">
                            <span class="num-f">{r(|r| corto(r.puntos))}</span>
                            <span class="den-f">{r(|r| corto(r.creditos_con_nota))}</span>
                        </div>
                        <span class="igual">"="</span>
                        <strong class="resultado">{ponderado}</strong>
                    </div>
                    <div class="kpis">
                        <div class="kpi">
                            <span class="rotulo">"Proyección"</span>
                            <span class="valor-kpi">
                                {r(|r| match (r.ponderado_minimo, r.ponderado_maximo) {
                                    (Some(a), Some(b)) => format!("{} – {}", fmt(a, 2), fmt(b, 2)),
                                    _ => "—".into(),
                                })}
                            </span>
                            <span class="sub">"con 0 o el máximo en todo lo pendiente"</span>
                        </div>
                        <div class="kpi">
                            <span class="rotulo">"Créditos asegurados"</span>
                            <span class="valor-kpi">{r(|r| format!("{} / {}", corto(r.creditos_aprobados), corto(r.creditos)))}</span>
                            <div class="barra-avance" aria-hidden="true">
                                <i style:width=move || {
                                    resumen.with(|r| {
                                        let p = if r.creditos > 0.0 { r.creditos_aprobados / r.creditos } else { 0.0 };
                                        format!("{:.1}%", p.clamp(0.0, 1.0) * 100.0)
                                    })
                                }></i>
                            </div>
                        </div>
                        <div class="kpi">
                            <span class="rotulo">"Créditos jalados"</span>
                            <span class="valor-kpi">{r(|r| corto(r.creditos_jalados))}</span>
                            <span class="sub">
                                {r(|r| {
                                    let en_juego = r.creditos - r.creditos_aprobados - r.creditos_jalados;
                                    format!("{} en juego", corto(en_juego.max(0.0)))
                                })}
                            </span>
                        </div>
                    </div>
                </div>
            </section>

            <section class="tabla-ciclo" aria-label="Cursos del ciclo">
                <div class="fila-ciclo encabezado" aria-hidden="true">
                    <span>"Curso"</span>
                    <span class="der">"Créditos"</span>
                    <span class="der">"Nota"</span>
                    <span class="der">"Nota × créditos"</span>
                    <span class="centro-t col-rango">"Rango posible"</span>
                    <span>"Estado"</span>
                </div>
                <Show when=move || resumen.with(|r| r.cursos.is_empty())>
                    <p class="vacio">
                        "Este ciclo aún no tiene cursos. Crea uno con «+ Curso» o importa los esquemas de tu malla; "
                        "el ciclo se guarda al agregar su primer curso. Puedes cambiarle el nombre arriba."
                    </p>
                </Show>
                <For
                    each=move || resumen.with(|r| r.cursos.iter().map(|c| c.id.clone()).collect::<Vec<_>>())
                    key=|id| id.clone()
                    children=move |id| view! { <FilaCiclo id=id resumen=resumen /> }
                />
                <div class="fila-ciclo total">
                    <span>"Total"</span>
                    <span class="der" title="Σ créditos de los cursos con nota">{r(|r| corto(r.creditos_con_nota))}</span>
                    <span></span>
                    <span class="der" title="Σ (nota × créditos)">{r(|r| corto(r.puntos))}</span>
                    <span class="col-rango"></span>
                    <span class="fuerte">{ponderado}</span>
                </div>
            </section>
            <p class="nota-al-pie">
                "Solo cuentan los cursos que ya tienen alguna nota (con su nota actual); los de 0 créditos no pesan. "
                "La proyección incluye todos los cursos del ciclo."
            </p>
        </div>
    }
}

#[component]
pub fn FilaCiclo(id: String, resumen: Memo<ResumenCiclo>) -> impl IntoView {
    let app = expect_context::<Global>();
    let id = StoredValue::new(id);
    let fila = Memo::new(move |_| resumen.with(|r| id.with_value(|id| r.cursos.iter().find(|c| &c.id == id).cloned())));
    let txt = move |f: fn(&NotaCiclo) -> String| move || fila.with(|c| c.as_ref().map(f).unwrap_or_default());
    let estado = move || {
        fila.with(|c| match c.as_ref().map(|c| c.estado) {
            Some(Estado::Aprobado) => "ok",
            Some(Estado::Desaprobado) => "mal",
            _ => "juego",
        })
    };

    view! {
        <div class="fila-ciclo" data-estado=estado>
            <button class="enlace-curso" title="Abrir el curso" on:click=move |_| app.abrir_curso(id.get_value())>
                <span class="nombre">{txt(|c| c.nombre.clone())}</span>
                <Icono nombre="flecha" tam=14 />
            </button>
            <Numero
                valor=Signal::derive(move || fila.with(|c| c.as_ref().map(|c| c.creditos)))
                al_cambiar=Callback::new(move |x: Option<f64>| {
                    let v = x.unwrap_or(0.0).max(0.0);
                    let id = id.get_value();
                    app.editar_donde(|c| c.id == id, |c| c.creditos = v)
                })
                clase="creditos"
                etiqueta=Signal::derive(move || format!("Créditos de {}", fila.with(|c| c.as_ref().map(|c| c.nombre.clone()).unwrap_or_default())))
            />
            <span class="der nota-ciclo">{txt(|c| c.nota.map(|n| fmt(n, c.decimales)).unwrap_or_else(|| "sin notas".into()))}</span>
            <span class="der mono">{txt(|c| c.puntos().map(corto).unwrap_or_else(|| "—".into()))}</span>
            <span class="centro-t mono tenue col-rango">{txt(|c| format!("{} – {}", fmt(c.minimo, c.decimales), fmt(c.maximo, c.decimales)))}</span>
            <span>
                <span class="estado-chip">
                    {txt(|c| match c.estado {
                        Estado::Aprobado => "Aprobado".into(),
                        Estado::Desaprobado => "Jalado".into(),
                        Estado::EnJuego => "En juego".into(),
                    })}
                </span>
            </span>
        </div>
    }
}
