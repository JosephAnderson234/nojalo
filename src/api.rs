//! Persistencia: comandos de Tauri (SQLite) o, fuera de Tauri (`trunk serve`
//! en un navegador), localStorage como respaldo para desarrollo.

use nojalo_core::Curso;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

fn en_tauri() -> bool {
    web_sys::window()
        .and_then(|w| js_sys::Reflect::has(&w, &"__TAURI__".into()).ok())
        .unwrap_or(false)
}

fn a_js<T: Serialize>(v: &T) -> JsValue {
    v.serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .unwrap_or(JsValue::NULL)
}

fn error(e: JsValue) -> String {
    e.as_string().unwrap_or_else(|| format!("{e:?}"))
}

const CLAVE: &str = "nojalo.cursos";

fn local() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

fn leer_local() -> Vec<Curso> {
    local()
        .and_then(|s| s.get_item(CLAVE).ok().flatten())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn escribir_local(v: &[Curso]) {
    if let (Some(s), Ok(t)) = (local(), serde_json::to_string(v)) {
        let _ = s.set_item(CLAVE, &t);
    }
}

pub async fn listar() -> Result<Vec<Curso>, String> {
    if !en_tauri() {
        return Ok(leer_local());
    }
    let v = invoke("listar_cursos", js_sys::Object::new().into()).await.map_err(error)?;
    serde_wasm_bindgen::from_value(v).map_err(|e| e.to_string())
}

pub async fn guardar(curso: &Curso) -> Result<(), String> {
    if !en_tauri() {
        let mut v = leer_local();
        match v.iter_mut().find(|c| c.id == curso.id) {
            Some(c) => *c = curso.clone(),
            None => v.push(curso.clone()),
        }
        escribir_local(&v);
        return Ok(());
    }
    #[derive(Serialize)]
    struct Args<'a> {
        curso: &'a Curso,
    }
    invoke("guardar_curso", a_js(&Args { curso })).await.map(|_| ()).map_err(error)
}

pub async fn eliminar(id: &str) -> Result<(), String> {
    if !en_tauri() {
        let mut v = leer_local();
        v.retain(|c| c.id != id);
        escribir_local(&v);
        return Ok(());
    }
    #[derive(Serialize)]
    struct Args<'a> {
        id: &'a str,
    }
    invoke("eliminar_curso", a_js(&Args { id })).await.map(|_| ()).map_err(error)
}

// ───────────────────────── archivos ─────────────────────────

/// Guarda `contenido` en un archivo: diálogo nativo en Tauri, descarga en el
/// navegador. Devuelve `false` si el usuario canceló.
pub async fn exportar(nombre: &str, contenido: &str) -> Result<bool, String> {
    if en_tauri() {
        #[derive(Serialize)]
        struct Args<'a> {
            nombre: &'a str,
            contenido: &'a str,
        }
        let v = invoke("exportar_archivo", a_js(&Args { nombre, contenido })).await.map_err(error)?;
        return Ok(v.as_bool().unwrap_or(true));
    }
    descargar(nombre, contenido).map_err(error)?;
    Ok(true)
}

fn descargar(nombre: &str, contenido: &str) -> Result<(), JsValue> {
    use wasm_bindgen::JsCast;
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

/// Lee como texto un archivo elegido con `<input type="file">`.
pub async fn leer_archivo(archivo: web_sys::File) -> Result<String, String> {
    let texto = wasm_bindgen_futures::JsFuture::from(archivo.text()).await.map_err(error)?;
    texto.as_string().ok_or_else(|| "El archivo no es de texto.".to_string())
}

/// Nombre de archivo seguro a partir de un título: «Cálculo I» → `calculo-i`.
pub fn nombre_archivo(titulo: &str, sufijo: &str) -> String {
    let mut s = String::new();
    for ch in titulo.trim().to_lowercase().chars() {
        let ch = match ch {
            'á' | 'à' | 'ä' => 'a',
            'é' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'ï' => 'i',
            'ó' | 'ò' | 'ö' => 'o',
            'ú' | 'ù' | 'ü' => 'u',
            'ñ' => 'n',
            c => c,
        };
        if ch.is_ascii_alphanumeric() {
            s.push(ch);
        } else if !s.ends_with('-') && !s.is_empty() {
            s.push('-');
        }
    }
    let s = s.trim_end_matches('-');
    format!("{}{sufijo}", if s.is_empty() { "nojalo" } else { s })
}
