//! Persistencia: comandos de Tauri (SQLite) o, fuera de Tauri (`trunk serve`
//! en un navegador), localStorage como respaldo para desarrollo.

use libreta_core::Curso;
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

const CLAVE: &str = "libreta.cursos";

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
