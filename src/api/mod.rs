//! Persistencia y archivos. Dentro de Tauri se usan los comandos del backend
//! (SQLite y diálogos nativos); en un navegador (`trunk serve`, GitHub Pages)
//! se usa localStorage y descargas.

mod navegador;
mod tauri;

use nojalo_core::Curso;

fn en_tauri() -> bool {
    web_sys::window()
        .and_then(|w| js_sys::Reflect::has(&w, &"__TAURI__".into()).ok())
        .unwrap_or(false)
}

pub async fn listar() -> Result<Vec<Curso>, String> {
    if en_tauri() {
        tauri::listar().await
    } else {
        Ok(navegador::listar())
    }
}

pub async fn guardar(curso: &Curso) -> Result<(), String> {
    if en_tauri() {
        tauri::guardar(curso).await
    } else {
        navegador::guardar(curso);
        Ok(())
    }
}

pub async fn eliminar(id: &str) -> Result<(), String> {
    if en_tauri() {
        tauri::eliminar(id).await
    } else {
        navegador::eliminar(id);
        Ok(())
    }
}

/// Guarda `contenido` en un archivo: diálogo nativo en Tauri, descarga en el
/// navegador. Devuelve `false` si el usuario canceló.
pub async fn exportar(nombre: &str, contenido: &str) -> Result<bool, String> {
    if en_tauri() {
        tauri::exportar(nombre, contenido).await
    } else {
        navegador::descargar(nombre, contenido)?;
        Ok(true)
    }
}

/// Lee como texto un archivo elegido con `<input type="file">`.
pub async fn leer_archivo(archivo: web_sys::File) -> Result<String, String> {
    let texto = wasm_bindgen_futures::JsFuture::from(archivo.text()).await.map_err(error)?;
    texto.as_string().ok_or_else(|| "El archivo no es de texto.".to_string())
}

fn error(e: wasm_bindgen::JsValue) -> String {
    e.as_string().unwrap_or_else(|| format!("{e:?}"))
}
