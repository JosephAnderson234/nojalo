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
    /// Ciclo abierto en la vista de ciclo; si hay uno, tiene prioridad sobre el curso.
    ciclo: RwSignal<Option<String>>,
    error: RwSignal<Option<String>>,
    /// Aviso de que algo salió bien (se cierra solo).
    aviso: RwSignal<Option<String>>,
    /// Diálogo de confirmación abierto.
    confirmacion: RwSignal<Option<Confirmacion>>,
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

    /// Aplica un cambio a todos los cursos que cumplen `pred` y los persiste.
    fn editar_donde(&self, pred: impl Fn(&Curso) -> bool, f: impl Fn(&mut Curso)) {
        let mut cambiados = vec![];
        self.cursos.update(|v| {
            for c in v.iter_mut().filter(|c| pred(c)) {
                f(c);
                cambiados.push(c.clone());
            }
        });
        for c in cambiados {
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
        self.ciclo.set(None);
        self.sel.set(Some(c.id.clone()));
        self.cursos.update(|v| v.push(c.clone()));
        self.guardar(c);
    }

    /// Muestra el diálogo de confirmación; `accion` corre solo si se acepta.
    fn confirmar(
        &self,
        titulo: &str,
        mensaje: String,
        boton: &'static str,
        accion: impl Fn() + Send + Sync + 'static,
    ) {
        self.confirmacion.set(Some(Confirmacion {
            titulo: titulo.into(),
            mensaje,
            boton,
            accion: Callback::new(move |()| accion()),
        }));
    }

    fn eliminar_curso(&self, id: String) {
        self.cursos.update(|v| v.retain(|x| x.id != id));
        if self.sel.get_untracked().as_deref() == Some(id.as_str()) {
            self.sel.set(self.cursos.with_untracked(|v| v.first().map(|c| c.id.clone())));
        }
        let Global { error, aviso, .. } = *self;
        spawn_local(async move {
            match api::eliminar(&id).await {
                Ok(()) => aviso.set(Some("Curso eliminado.".into())),
                Err(e) => error.set(Some(format!("No se pudo eliminar el curso: {e}"))),
            }
        });
    }

    /// Mueve todos los cursos de `viejo` a `nuevo` y abre el ciclo resultante.
    fn renombrar_ciclo(&self, viejo: String, nuevo: String) {
        self.editar_donde(|c| c.periodo == viejo, |c| c.periodo = nuevo.clone());
        self.abrir_ciclo(nuevo);
    }

    /// Abre un ciclo vacío con el nombre que sigue al más reciente.
    fn nuevo_ciclo(&self) {
        let existentes = self.cursos.with_untracked(|v| ciclos(v));
        self.abrir_ciclo(ciclo_siguiente(&existentes, &periodo_actual()));
    }

    fn abrir_curso(&self, id: String) {
        self.ciclo.set(None);
        self.sel.set(Some(id));
    }

    fn abrir_ciclo(&self, ciclo: String) {
        self.ciclo.set(Some(ciclo));
    }

    /// Ciclo donde caen los cursos nuevos: el ciclo abierto, el del curso
    /// abierto o, si no hay ninguno, el semestre actual.
    fn ciclo_destino(&self) -> String {
        if let Some(c) = self.ciclo.get_untracked() {
            return c;
        }
        let sel = self.sel.get_untracked();
        self.cursos
            .with_untracked(|v| v.iter().find(|c| Some(&c.id) == sel.as_ref()).map(|c| c.periodo.clone()))
            .unwrap_or_else(periodo_actual)
    }

    /// Crea cursos (sin notas) a partir de los esquemas de un archivo.
    fn importar(&self, texto: &str, ciclo: String) {
        let esquemas = match leer_esquemas(texto) {
            Ok(e) => e,
            Err(e) => return self.error.set(Some(format!("No se pudo importar: {e}"))),
        };
        let base = nuevo_id();
        let nuevos: Vec<Curso> = esquemas
            .into_iter()
            .enumerate()
            .map(|(i, e)| Curso::desde_esquema(format!("{base}{i}"), &ciclo, e))
            .collect();
        let n = nuevos.len();
        let primero = nuevos[0].id.clone();
        self.cursos.update(|v| v.extend(nuevos.iter().cloned()));
        for c in nuevos {
            self.guardar(c);
        }
        let donde = if ciclo.is_empty() { "sin ciclo".to_string() } else { format!("en {ciclo}") };
        if n == 1 {
            self.abrir_curso(primero);
            self.aviso.set(Some(format!("Esquema importado {donde}. Ya puedes registrar tus notas.")));
        } else {
            self.sel.set(Some(primero));
            self.abrir_ciclo(ciclo);
            self.aviso.set(Some(format!("{n} esquemas importados {donde}.")));
        }
    }

    /// Guarda un archivo (diálogo nativo en escritorio, descarga en el navegador).
    fn exportar(&self, nombre: String, contenido: String) {
        let Global { error, aviso, .. } = *self;
        spawn_local(async move {
            match api::exportar(&nombre, &contenido).await {
                Ok(true) => aviso.set(Some("Esquema exportado.".into())),
                Ok(false) => {}
                Err(e) => error.set(Some(format!("No se pudo exportar: {e}"))),
            }
        });
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

/// Pregunta pendiente del diálogo de confirmación de la app.
///
/// No se usa `window.confirm`: en Tauri el plugin de diálogos lo reemplaza por
/// una versión asíncrona que, llamada de forma síncrona, siempre devuelve `false`.
#[derive(Clone)]
struct Confirmacion {
    titulo: String,
    mensaje: String,
    boton: &'static str,
    accion: Callback<()>,
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
        ciclo: RwSignal::new(None),
        error: RwSignal::new(None),
        aviso: RwSignal::new(None),
        confirmacion: RwSignal::new(None),
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

    Effect::new(move |_| {
        if let Some(a) = app.aviso.get() {
            set_timeout(
                move || {
                    if app.aviso.get_untracked().as_deref() == Some(a.as_str()) {
                        app.aviso.set(None);
                    }
                },
                std::time::Duration::from_secs(4),
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
                {move || match (app.ciclo.get(), app.sel.get()) {
                    (Some(c), _) => view! { <VistaCiclo ciclo=c /> }.into_any(),
                    (None, Some(_)) => view! { <Editor /> }.into_any(),
                    (None, None) => view! { <Bienvenida /> }.into_any(),
                }}
            </main>
            <Dialogo />
            <div class="pila-avisos">
            <div class="avisos" role="status" aria-live="polite">
                {move || {
                    app.aviso
                        .get()
                        .map(|a| {
                            view! {
                                <div class="toast info">
                                    <Icono nombre="check" />
                                    <span>{a}</span>
                                    <button class="icono-btn" aria-label="Cerrar aviso" on:click=move |_| app.aviso.set(None)>
                                        <Icono nombre="cerrar" />
                                    </button>
                                </div>
                            }
                        })
                }}
            </div>
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
                <BotonImportar
                    titulo="Crear cursos a partir de esquemas de calificación (.json)"
                    al_leer=Callback::new(move |t: String| app.importar(&t, periodo_actual()))
                >
                    <Icono nombre="importar" />
                    "Importar esquema"
                </BotonImportar>
            </div>
        </div>
    }
}

// ───────────────────────── diálogo de confirmación ─────────────────────────

#[component]
fn Dialogo() -> impl IntoView {
    let app = expect_context::<Global>();
    let cancelar_ref = NodeRef::<leptos::html::Button>::new();
    // Al abrirse, el foco va a «Cancelar» (la opción segura).
    Effect::new(move |_| {
        if let Some(b) = cancelar_ref.get() {
            let _ = b.focus();
        }
    });
    let cerrar = move || app.confirmacion.set(None);
    let aceptar = move |_| {
        if let Some(c) = app.confirmacion.get_untracked() {
            app.confirmacion.set(None);
            c.accion.run(());
        }
    };
    let dato = move |f: fn(&Confirmacion) -> String| move || app.confirmacion.with(|c| c.as_ref().map(f).unwrap_or_default());

    view! {
        <Show when=move || app.confirmacion.with(|c| c.is_some())>
            <div class="velo" on:click=move |_| cerrar()>
                <div
                    class="dialogo"
                    role="alertdialog"
                    aria-modal="true"
                    aria-labelledby="dialogo-titulo"
                    aria-describedby="dialogo-texto"
                    on:click=|ev| ev.stop_propagation()
                    on:keydown=move |ev| {
                        if ev.key() == "Escape" {
                            cerrar()
                        }
                    }
                >
                    <span class="dialogo-icono">
                        <Icono nombre="alerta" tam=22 />
                    </span>
                    <h2 id="dialogo-titulo">{dato(|c| c.titulo.clone())}</h2>
                    <p id="dialogo-texto">{dato(|c| c.mensaje.clone())}</p>
                    <div class="dialogo-botones">
                        <button node_ref=cancelar_ref on:click=move |_| cerrar()>
                            "Cancelar"
                        </button>
                        <button class="peligro-fuerte" on:click=aceptar>
                            {dato(|c| c.boton.to_string())}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}

// ───────────────────────── barra lateral ─────────────────────────

fn nombre_ciclo(p: &str) -> String {
    if p.is_empty() {
        "Sin ciclo".into()
    } else {
        p.into()
    }
}

/// Botón que abre el selector de archivos y entrega el texto del `.json` elegido.
#[component]
fn BotonImportar(
    al_leer: Callback<String>,
    #[prop(optional)] clase: &'static str,
    #[prop(optional)] titulo: Option<&'static str>,
    /// Nombre accesible para botones que solo muestran un icono.
    #[prop(optional)]
    etiqueta: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    let app = expect_context::<Global>();
    let entrada = NodeRef::<leptos::html::Input>::new();
    view! {
        <button
            class=clase
            title=titulo
            aria-label=etiqueta
            on:click=move |_| {
                if let Some(i) = entrada.get() {
                    i.click()
                }
            }
        >
            {children()}
        </button>
        <input
            type="file"
            accept=".json,application/json"
            class="oculto"
            tabindex="-1"
            aria-hidden="true"
            node_ref=entrada
            on:change=move |ev| {
                let input: web_sys::HtmlInputElement = event_target(&ev);
                let archivo = input.files().and_then(|f| f.get(0));
                input.set_value("");
                if let Some(a) = archivo {
                    spawn_local(async move {
                        match api::leer_archivo(a).await {
                            Ok(t) => al_leer.run(t),
                            Err(e) => app.error.set(Some(format!("No se pudo leer el archivo: {e}"))),
                        }
                    });
                }
            }
        />
    }
}

#[component]
fn Barra() -> impl IntoView {
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

// ───────────────────────── vista de ciclo ─────────────────────────

#[component]
fn VistaCiclo(ciclo: String) -> impl IntoView {
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
        app.exportar(api::nombre_archivo(&format!("ciclo {}", nombre_ciclo(&c)), ".esquemas.json"), json);
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
fn FilaCiclo(id: String, resumen: Memo<ResumenCiclo>) -> impl IntoView {
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

    let exportar = move |_| {
        if let Some(c) = curso.get_untracked() {
            app.exportar(api::nombre_archivo(&c.nombre, ".esquema.json"), c.esquema().a_json());
        }
    };
    let aplicar = Callback::new(move |t: String| match leer_esquemas(&t) {
        Err(e) => app.error.set(Some(format!("No se pudo importar: {e}"))),
        Ok(v) if v.len() != 1 => app.error.set(Some(format!(
            "El archivo trae {} esquemas. Para aplicarlo a este curso elige uno con un solo esquema, o impórtalo desde la barra lateral para crear un curso por cada uno.",
            v.len()
        ))),
        Ok(mut v) => {
            let e = v.remove(0);
            let pregunta = format!(
                "Las evaluaciones de este curso se reemplazarán por las del esquema «{}». Se borrarán las notas registradas.",
                e.nombre
            );
            app.confirmar("¿Aplicar esquema?", pregunta, "Reemplazar", move || {
                let e = e.clone();
                app.editar(|c| c.aplicar_esquema(e));
                app.aviso.set(Some("Esquema aplicado: créditos, escala, pesos y reglas actualizados.".into()));
            });
        }
    });
    let ciclo_actual = move || curso.with_untracked(|c| c.as_ref().map(|c| c.periodo.clone()).unwrap_or_default());

    let borrar = move |_| {
        let Some(c) = curso.get_untracked() else { return };
        let id = c.id.clone();
        app.confirmar(
            "¿Eliminar curso?",
            format!("Se eliminará «{}» con todas sus notas. No se puede deshacer.", c.nombre),
            "Eliminar",
            move || app.eliminar_curso(id.clone()),
        );
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
                <div class="acciones-curso">
                    <button title="Guardar la estructura y configuración del curso (sin notas) en un archivo" on:click=exportar>
                        <Icono nombre="exportar" tam=14 />
                        "Exportar esquema"
                    </button>
                    <BotonImportar
                        titulo="Reemplazar las evaluaciones de este curso por las de un esquema (.json)"
                        al_leer=aplicar
                    >
                        <Icono nombre="aplicar" tam=14 />
                        "Aplicar esquema"
                    </BotonImportar>
                </div>
                </div>
                <div class="meta">
                    <label>
                        "Ciclo"
                        <input
                            class="corto"
                            list="lista-ciclos"
                            placeholder="2026-2"
                            prop:value=texto(|c| c.periodo.clone())
                            on:change=move |ev| {
                                let v = event_target_value(&ev);
                                app.editar(|c| c.periodo = v.trim().to_string())
                            }
                        />
                    </label>
                    <datalist id="lista-ciclos">
                        {move || {
                            app.cursos
                                .with(|v| ciclos(v))
                                .into_iter()
                                .filter(|c| !c.is_empty())
                                .map(|c| view! { <option value=c></option> })
                                .collect_view()
                        }}
                    </datalist>
                    <button class="sutil" title="Ver el promedio ponderado del ciclo" on:click=move |_| app.abrir_ciclo(ciclo_actual())>
                        <Icono nombre="ciclo" tam=14 />
                        "Ver ciclo"
                    </button>
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
