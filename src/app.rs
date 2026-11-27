use leptos::prelude::*;

use crate::componentes::{Avisos, Dialogo};
use crate::estado::Global;
use crate::vistas::{Barra, Bienvenida, Editor, VistaCiclo};

#[component]
pub fn App() -> impl IntoView {
    let app = Global::nuevo();
    provide_context(app);
    app.cargar();

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
            <Avisos />
        </div>
    }
}
