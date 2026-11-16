use leptos::prelude::*;
use leptos::task::spawn_local;
use nojalo_core::*;

use crate::api;
use crate::iconos::Icono;

// ───────────────────────── estado global ─────────────────────────

#[derive(Clone, Copy)]
struct Global {
    cursos: RwSignal<Vec<Curso>>,
    sel: RwSignal<Option<String>>,
    error: RwSignal<Option<String>>,
    /// Guardados en curso (para el indicador «Guardando…»).
    guardando: RwSignal<u32>,
    /// Si ya se guardó algo en esta sesión (para mostrar «Guardado»).
    guardado: RwSignal<bool>,
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
        let Global { error, guardando, guardado, .. } = *self;
        guardando.update(|n| *n += 1);
        spawn_local(async move {
            let r = api::guardar(&c).await;
            guardando.update(|n| *n = n.saturating_sub(1));
            match r {
                Ok(()) => guardado.set(true),
                Err(e) => error.set(Some(format!("No se pudo guardar «{}»: {e}. Vuelve a editar un campo para reintentar.", c.nombre))),
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
///
/// Vacío = sin valor. Si el texto no es un número no se guarda nada: el campo
/// se marca como inválido y conserva lo escrito para que se pueda corregir.
#[component]
fn Numero(
    valor: Signal<Option<f64>>,
    al_cambiar: Callback<Option<f64>>,
    #[prop(optional)] placeholder: Option<Signal<String>>,
    #[prop(optional)] clase: &'static str,
    /// Nombre accesible cuando el campo no está dentro de un `<label>`.
    #[prop(optional)]
    etiqueta: Option<Signal<String>>,
) -> impl IntoView {
    let invalido = RwSignal::new(false);
    view! {
        <input
            type="text"
            inputmode="decimal"
            class=move || format!("num {clase}{}", if invalido.get() { " invalido" } else { "" })
            aria-label=move || etiqueta.map(|e| e.get())
            aria-invalid=move || if invalido.get() { "true" } else { "false" }
            title=move || invalido.get().then_some("Escribe un número, por ejemplo 14.5 (o déjalo vacío)")
            placeholder=move || placeholder.map(|p| p.get()).unwrap_or_default()
            prop:value=move || valor.get().map(corto).unwrap_or_default()
            on:change=move |ev| {
                let s = event_target_value(&ev);
                if s.trim().is_empty() {
                    invalido.set(false);
                    al_cambiar.run(None);
                } else if let Some(x) = num(&s) {
                    invalido.set(false);
                    al_cambiar.run(Some(x));
                } else {
                    invalido.set(true);
                }
            }
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
        guardando: RwSignal::new(0),
        guardado: RwSignal::new(false),
    };
    provide_context(app);

    // Los avisos de error se cierran solos a los 8 s (o al hacer clic en ✕).
    Effect::new(move |_| {
        if let Some(e) = app.error.get() {
            set_timeout(
                move || {
                    if app.error.get_untracked().as_deref() == Some(e.as_str()) {
                        app.error.set(None);
                    }
                },
                std::time::Duration::from_secs(8),
            );
        }
    });

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
            <div class="avisos" role="alert" aria-live="assertive">
                {move || {
                    app.error
                        .get()
                        .map(|e| {
                            view! {
                                <div class="toast">
                                    <Icono nombre="alerta" />
                                    <span>{e}</span>
                                    <button class="icono-btn" aria-label="Cerrar aviso" on:click=move |_| app.error.set(None)>
                                        <Icono nombre="cerrar" />
                                    </button>
                                </div>
                            }
                        })
                }}
            </div>
        </div>
    }
}

#[component]
fn Bienvenida() -> impl IntoView {
    let app = expect_context::<Global>();
    view! {
        <div class="bienvenida">
            <span class="logo-grande">
                <Icono nombre="logo" tam=36 />
            </span>
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
                    let items: Vec<(String, String, Option<f64>, Estado, u8, f64)> = cursos
                        .iter()
                        .map(|c| {
                            let i = informe(c);
                            (c.id.clone(), c.nombre.clone(), i.actual, i.estado, c.decimales, c.escala_max)
                        })
                        .collect();
                    (p, pond, items)
                })
                .collect::<Vec<_>>()
        })
    };

    view! {
        <aside class="barra" aria-label="Cursos">
            <div class="marca">
                <span class="logo">
                    <Icono nombre="logo" tam=20 />
                </span>
                "Nojalo"
            </div>
            <div class="fila-botones">
                <button
                    class="primario"
                    on:click=move |_| app.crear(Curso::nuevo(nuevo_id(), "Nuevo curso", &periodo_actual()))
                >
                    <Icono nombre="mas" />
                    "Curso"
                </button>
                <button
                    title="Agrega un curso de ejemplo con todas las reglas"
                    on:click=move |_| app.crear(Curso::ejemplo(nuevo_id(), &periodo_actual()))
                >
                    <Icono nombre="libro" />
                    "Ejemplo"
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
                                    .map(|(id, nombre, nota, estado, dec, escala)| {
                                        let id2 = id.clone();
                                        let (clase, texto_estado) = match estado {
                                            Estado::Aprobado => ("ok", "aprobado"),
                                            Estado::Desaprobado => ("mal", "jalado"),
                                            Estado::EnJuego => ("juego", "en juego"),
                                        };
                                        let activo = move || app.sel.get().as_deref() == Some(id.as_str());
                                        view! {
                                            <button
                                                class="curso-item"
                                                class:activo=activo.clone()
                                                aria-current=move || activo().then_some("page")
                                                title=format!("{nombre} ({texto_estado})")
                                                on:click=move |_| app.sel.set(Some(id2.clone()))
                                            >
                                                {mini_anillo(nota.map_or(0.0, |x| x / escala), clase)}
                                                <span class="sr">{format!("{texto_estado}: ")}</span>
                                                <span class="nombre">{nombre.clone()}</span>
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
                <div class="linea-titulo">
                <input
                    class="titulo"
                    aria-label="Nombre del curso"
                    prop:value=texto(|c| c.nombre.clone())
                    on:change=move |ev| {
                        let v = event_target_value(&ev);
                        app.editar(|c| c.nombre = v)
                    }
                />
                <span class="estado-guardado" role="status" aria-live="polite">
                    {move || {
                        if app.guardando.get() > 0 {
                            view! { <span class="guardando">"Guardando…"</span> }.into_any()
                        } else if app.guardado.get() {
                            view! {
                                <span class="guardado">
                                    <Icono nombre="check" tam=14 />
                                    "Guardado"
                                </span>
                            }
                                .into_any()
                        } else {
                            ().into_any()
                        }
                    }}
                </span>
                </div>
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
                        <Icono nombre="papelera" tam=14 />
                        "Eliminar curso"
                    </button>
                </div>
            </header>
            <Resumen />
            <Advertencias />
            <section class="arbol" aria-label="Evaluaciones del curso">
                <div class="fila encabezado" aria-hidden="true">
                    <span>"Evaluación"</span>
                    <span class="der">"Nota"</span>
                    <span class="col-rango">"Rango posible"</span>
                    <span>"Peso"</span>
                    <span class="der" title="Peso efectivo sobre la nota final">"% final"</span>
                    <span>"Reglas"</span>
                    <span></span>
                </div>
                <Show when=move || curso.with(|c| c.as_ref().is_some_and(|c| c.hijos_ids(ID_RAIZ).is_empty()))>
                    <p class="vacio">
                        "Este curso aún no tiene evaluaciones. Agrega notas sueltas (un examen) "
                        "o grupos (prácticas, laboratorios) y asígnales su peso."
                    </p>
                </Show>
                <Hijos padre=ID_RAIZ nivel=0 />
                <Agregar padre=ID_RAIZ nivel=0 />
            </section>
        </div>
    }
}

/// Fracción del arco del medidor (240° de los 360° de la circunferencia).
const ARCO: f64 = 240.0 / 360.0 * 100.0;

/// Punto sobre el medidor: el arco arranca abajo a la izquierda (150°) y gira 240°.
fn punto_arco(frac: f64, r: f64) -> (f64, f64) {
    let a = (150.0 + 240.0 * frac.clamp(0.0, 1.0)).to_radians();
    (60.0 + r * a.cos(), 60.0 + r * a.sin())
}

/// Anillo pequeño para la barra lateral.
fn mini_anillo(frac: f64, clase: &'static str) -> impl IntoView {
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
fn Advertencias() -> impl IntoView {
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
fn Fila(id: Id, nivel: usize) -> impl IntoView {
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
                            let nombre = nombre.get_untracked();
                            if !grupo || confirmar(&format!("¿Eliminar el grupo «{nombre}» con todo su contenido?")) {
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
