use leptos::prelude::*;

use crate::estado::{Confirmacion, Global};
use crate::iconos::Icono;

#[component]
pub fn Dialogo() -> impl IntoView {
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
