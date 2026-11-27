use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::estado::Global;

/// Botón que abre el selector de archivos y entrega el texto del `.json` elegido.
#[component]
pub fn BotonImportar(
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
