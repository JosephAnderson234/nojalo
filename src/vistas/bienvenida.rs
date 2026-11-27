use leptos::prelude::*;
use nojalo_core::Curso;

use crate::componentes::BotonImportar;
use crate::estado::{nuevo_id, periodo_actual, Global};
use crate::iconos::Icono;

#[component]
pub fn Bienvenida() -> impl IntoView {
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
