//! Avisos flotantes (toasts): éxito en `Global::aviso`, error en `Global::error`.

use std::time::Duration;

use leptos::prelude::*;

use crate::estado::Global;
use crate::iconos::Icono;

/// Cierra el aviso tras `segundos`, salvo que ya lo haya reemplazado otro.
fn autocerrar(senal: RwSignal<Option<String>>, segundos: u64) {
    Effect::new(move |_| {
        if let Some(texto) = senal.get() {
            set_timeout(
                move || {
                    if senal.get_untracked().as_deref() == Some(texto.as_str()) {
                        senal.set(None);
                    }
                },
                Duration::from_secs(segundos),
            );
        }
    });
}

#[component]
fn Toast(senal: RwSignal<Option<String>>, clase: &'static str, icono: &'static str) -> impl IntoView {
    move || {
        senal.get().map(|texto| {
            view! {
                <div class=clase>
                    <Icono nombre=icono />
                    <span>{texto}</span>
                    <button class="icono-btn" aria-label="Cerrar aviso" on:click=move |_| senal.set(None)>
                        <Icono nombre="cerrar" />
                    </button>
                </div>
            }
        })
    }
}

#[component]
pub fn Avisos() -> impl IntoView {
    let app = expect_context::<Global>();
    // Los errores se leen con calma (8 s); los avisos de éxito duran 4 s.
    autocerrar(app.error, 8);
    autocerrar(app.aviso, 4);

    view! {
        <div class="pila-avisos">
            <div class="avisos" role="status" aria-live="polite">
                <Toast senal=app.aviso clase="toast info" icono="check" />
            </div>
            <div class="avisos" role="alert" aria-live="assertive">
                <Toast senal=app.error clase="toast" icono="alerta" />
            </div>
        </div>
    }
}
