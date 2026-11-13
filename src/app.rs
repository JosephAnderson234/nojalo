use leptos::prelude::*;
use leptos::task::spawn_local;
use nojalo_core::*;

use crate::api;

// ───────────────────────── estado global ─────────────────────────

#[derive(Clone, Copy)]
struct Global {
    cursos: RwSignal<Vec<Curso>>,
    sel: RwSignal<Option<String>>,
    error: RwSignal<Option<String>>,
}

impl Global {
    /// Aplica un cambio al curso seleccionado y lo persiste.
    fn editar(&self, f: impl FnOnce(&mut Curso)) {
        let Some(id) = self.sel.get_untracked() else { return };
        let mut copia = None;
        self.cursos.update(|v| {
            if let Some(c) = v.iter_mut().find(|c| c.id == id) {
                f(c);
                copia = Some(c.clone());
            }
        });
        if let Some(c) = copia {
            self.guardar(c);
        }
    }

    fn editar_nodo(&self, id: Id, f: impl FnOnce(&mut Nodo)) {
        self.editar(|c| {
            if let Some(n) = c.buscar_mut(id) {
                f(n)
            }
        })
    }

    fn guardar(&self, c: Curso) {
        let error = self.error;
        spawn_local(async move {
            if let Err(e) = api::guardar(&c).await {
                error.set(Some(format!("No se pudo guardar: {e}")));
            }
        });
    }

    fn crear(&self, c: Curso) {
        self.sel.set(Some(c.id.clone()));
        self.cursos.update(|v| v.push(c.clone()));
        self.guardar(c);
    }
}

/// Curso abierto en el editor y su informe calculado.
#[derive(Clone, Copy)]
struct Vista {
    curso: Memo<Option<Curso>>,
    informe: Memo<Option<Informe>>,
}

// ───────────────────────── utilidades ─────────────────────────

fn num(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', ".");
    if t.is_empty() {
        None
    } else {
        t.parse().ok().filter(|x: &f64| x.is_finite())
    }
}

fn corto(x: f64) -> String {
    let r = (x * 100.0).round() / 100.0;
    format!("{r}")
}

fn fmt(x: f64, dec: u8) -> String {
    format!("{:.*}", dec as usize, x)
}

fn pct(x: f64) -> String {
    let p = x * 100.0;
    if (p - p.round()).abs() < 0.05 {
        format!("{p:.0}%")
    } else {
        format!("{p:.1}%")
    }
}

fn nuevo_id() -> String {
    format!("c{:x}{:04x}", js_sys::Date::now() as u64, (js_sys::Math::random() * 65535.0) as u32)
}

fn periodo_actual() -> String {
    let d = js_sys::Date::new_0();
    format!("{}-{}", d.get_full_year(), if d.get_month() < 6 { 1 } else { 2 })
}

fn confirmar(msg: &str) -> bool {
    web_sys::window().and_then(|w| w.confirm_with_message(msg).ok()).unwrap_or(true)
}

/// Campo numérico que confirma al salir del input o con Enter.
#[component]
fn Numero(
    valor: Signal<Option<f64>>,
    al_cambiar: Callback<Option<f64>>,
    #[prop(optional)] placeholder: Option<Signal<String>>,
    #[prop(optional)] clase: &'static str,
) -> impl IntoView {
    view! {
        <input
            type="text"
            inputmode="decimal"
            class=format!("num {clase}")
            placeholder=move || placeholder.map(|p| p.get()).unwrap_or_default()
            prop:value=move || valor.get().map(corto).unwrap_or_default()
            on:change=move |ev| al_cambiar.run(num(&event_target_value(&ev)))
        />
    }
}

// ───────────────────────── raíz ─────────────────────────

#[component]
pub fn App() -> impl IntoView {
    let app = Global {
        cursos: RwSignal::new(vec![]),
        sel: RwSignal::new(None),
        error: RwSignal::new(None),
    };
    provide_context(app);

    spawn_local(async move {
        match api::listar().await {
            Ok(v) => {
                app.sel.set(v.first().map(|c| c.id.clone()));
                app.cursos.set(v);
            }
            Err(e) => app.error.set(Some(format!("No se pudieron cargar los cursos: {e}"))),
        }
    });

    view! {
        <div class="app">
            <Barra />
            <main class="principal">
                {move || match app.sel.get() {
                    Some(_) => view! { <Editor /> }.into_any(),
                    None => view! { <Bienvenida /> }.into_any(),
                }}
            </main>
            {move || {
                app.error
                    .get()
                    .map(|e| {
                        view! {
                            <div class="toast" on:click=move |_| app.error.set(None)>
                                {e}
                            </div>
                        }
                    })
            }}
        </div>
    }
}

#[component]
fn Bienvenida() -> impl IntoView {
    let app = expect_context::<Global>();
    view! {
        <div class="bienvenida">
            <h1>"Nojalo"</h1>
            <p>
                "Registra tus notas como un árbol: grupos dentro de grupos, pesos relativos al grupo o a la nota final, "
                "candados de nota mínima y eliminación de las notas más bajas."
            </p>
            <div class="fila-botones">
                <button
                    class="primario"
                    on:click=move |_| app.crear(Curso::nuevo(nuevo_id(), "Nuevo curso", &periodo_actual()))
                >
                    "Crear curso"
                </button>
                <button on:click=move |_| app.crear(Curso::ejemplo(nuevo_id(), &periodo_actual()))>
                    "Ver un ejemplo"
                </button>
            </div>
        </div>
    }
}

// ───────────────────────── barra lateral ─────────────────────────

#[component]
fn Barra() -> impl IntoView {
    let app = expect_context::<Global>();

    let periodos = move || {
        app.cursos.with(|v| {
            let mut ps: Vec<String> = v.iter().map(|c| c.periodo.clone()).collect();
            ps.sort_by(|a, b| b.cmp(a));
            ps.dedup();
            ps.into_iter()
                .map(|p| {
                    let cursos: Vec<&Curso> = v.iter().filter(|c| c.periodo == p).collect();
                    let pond = ponderado(cursos.iter().copied());
                    let items: Vec<(String, String, Option<f64>, Estado, u8)> = cursos
                        .iter()
                        .map(|c| {
                            let i = informe(c);
                            (c.id.clone(), c.nombre.clone(), i.actual, i.estado, c.decimales)
                        })
                        .collect();
                    (p, pond, items)
                })
                .collect::<Vec<_>>()
        })
    };

    view! {
        <aside class="barra">
            <div class="marca">
                <span class="logo">"▦"</span>
                "Nojalo"
            </div>
            <div class="fila-botones">
                <button
                    class="primario"
                    on:click=move |_| app.crear(Curso::nuevo(nuevo_id(), "Nuevo curso", &periodo_actual()))
                >
                    "+ Curso"
                </button>
                <button
                    title="Curso de ejemplo"
                    on:click=move |_| app.crear(Curso::ejemplo(nuevo_id(), &periodo_actual()))
                >
                    "+ Ejemplo"
                </button>
            </div>
            {move || {
                periodos()
                    .into_iter()
                    .map(|(p, pond, items)| {
                        view! {
                            <section class="periodo">
                                <header>
                                    <span>{if p.is_empty() { "Sin periodo".to_string() } else { p }}</span>
                                    <span class="pond" title="Promedio ponderado por créditos">
                                        {pond.map(|x| fmt(x, 2)).unwrap_or_else(|| "—".into())}
                                    </span>
                                </header>
                                {items
                                    .into_iter()
                                    .map(|(id, nombre, nota, estado, dec)| {
                                        let id2 = id.clone();
                                        let clase = match estado {
                                            Estado::Aprobado => "punto ok",
                                            Estado::Desaprobado => "punto mal",
                                            Estado::EnJuego => "punto juego",
                                        };
                                        view! {
                                            <button
                                                class="curso-item"
                                                class:activo=move || app.sel.get().as_deref() == Some(id.as_str())
                                                on:click=move |_| app.sel.set(Some(id2.clone()))
                                            >
                                                <span class=clase></span>
                                                <span class="nombre">{nombre}</span>
                                                <span class="nota">
                                                    {nota.map(|x| fmt(x, dec)).unwrap_or_else(|| "—".into())}
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
        </aside>
    }
}

// ───────────────────────── editor de curso ─────────────────────────

#[component]
fn Editor() -> impl IntoView {
    let app = expect_context::<Global>();
    let curso = Memo::new(move |_| {
        let sel = app.sel.get();
        app.cursos.with(|v| v.iter().find(|c| Some(&c.id) == sel.as_ref()).cloned())
    });
    let informe = Memo::new(move |_| curso.with(|c| c.as_ref().map(informe)));
    provide_context(Vista { curso, informe });

    let texto = move |f: fn(&Curso) -> String| move || curso.with(|c| c.as_ref().map(f).unwrap_or_default());
    let numero = move |f: fn(&Curso) -> f64| Signal::derive(move || curso.with(|c| c.as_ref().map(f)));

    let borrar = move |_| {
        let Some(c) = curso.get_untracked() else { return };
        if !confirmar(&format!("¿Eliminar «{}» y todas sus notas?", c.nombre)) {
            return;
        }
        app.cursos.update(|v| v.retain(|x| x.id != c.id));
        app.sel.set(app.cursos.with_untracked(|v| v.first().map(|c| c.id.clone())));
        let error = app.error;
        spawn_local(async move {
            if let Err(e) = api::eliminar(&c.id).await {
                error.set(Some(e));
            }
        });
    };

    view! {
        <div class="editor">
            <header class="cabecera">
                <input
                    class="titulo"
                    prop:value=texto(|c| c.nombre.clone())
                    on:change=move |ev| {
                        let v = event_target_value(&ev);
                        app.editar(|c| c.nombre = v)
                    }
                />
                <div class="meta">
                    <label>
                        "Periodo"
                        <input
                            class="corto"
                            prop:value=texto(|c| c.periodo.clone())
                            on:change=move |ev| {
                                let v = event_target_value(&ev);
                                app.editar(|c| c.periodo = v.trim().to_string())
                            }
                        />
                    </label>
                    <label>
                        "Créditos"
                        <Numero
                            valor=numero(|c| c.creditos)
                            al_cambiar=Callback::new(move |v: Option<f64>| app.editar(|c| c.creditos = v.unwrap_or(0.0).max(0.0)))
                        />
                    </label>
                    <label>
                        "Escala 0 –"
                        <Numero
                            valor=numero(|c| c.escala_max)
                            al_cambiar=Callback::new(move |v: Option<f64>| {
                                if let Some(v) = v.filter(|v| *v > 0.0) {
                                    app.editar(|c| c.escala_max = v)
                                }
                            })
                        />
                    </label>
                    <label>
                        "Aprueba con"
                        <Numero
                            valor=numero(|c| c.aprobatoria)
                            al_cambiar=Callback::new(move |v: Option<f64>| {
                                if let Some(v) = v {
                                    app.editar(|c| c.aprobatoria = v)
                                }
                            })
                        />
                    </label>
                    <label>
                        "Decimales"
                        <Numero
                            valor=numero(|c| c.decimales as f64)
                            al_cambiar=Callback::new(move |v: Option<f64>| {
                                app.editar(|c| c.decimales = v.unwrap_or(2.0).clamp(0.0, 4.0) as u8)
                            })
                        />
                    </label>
                    <button class="peligro" on:click=borrar>
                        "Eliminar curso"
                    </button>
                </div>
            </header>
            <Resumen />
            <Advertencias />
            <section class="arbol">
                <div class="fila encabezado">
                    <span>"Evaluación"</span>
                    <span>"Nota"</span>
                    <span>"Rango posible"</span>
                    <span>"Peso"</span>
                    <span title="Peso efectivo sobre la nota final">"% final"</span>
                    <span>"Reglas"</span>
                    <span></span>
                </div>
                <Hijos padre=ID_RAIZ nivel=0 />
                <Agregar padre=ID_RAIZ nivel=0 />
            </section>
        </div>
    }
}

#[component]
fn Resumen() -> impl IntoView {
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
    // Posición (0..100%) en la barra de escala.
    let pos = move |f: fn(&Informe) -> f64| {
        move || {
            let max = v.curso.with(|c| c.as_ref().map_or(20.0, |c| c.escala_max));
            let x = v.informe.with(|i| i.as_ref().map_or(0.0, f));
            format!("{:.2}%", (x / max * 100.0).clamp(0.0, 100.0))
        }
    };
    let estado = move || v.informe.with(|i| i.as_ref().map(|i| i.estado));
    let max = move || v.curso.with(|c| c.as_ref().map_or(20.0, |c| c.escala_max));
    let sim_x = move || simulado.get().unwrap_or_else(|| v.informe.with(|i| i.as_ref().and_then(|i| i.necesaria)).unwrap_or(max() / 2.0));
    let sim_final = move || v.curso.with(|c| c.as_ref().and_then(|c| simular(c, sim_x()).map(|n| fmt(n, c.decimales))));

    view! {
        <section class="resumen">
            <div class="tarjetas">
                <div class="tarjeta">
                    <span class="rotulo">"Nota actual"</span>
                    <span class="grande">
                        {dato(|c, i| i.actual.map(|x| fmt(x, c.decimales)).unwrap_or_else(|| "—".into()))}
                    </span>
                    <span class="sub">"con lo rendido hasta ahora"</span>
                </div>
                <div class="tarjeta">
                    <span class="rotulo">"Rango posible"</span>
                    <span class="grande">
                        {dato(|c, i| format!("{} – {}", fmt(i.minimo, c.decimales), fmt(i.maximo, c.decimales)))}
                    </span>
                    <span class="sub">"sacando 0 o el máximo en lo pendiente"</span>
                </div>
                <div
                    class="tarjeta"
                    class:ok=move || estado() == Some(Estado::Aprobado)
                    class:mal=move || estado() == Some(Estado::Desaprobado)
                >
                    <span class="rotulo">"Para aprobar"</span>
                    <span class="grande">
                        {dato(|c, i| match i.estado {
                            Estado::Aprobado => "Asegurado".into(),
                            Estado::Desaprobado => "No alcanza".into(),
                            Estado::EnJuego => {
                                i.necesaria.map(|x| fmt(x, c.decimales)).unwrap_or_default()
                            }
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
                <div class="tarjeta">
                    <span class="rotulo">"Avance"</span>
                    <span class="grande">{dato(|_, i| pct(i.avance))}</span>
                    <span class="sub">"del peso del curso ya calificado"</span>
                </div>
            </div>
            <div class="escala" title="Rango posible, nota actual y nota aprobatoria">
                <div class="rango" style:left=pos(|i| i.minimo) style:right=move || {
                    let max = v.curso.with(|c| c.as_ref().map_or(20.0, |c| c.escala_max));
                    let x = v.informe.with(|i| i.as_ref().map_or(0.0, |i| i.maximo));
                    format!("{:.2}%", (100.0 - x / max * 100.0).clamp(0.0, 100.0))
                }></div>
                <div class="linea-apr" style:left=move || {
                    v.curso.with(|c| c.as_ref().map_or("0%".into(), |c| format!("{:.2}%", (c.aprobatoria / c.escala_max * 100.0).clamp(0.0, 100.0))))
                }></div>
                <Show when=move || v.informe.with(|i| i.as_ref().is_some_and(|i| i.actual.is_some()))>
                    <div class="marca-actual" style:left=pos(|i| i.actual.unwrap_or(0.0))></div>
                </Show>
            </div>
            <Show when=move || estado() == Some(Estado::EnJuego)>
                <div class="simulador">
                    <span>"Si saco"</span>
                    <input
                        type="range"
                        min="0"
                        max=move || max().to_string()
                        step="0.5"
                        prop:value=move || sim_x().to_string()
                        on:input=move |ev| simulado.set(num(&event_target_value(&ev)))
                    />
                    <strong>{move || corto(sim_x())}</strong>
                    <span>"en todo lo pendiente, termino con"</span>
                    <strong>{sim_final}</strong>
                </div>
            </Show>
        </section>
    }
}

#[component]
fn Advertencias() -> impl IntoView {
    let v = expect_context::<Vista>();
    let lista = move || v.informe.with(|i| i.as_ref().map(|i| i.advertencias.clone()).unwrap_or_default());
    view! {
        <Show when=move || !lista().is_empty()>
            <ul class="advertencias">
                {move || lista().into_iter().map(|a| view! { <li>{a}</li> }).collect_view()}
            </ul>
        </Show>
    }
}

// ───────────────────────── árbol ─────────────────────────

#[component]
fn Hijos(padre: Id, nivel: usize) -> AnyView {
    let v = expect_context::<Vista>();
    let ids = move || v.curso.with(|c| c.as_ref().map(|c| c.hijos_ids(padre)).unwrap_or_default());
    view! { <For each=ids key=|id| *id children=move |id| view! { <Fila id=id nivel=nivel /> } /> }
        .into_any()
}

#[component]
fn Agregar(padre: Id, nivel: usize) -> impl IntoView {
    let app = expect_context::<Global>();
    view! {
        <div class="agregar" style=format!("padding-left: {}px", 12 + nivel * 22)>
            <button on:click=move |_| app.editar(|c| { c.agregar(padre, false); })>"+ nota"</button>
            <button on:click=move |_| app.editar(|c| { c.agregar(padre, true); })>"+ grupo"</button>
        </div>
    }
}

#[component]
fn Fila(id: Id, nivel: usize) -> impl IntoView {
    let app = expect_context::<Global>();
    let v = expect_context::<Vista>();
    let nodo = Memo::new(move |_| v.curso.with(|c| c.as_ref().and_then(|c| c.buscar(id).cloned())));
    let res = Memo::new(move |_| v.informe.with(|i| i.as_ref().and_then(|i| i.nodos.get(&id).cloned())));
    let dec = move || v.curso.with(|c| c.as_ref().map_or(2, |c| c.decimales));
    let grupo = nodo.with_untracked(|n| n.as_ref().is_some_and(|n| n.es_grupo()));
    let abierto = RwSignal::new(false);
    let plegado = RwSignal::new(false);

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

    let celda_nota = if grupo {
        view! {
            <span class="nota-grupo">
                {move || res.with(|r| r.as_ref().and_then(|r| r.actual).map(|x| fmt(x, dec())).unwrap_or_else(|| "—".into()))}
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
            />
        }
        .into_any()
    };

    view! {
        <div class="nodo" class:grupo=grupo>
            <div class="fila" class:eliminado=move || res.with(|r| r.as_ref().is_some_and(|r| r.eliminado))>
                <span class="celda-nombre" style=format!("padding-left: {}px", nivel * 22)>
                    {if grupo {
                        view! {
                            <button class="plegar" on:click=move |_| plegado.update(|p| *p = !*p)>
                                {move || if plegado.get() { "▸" } else { "▾" }}
                            </button>
                        }
                        .into_any()
                    } else {
                        view! { <span class="vineta">"·"</span> }.into_any()
                    }}
                    <input
                        class="nombre"
                        prop:value=move || nodo.with(|n| n.as_ref().map(|n| n.nombre.clone()).unwrap_or_default())
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
                    <select on:change=move |ev| {
                        let m = event_target_value(&ev);
                        let actual = peso_valor.get_untracked();
                        let sugerido = res.with_untracked(|r| r.as_ref().map(|r| r.peso.clone()));
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
                <span class="pct-final">{move || res.with(|r| r.as_ref().map(|r| pct(r.peso.final_)).unwrap_or_default())}</span>
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
                                view! {
                                    <span
                                        class=clase
                                        title=format!("{titulo}: si queda bajo {}, {alcance} se limita a {}", corto(c.minimo), corto(c.tope))
                                    >
                                        {format!("🔒 ≥{}", corto(c.minimo))}
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
                    <button title="Reglas" class:activo=move || abierto.get() on:click=move |_| abierto.update(|a| *a = !*a)>"⚙"</button>
                    <button title="Subir" on:click=move |_| app.editar(|c| { c.mover(id, -1); })>"↑"</button>
                    <button title="Bajar" on:click=move |_| app.editar(|c| { c.mover(id, 1); })>"↓"</button>
                    <button
                        title="Eliminar"
                        class="peligro"
                        on:click=move |_| {
                            let nombre = nodo.with_untracked(|n| n.as_ref().map(|n| n.nombre.clone()).unwrap_or_default());
                            if !grupo || confirmar(&format!("¿Eliminar el grupo «{nombre}» con todo su contenido?")) {
                                app.editar(|c| { c.eliminar(id); });
                            }
                        }
                    >
                        "✕"
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
                                        />
                                        ", la nota de"
                                        <select on:change=move |ev| {
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
