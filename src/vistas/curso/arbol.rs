//! Árbol editable de notas y grupos.

use leptos::prelude::*;
use nojalo_core::*;

use super::Vista;
use crate::componentes::Numero;
use crate::estado::Global;
use crate::formato::{corto, fmt, pct};
use crate::iconos::Icono;

#[component]
pub fn Hijos(padre: Id, nivel: usize) -> AnyView {
    let v = expect_context::<Vista>();
    let ids = move || v.curso.with(|c| c.as_ref().map(|c| c.hijos_ids(padre)).unwrap_or_default());
    view! { <For each=ids key=|id| *id children=move |id| view! { <Fila id=id nivel=nivel /> } /> }
        .into_any()
}

#[component]
pub fn Agregar(padre: Id, nivel: usize) -> impl IntoView {
    let app = expect_context::<Global>();
    view! {
        <div class="agregar" style=format!("padding-left: {}px", 12 + nivel * 22)>
            <button on:click=move |_| app.editar(|c| { c.agregar(padre, false); })>
                <Icono nombre="mas" tam=14 />
                "nota"
            </button>
            <button on:click=move |_| app.editar(|c| { c.agregar(padre, true); })>
                <Icono nombre="carpeta" tam=14 />
                "grupo"
            </button>
        </div>
    }
}

#[component]
pub fn Fila(id: Id, nivel: usize) -> impl IntoView {
    let app = expect_context::<Global>();
    let v = expect_context::<Vista>();
    let nodo = Memo::new(move |_| v.curso.with(|c| c.as_ref().and_then(|c| c.buscar(id).cloned())));
    let res = Memo::new(move |_| v.informe.with(|i| i.as_ref().and_then(|i| i.nodos.get(&id).cloned())));
    let dec = move || v.curso.with(|c| c.as_ref().map_or(2, |c| c.decimales));
    let grupo = nodo.with_untracked(|n| n.as_ref().is_some_and(|n| n.es_grupo()));
    let abierto = RwSignal::new(false);
    let plegado = RwSignal::new(false);
    let nombre = Signal::derive(move || nodo.with(|n| n.as_ref().map(|n| n.nombre.clone()).unwrap_or_default()));
    let rotulo = move |pre: &'static str| Signal::derive(move || format!("{pre} {}", nombre.get()));

    let modo = move || {
        nodo.with(|n| match n.as_ref().map(|n| &n.peso) {
            Some(Peso::Grupo(_)) => "grupo",
            Some(Peso::Final(_)) => "final",
            _ => "auto",
        })
    };
    let peso_valor = Signal::derive(move || {
        nodo.with(|n| match n.as_ref().map(|n| &n.peso) {
            Some(Peso::Grupo(p) | Peso::Final(p)) => Some(*p),
            _ => None,
        })
    });
    let peso_sugerido = Signal::derive(move || res.with(|r| r.as_ref().map(|r| corto(r.peso.local * 100.0))).unwrap_or_default());
    let candado = move || nodo.with(|n| n.as_ref().and_then(|n| n.candado.clone()));
    let eliminar_menores = move || {
        nodo.with(|n| match n.as_ref().map(|n| &n.tipo) {
            Some(Tipo::Grupo { eliminar_menores, .. }) => *eliminar_menores,
            _ => 0,
        })
    };

    // Tono de la fila: sobre o bajo la nota aprobatoria (pendiente si no hay nota).
    let tono = move || {
        let apr = v.curso.with(|c| c.as_ref().map_or(0.0, |c| c.aprobatoria));
        res.with(|r| match r.as_ref().and_then(|r| r.actual) {
            Some(x) if x + 1e-9 >= apr => "sobre",
            Some(_) => "bajo",
            None => "pendiente",
        })
    };
    let escala = move || v.curso.with(|c| c.as_ref().map_or(20.0, |c| c.escala_max));

    let celda_nota = if grupo {
        view! {
            <span class="nota-grupo">
                <span class="pildora-nota">
                    {move || res.with(|r| r.as_ref().and_then(|r| r.actual).map(|x| fmt(x, dec())).unwrap_or_else(|| "—".into()))}
                </span>
                <span class="medida" aria-hidden="true">
                    <i style:width=move || {
                        let x = res.with(|r| r.as_ref().and_then(|r| r.actual)).unwrap_or(0.0);
                        format!("{:.1}%", (x / escala() * 100.0).clamp(0.0, 100.0))
                    }></i>
                </span>
            </span>
        }
        .into_any()
    } else {
        view! {
            <Numero
                valor=Signal::derive(move || {
                    nodo.with(|n| match n.as_ref().map(|n| &n.tipo) {
                        Some(Tipo::Nota { valor }) => *valor,
                        _ => None,
                    })
                })
                al_cambiar=Callback::new(move |x: Option<f64>| {
                    app.editar_nodo(id, |n| {
                        if let Tipo::Nota { valor } = &mut n.tipo {
                            *valor = x
                        }
                    })
                })
                placeholder=Signal::derive(|| "pendiente".to_string())
                clase="nota"
                etiqueta=rotulo("Nota de")
            />
        }
        .into_any()
    };

    view! {
        <div class="nodo" class:grupo=grupo>
            <div
                class="fila"
                class:eliminado=move || res.with(|r| r.as_ref().is_some_and(|r| r.eliminado))
                data-tono=tono
                data-nivel=nivel.min(3).to_string()
            >
                <span class="celda-nombre">
                    {(0..nivel).map(|_| view! { <span class="guia" aria-hidden="true"></span> }).collect_view()}
                    {if grupo {
                        view! {
                            <button
                                class="plegar"
                                aria-expanded=move || if plegado.get() { "false" } else { "true" }
                                aria-label=move || format!("{} {}", if plegado.get() { "Desplegar" } else { "Plegar" }, nombre.get())
                                on:click=move |_| plegado.update(|p| *p = !*p)
                            >
                                {move || {
                                    if plegado.get() {
                                        view! { <Icono nombre="cerrado" tam=14 /> }.into_any()
                                    } else {
                                        view! { <Icono nombre="abierto" tam=14 /> }.into_any()
                                    }
                                }}
                            </button>
                        }
                        .into_any()
                    } else {
                        view! { <span class="vineta" aria-hidden="true"></span> }.into_any()
                    }}
                    <input
                        class="nombre"
                        aria-label=if grupo { "Nombre del grupo" } else { "Nombre de la evaluación" }
                        prop:value=nombre
                        on:change=move |ev| {
                            let s = event_target_value(&ev);
                            app.editar_nodo(id, |n| n.nombre = s)
                        }
                    />
                </span>
                {celda_nota}
                <span class="rango-celda">
                    {move || {
                        res.with(|r| match r {
                            Some(r) if (r.maximo - r.minimo).abs() > 1e-9 => {
                                format!("{} – {}", fmt(r.minimo, dec()), fmt(r.maximo, dec()))
                            }
                            _ => String::new(),
                        })
                    }}
                </span>
                <span class="peso">
                    <select aria-label=move || format!("Tipo de peso de {}", nombre.get()) on:change=move |ev| {
                        let m = event_target_value(&ev);
                        let actual = peso_valor.get_untracked();
                        let sugerido = res.with_untracked(|r| r.as_ref().map(|r| r.peso));
                        app.editar_nodo(id, |n| {
                            n.peso = match m.as_str() {
                                "grupo" => Peso::Grupo(actual.or(sugerido.map(|p| p.local * 100.0)).unwrap_or(0.0)),
                                "final" => Peso::Final(actual.or(sugerido.map(|p| p.final_ * 100.0)).unwrap_or(0.0)),
                                _ => Peso::Auto,
                            }
                        })
                    }>
                        <option value="auto" prop:selected=move || modo() == "auto">"auto"</option>
                        <option value="grupo" prop:selected=move || modo() == "grupo">"% grupo"</option>
                        <option value="final" prop:selected=move || modo() == "final">"% final"</option>
                    </select>
                    <Numero
                        valor=peso_valor
                        placeholder=peso_sugerido
                        clase="peso-num"
                        etiqueta=rotulo("Peso (%) de")
                        al_cambiar=Callback::new(move |x: Option<f64>| {
                            app.editar_nodo(id, |n| {
                                n.peso = match (&n.peso, x) {
                                    (_, None) => Peso::Auto,
                                    (Peso::Final(_), Some(p)) => Peso::Final(p),
                                    (_, Some(p)) => Peso::Grupo(p),
                                }
                            })
                        })
                    />
                </span>
                <span class="pct-final">
                    <span>{move || res.with(|r| r.as_ref().map(|r| pct(r.peso.final_)).unwrap_or_default())}</span>
                    <span class="medida" aria-hidden="true">
                        <i style:width=move || {
                            format!("{:.1}%", res.with(|r| r.as_ref().map_or(0.0, |r| r.peso.final_)).clamp(0.0, 1.0) * 100.0)
                        }></i>
                    </span>
                </span>
                <span class="reglas">
                    {move || {
                        candado()
                            .map(|c| {
                                let (clase, titulo) = match res.with(|r| r.as_ref().and_then(|r| r.candado)) {
                                    Some(EstadoCandado::Roto) => ("chip mal", "Candado roto"),
                                    Some(EstadoCandado::Cumple) => ("chip ok", "Candado cumplido"),
                                    _ => ("chip", "Candado pendiente"),
                                };
                                let alcance = match c.alcance {
                                    Alcance::Final => "la final",
                                    Alcance::Grupo => "el grupo",
                                };
                                let detalle = format!("{titulo}: si queda bajo {}, {alcance} se limita a {}", corto(c.minimo), corto(c.tope));
                                view! {
                                    <span class=clase title=detalle.clone() aria-label=detalle>
                                        <Icono nombre="candado" tam=12 />
                                        {format!("≥{}", corto(c.minimo))}
                                    </span>
                                }
                            })
                    }}
                    {move || {
                        let n = eliminar_menores();
                        (n > 0).then(|| view! { <span class="chip" title="Se eliminan las notas más bajas">{format!("−{n} menor")}</span> })
                    }}
                    {move || {
                        res.with(|r| r.as_ref().is_some_and(|r| r.eliminado))
                            .then(|| view! { <span class="chip tenue">"eliminada"</span> })
                    }}
                </span>
                <span class="acciones">
                    <button
                        title="Reglas: candado y eliminar menores"
                        aria-label=rotulo("Reglas de")
                        aria-expanded=move || if abierto.get() { "true" } else { "false" }
                        class:activo=move || abierto.get()
                        on:click=move |_| abierto.update(|a| *a = !*a)
                    >
                        <Icono nombre="ajustes" tam=15 />
                    </button>
                    <button title="Subir" aria-label=rotulo("Subir") on:click=move |_| app.editar(|c| { c.mover(id, -1); })>
                        <Icono nombre="subir" tam=15 />
                    </button>
                    <button title="Bajar" aria-label=rotulo("Bajar") on:click=move |_| app.editar(|c| { c.mover(id, 1); })>
                        <Icono nombre="bajar" tam=15 />
                    </button>
                    <button
                        title="Eliminar"
                        aria-label=rotulo("Eliminar")
                        class="peligro"
                        on:click=move |_| {
                            if grupo {
                                app.confirmar(
                                    "¿Eliminar grupo?",
                                    format!("Se eliminará el grupo «{}» con todo su contenido.", nombre.get_untracked()),
                                    "Eliminar",
                                    move || app.editar(|c| { c.eliminar(id); }),
                                );
                            } else {
                                app.editar(|c| { c.eliminar(id); });
                            }
                        }
                    >
                        <Icono nombre="cerrar" tam=15 />
                    </button>
                </span>
            </div>
            <Show when=move || abierto.get()>
                <div class="opciones" style=format!("margin-left: {}px", 12 + nivel * 22)>
                    <label class="check">
                        <input
                            type="checkbox"
                            prop:checked=move || candado().is_some()
                            on:change=move |ev| {
                                let on = event_target_checked(&ev);
                                app.editar(|c| {
                                    let apr = c.aprobatoria;
                                    if let Some(n) = c.buscar_mut(id) {
                                        n.candado = on.then(|| Candado {
                                            minimo: apr,
                                            tope: (apr - 0.5).max(0.0),
                                            alcance: Alcance::Final,
                                        });
                                    }
                                })
                            }
                        />
                        "Nota mínima (candado)"
                    </label>
                    {move || {
                        candado()
                            .map(|c| {
                                let set = move |f: fn(&mut Candado, f64), x: Option<f64>| {
                                    if let Some(x) = x {
                                        app.editar_nodo(id, |n| {
                                            if let Some(c) = &mut n.candado {
                                                f(c, x)
                                            }
                                        })
                                    }
                                };
                                view! {
                                    <span class="frase">
                                        "Si queda por debajo de"
                                        <Numero
                                            valor=Signal::derive(move || candado().map(|c| c.minimo))
                                            al_cambiar=Callback::new(move |x| set(|c, x| c.minimo = x, x))
                                            etiqueta=Signal::derive(|| "Nota mínima del candado".to_string())
                                        />
                                        ", la nota de"
                                        <select aria-label="Alcance del candado" on:change=move |ev| {
                                            let a = if event_target_value(&ev) == "grupo" { Alcance::Grupo } else { Alcance::Final };
                                            app.editar_nodo(id, |n| {
                                                if let Some(c) = &mut n.candado {
                                                    c.alcance = a
                                                }
                                            })
                                        }>
                                            <option value="final" selected={c.alcance == Alcance::Final}>"todo el curso"</option>
                                            <option value="grupo" selected={c.alcance == Alcance::Grupo}>"su grupo"</option>
                                        </select>
                                        "queda limitada a"
                                        <Numero
                                            valor=Signal::derive(move || candado().map(|c| c.tope))
                                            al_cambiar=Callback::new(move |x| set(|c, x| c.tope = x, x))
                                            etiqueta=Signal::derive(|| "Nota tope si se rompe el candado".to_string())
                                        />
                                    </span>
                                }
                            })
                    }}
                    {grupo
                        .then(|| {
                            view! {
                                <label class="frase">
                                    "Eliminar las"
                                    <Numero
                                        valor=Signal::derive(move || Some(eliminar_menores() as f64))
                                        al_cambiar=Callback::new(move |x: Option<f64>| {
                                            let k = x.unwrap_or(0.0).max(0.0) as usize;
                                            app.editar_nodo(id, |n| {
                                                if let Tipo::Grupo { eliminar_menores, .. } = &mut n.tipo {
                                                    *eliminar_menores = k
                                                }
                                            })
                                        })
                                    />
                                    "notas más bajas de este grupo"
                                </label>
                            }
                        })}
                </div>
            </Show>
            {grupo
                .then(|| {
                    view! {
                        <div class="subarbol" class:oculto=move || plegado.get()>
                            <Hijos padre=id nivel={nivel + 1} />
                            <Agregar padre=id nivel={nivel + 1} />
                        </div>
                    }
                })}
        </div>
    }
}
