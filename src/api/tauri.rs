//! Comandos del backend de escritorio (`src-tauri`).

use nojalo_core::Curso;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use super::error;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

fn a_js<T: Serialize>(v: &T) -> JsValue {
    v.serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .unwrap_or(JsValue::NULL)
}

pub async fn listar() -> Result<Vec<Curso>, String> {
    let v = invoke("listar_cursos", js_sys::Object::new().into()).await.map_err(error)?;
    serde_wasm_bindgen::from_value(v).map_err(|e| e.to_string())
}

pub async fn guardar(curso: &Curso) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args<'a> {
        curso: &'a Curso,
    }
    invoke("guardar_curso", a_js(&Args { curso })).await.map(|_| ()).map_err(error)
}

pub async fn eliminar(id: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args<'a> {
        id: &'a str,
    }
    invoke("eliminar_curso", a_js(&Args { id })).await.map(|_| ()).map_err(error)
}

pub async fn exportar(nombre: &str, contenido: &str) -> Result<bool, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        nombre: &'a str,
        contenido: &'a str,
    }
    let v = invoke("exportar_archivo", a_js(&Args { nombre, contenido })).await.map_err(error)?;
    Ok(v.as_bool().unwrap_or(true))
}
