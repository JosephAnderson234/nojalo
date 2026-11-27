use leptos::prelude::*;

use crate::formato::{corto, num};

/// Campo numérico que confirma al salir del input o con Enter.
///
/// Vacío = sin valor. Si el texto no es un número no se guarda nada: el campo
/// se marca como inválido y conserva lo escrito para que se pueda corregir.
#[component]
pub fn Numero(
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
