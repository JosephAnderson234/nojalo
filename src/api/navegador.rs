//! Respaldo para el navegador: cursos en localStorage y exportación como descarga.

use nojalo_core::Curso;
use wasm_bindgen::{JsCast, JsValue};

use super::error;

const CLAVE: &str = "nojalo.cursos";

fn almacen() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

pub fn listar() -> Vec<Curso> {
    almacen()
        .and_then(|s| s.get_item(CLAVE).ok().flatten())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn escribir(v: &[Curso]) {
    if let (Some(s), Ok(t)) = (almacen(), serde_json::to_string(v)) {
        let _ = s.set_item(CLAVE, &t);
    }
}

pub fn guardar(curso: &Curso) {
    let mut v = listar();
    match v.iter_mut().find(|c| c.id == curso.id) {
        Some(c) => *c = curso.clone(),
        None => v.push(curso.clone()),
    }
    escribir(&v);
}

pub fn eliminar(id: &str) {
    let mut v = listar();
    v.retain(|c| c.id != id);
    escribir(&v);
}

pub fn descargar(nombre: &str, contenido: &str) -> Result<(), String> {
    descargar_js(nombre, contenido).map_err(error)
}

fn descargar_js(nombre: &str, contenido: &str) -> Result<(), JsValue> {
    let partes = js_sys::Array::of1(&JsValue::from_str(contenido));
    let opciones = web_sys::BlobPropertyBag::new();
    opciones.set_type("application/json");
    let blob = web_sys::Blob::new_with_str_sequence_and_options(&partes, &opciones)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    let documento = web_sys::window().and_then(|w| w.document()).ok_or("sin documento")?;
    let a: web_sys::HtmlAnchorElement = documento.create_element("a")?.dyn_into()?;
    a.set_href(&url);
    a.set_download(nombre);
    a.click();
    web_sys::Url::revoke_object_url(&url)?;
    Ok(())
}
