use leptos::prelude::*;

/// Trazos de [Lucide](https://lucide.dev) (ISC), viewBox 24×24.
fn trazos(nombre: &str) -> &'static str {
    match nombre {
        "logo" => r#"<path d="M21.42 10.922a1 1 0 0 0-.019-1.838L12.83 5.18a2 2 0 0 0-1.66 0L2.6 9.08a1 1 0 0 0 0 1.832l8.57 3.908a2 2 0 0 0 1.66 0z"/><path d="M22 10v6"/><path d="M6 12.5V16a6 3 0 0 0 12 0v-3.5"/>"#,
        "candado" => r#"<rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>"#,
        "ajustes" => r#"<line x1="21" x2="14" y1="4" y2="4"/><line x1="10" x2="3" y1="4" y2="4"/><line x1="21" x2="12" y1="12" y2="12"/><line x1="8" x2="3" y1="12" y2="12"/><line x1="21" x2="16" y1="20" y2="20"/><line x1="12" x2="3" y1="20" y2="20"/><line x1="14" x2="14" y1="2" y2="6"/><line x1="8" x2="8" y1="10" y2="14"/><line x1="16" x2="16" y1="18" y2="22"/>"#,
        "subir" => r#"<path d="m5 12 7-7 7 7"/><path d="M12 19V5"/>"#,
        "bajar" => r#"<path d="M12 5v14"/><path d="m19 12-7 7-7-7"/>"#,
        "cerrar" => r#"<path d="M18 6 6 18"/><path d="m6 6 12 12"/>"#,
        "abierto" => r#"<path d="m6 9 6 6 6-6"/>"#,
        "cerrado" => r#"<path d="m9 18 6-6-6-6"/>"#,
        "mas" => r#"<path d="M5 12h14"/><path d="M12 5v14"/>"#,
        "carpeta" => r#"<path d="M12 10v6"/><path d="M9 13h6"/><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>"#,
        "papelera" => r#"<path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/>"#,
        "alerta" => r#"<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/>"#,
        "check" => r#"<path d="M20 6 9 17l-5-5"/>"#,
        "libro" => r#"<path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H19a1 1 0 0 1 1 1v18a1 1 0 0 1-1 1H6.5a1 1 0 0 1 0-5H20"/>"#,
        "importar" => r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" x2="12" y1="3" y2="15"/>"#,
        "exportar" => r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/>"#,
        "ciclo" => r#"<path d="M8 2v4"/><path d="M16 2v4"/><rect width="18" height="18" x="3" y="4" rx="2"/><path d="M3 10h18"/>"#,
        "flecha" => r#"<path d="M5 12h14"/><path d="m12 5 7 7-7 7"/>"#,
        "aplicar" => r#"<path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M8 16H3v5"/>"#,
        _ => "",
    }
}

/// Icono decorativo (oculto para lectores de pantalla): el control que lo
/// contiene debe llevar su propio texto o `aria-label`.
#[component]
pub fn Icono(nombre: &'static str, #[prop(default = 16)] tam: u32) -> impl IntoView {
    view! {
        <svg
            class="icono"
            width=tam.to_string()
            height=tam.to_string()
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            focusable="false"
            inner_html=trazos(nombre)
        ></svg>
    }
}
