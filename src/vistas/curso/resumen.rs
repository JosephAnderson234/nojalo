//! Medidor de la nota, veredicto, KPIs y simulador del curso abierto.

use leptos::prelude::*;
use nojalo_core::*;

use super::Vista;
use crate::formato::{corto, fmt, num, pct};
use crate::iconos::Icono;

/// Fracción del arco del medidor (240° de los 360° de la circunferencia).
pub const ARCO: f64 = 240.0 / 360.0 * 100.0;

/// Punto sobre el medidor: el arco arranca abajo a la izquierda (150°) y gira 240°.
pub fn punto_arco(frac: f64, r: f64) -> (f64, f64) {
    let a = (150.0 + 240.0 * frac.clamp(0.0, 1.0)).to_radians();
    (60.0 + r * a.cos(), 60.0 + r * a.sin())
}

#[component]
pub fn Resumen() -> impl IntoView {
    let v = expect_context::<Vista>();
    let simulado = RwSignal::new(None::<f64>);

    let dato = move |f: fn(&Curso, &Informe) -> String| {
        move || {
            v.curso.with(|c| {
                v.informe.with(|i| match (c, i) {
                    (Some(c), Some(i)) => f(c, i),
                    _ => String::new(),
                })
            })
        }
    };
    let max = move || v.curso.with(|c| c.as_ref().map_or(20.0, |c| c.escala_max));
    let dec = move || v.curso.with(|c| c.as_ref().map_or(2, |c| c.decimales));
    let apr = move || v.curso.with(|c| c.as_ref().map_or(0.0, |c| c.aprobatoria));
    let frac = move |x: f64| (x / max()).clamp(0.0, 1.0);
    let actual = move || v.informe.with(|i| i.as_ref().and_then(|i| i.actual));
    let minimo = move || v.informe.with(|i| i.as_ref().map_or(0.0, |i| i.minimo));
    let maximo = move || v.informe.with(|i| i.as_ref().map_or(0.0, |i| i.maximo));
    let avance = move || v.informe.with(|i| i.as_ref().map_or(0.0, |i| i.avance));
    let estado = move || v.informe.with(|i| i.as_ref().map(|i| i.estado));
    let clase_estado = move || match estado() {
        Some(Estado::Aprobado) => "ok",
        Some(Estado::Desaprobado) => "mal",
        _ => "juego",
    };
    let sim_x = move || simulado.get().unwrap_or_else(|| v.informe.with(|i| i.as_ref().and_then(|i| i.necesaria)).unwrap_or(max() / 2.0));
    let sim_final = move || v.curso.with(|c| c.as_ref().and_then(|c| simular(c, sim_x()).map(|n| fmt(n, c.decimales))));

    let marca_apr = move |r: f64| punto_arco(frac(apr()), r);

    view! {
        <section class=move || format!("hero {}", clase_estado())>
            <div
                class="medidor"
                role="img"
                aria-label=dato(|c, i| {
                    let d = c.decimales;
                    let actual = i.actual.map(|x| format!("nota actual {}; ", fmt(x, d))).unwrap_or_default();
                    format!(
                        "Escala de 0 a {}: {actual}rango posible de {} a {}; se aprueba con {}",
                        corto(c.escala_max),
                        fmt(i.minimo, d),
                        fmt(i.maximo, d),
                        corto(c.aprobatoria),
                    )
                })
            >
                <svg viewBox="0 0 120 120" aria-hidden="true">
                    <defs>
                        <linearGradient id="grad-valor" x1="0" y1="1" x2="1" y2="0">
                            <stop offset="0%" class="g0"></stop>
                            <stop offset="100%" class="g1"></stop>
                        </linearGradient>
                    </defs>
                    <circle
                        class="pista"
                        cx="60"
                        cy="60"
                        r="50"
                        pathLength="100"
                        stroke-dasharray=format!("{:.3} 100", ARCO)
                        transform="rotate(150 60 60)"
                    ></circle>
                    <circle
                        class="rango"
                        cx="60"
                        cy="60"
                        r="50"
                        pathLength="100"
                        stroke-dasharray=move || format!("{:.3} 100", (frac(maximo()) - frac(minimo())).max(0.0) * ARCO)
                        stroke-dashoffset=move || format!("{:.3}", -frac(minimo()) * ARCO)
                        transform="rotate(150 60 60)"
                    ></circle>
                    <circle
                        class="valor"
                        cx="60"
                        cy="60"
                        r="50"
                        pathLength="100"
                        stroke="url(#grad-valor)"
                        stroke-dasharray=move || format!("{:.3} 100", actual().map_or(0.0, frac) * ARCO)
                        transform="rotate(150 60 60)"
                    ></circle>
                    <line
                        class="apr"
                        x1=move || format!("{:.2}", marca_apr(41.0).0)
                        y1=move || format!("{:.2}", marca_apr(41.0).1)
                        x2=move || format!("{:.2}", marca_apr(59.0).0)
                        y2=move || format!("{:.2}", marca_apr(59.0).1)
                    ></line>
                </svg>
                <div class="centro">
                    <span class="cifra">{move || actual().map(|x| fmt(x, dec())).unwrap_or_else(|| "—".into())}</span>
                    <span class="de">{move || format!("nota actual · de {}", corto(max()))}</span>
                    <span class="pildora">
                        {move || match estado() {
                            Some(Estado::Aprobado) => "Aprobado",
                            Some(Estado::Desaprobado) => "Jalado",
                            _ => "En juego",
                        }}
                    </span>
                </div>
                <span class="extremo izq" aria-hidden="true">"0"</span>
                <span class="extremo der" aria-hidden="true">{move || corto(max())}</span>
            </div>

            <div class="hero-info">
                <div class="veredicto">
                    <span class="rotulo">"Para aprobar"</span>
                    <span class="cifra-xl">
                        {dato(|c, i| match i.estado {
                            Estado::Aprobado => "Asegurado".into(),
                            Estado::Desaprobado => "No alcanza".into(),
                            Estado::EnJuego => i.necesaria.map(|x| fmt(x, c.decimales)).unwrap_or_default(),
                        })}
                    </span>
                    <span class="sub">
                        {dato(|c, i| match i.estado {
                            Estado::Aprobado => format!("ya tienes ≥ {} pase lo que pase", corto(c.aprobatoria)),
                            Estado::Desaprobado => format!("ni con {} en todo llegas a {}", corto(c.escala_max), corto(c.aprobatoria)),
                            Estado::EnJuego => "de promedio en todo lo pendiente".into(),
                        })}
                    </span>
                </div>

                <div class="kpis">
                    <div class="kpi">
                        <span class="rotulo">"Rango posible"</span>
                        <span class="valor-kpi">{move || format!("{} – {}", fmt(minimo(), dec()), fmt(maximo(), dec()))}</span>
                        <span class="sub">"con 0 o el máximo en lo pendiente"</span>
                    </div>
                    <div class="kpi">
                        <span class="rotulo">"Avance"</span>
                        <span class="valor-kpi">{move || pct(avance())}</span>
                        <div class="barra-avance" aria-hidden="true">
                            <i style:width=move || format!("{:.1}%", avance().clamp(0.0, 1.0) * 100.0)></i>
                        </div>
                    </div>
                    <div class="kpi">
                        <span class="rotulo">"Aprueba con"</span>
                        <span class="valor-kpi">{move || corto(apr())}</span>
                        <span class="sub">{move || format!("en escala 0 – {}", corto(max()))}</span>
                    </div>
                </div>

                <Show when=move || estado() == Some(Estado::EnJuego)>
                    <div class="simulador">
                        <div class="sim-texto">
                            <label for="simulador">"Si saco"</label>
                            <strong>{move || corto(sim_x())}</strong>
                            <span>"en todo lo pendiente, termino con"</span>
                            <strong class="sim-final" aria-live="polite">{sim_final}</strong>
                        </div>
                        <input
                            id="simulador"
                            type="range"
                            min="0"
                            max=move || max().to_string()
                            step="0.5"
                            aria-valuetext=move || format!("{} en todo lo pendiente", corto(sim_x()))
                            style:background-size=move || format!("{:.1}% 100%", frac(sim_x()) * 100.0)
                            prop:value=move || sim_x().to_string()
                            on:input=move |ev| simulado.set(num(&event_target_value(&ev)))
                        />
                    </div>
                </Show>

                <div class="leyenda" aria-hidden="true">
                    <span><i class="muestra actual"></i>"nota actual"</span>
                    <span><i class="muestra rango"></i>"rango posible"</span>
                    <span><i class="muestra apr"></i>"nota aprobatoria"</span>
                </div>
            </div>
        </section>
    }
}

#[component]
pub fn Advertencias() -> impl IntoView {
    let v = expect_context::<Vista>();
    let lista = move || v.informe.with(|i| i.as_ref().map(|i| i.advertencias.clone()).unwrap_or_default());
    view! {
        <Show when=move || !lista().is_empty()>
            <div class="advertencias" role="status">
                <Icono nombre="alerta" tam=18 />
                <ul>{move || lista().into_iter().map(|a| view! { <li>{a}</li> }).collect_view()}</ul>
            </div>
        </Show>
    }
}
