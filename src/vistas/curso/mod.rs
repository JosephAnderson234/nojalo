//! Editor de un curso: cabecera, resumen y árbol de evaluaciones.

mod arbol;
mod resumen;

use leptos::prelude::*;
use nojalo_core::*;

use crate::componentes::{BotonImportar, Numero};
use crate::estado::Global;
use crate::formato::nombre_archivo;
use crate::iconos::Icono;

use arbol::{Agregar, Hijos};
use resumen::{Advertencias, Resumen};

/// Curso abierto en el editor y su informe calculado.
#[derive(Clone, Copy)]
pub struct Vista {
    pub curso: Memo<Option<Curso>>,
    pub informe: Memo<Option<Informe>>,
}

#[component]
pub fn Editor() -> impl IntoView {
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
            app.exportar(nombre_archivo(&c.nombre, ".esquema.json"), c.esquema().a_json());
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
