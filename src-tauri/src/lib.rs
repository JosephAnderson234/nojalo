//! Backend de escritorio: guarda cada curso como un documento JSON en SQLite.
//! Toda la lógica de notas vive en `libreta-core` y se ejecuta en el frontend.

use std::sync::Mutex;

use libreta_core::Curso;
use rusqlite::{params, Connection};
use tauri::{Manager, State};

struct Db(Mutex<Connection>);

const ESQUEMA: &str = "
CREATE TABLE IF NOT EXISTS cursos (
    id          TEXT PRIMARY KEY,
    periodo     TEXT NOT NULL,
    datos       TEXT NOT NULL,
    actualizado INTEGER NOT NULL DEFAULT (unixepoch())
);";

fn texto<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[tauri::command]
fn listar_cursos(db: State<Db>) -> Result<Vec<Curso>, String> {
    let con = db.0.lock().map_err(texto)?;
    let mut st = con
        .prepare("SELECT datos FROM cursos ORDER BY periodo DESC, rowid")
        .map_err(texto)?;
    let filas = st.query_map([], |r| r.get::<_, String>(0)).map_err(texto)?;
    filas
        .map(|f| serde_json::from_str(&f.map_err(texto)?).map_err(texto))
        .collect()
}

#[tauri::command]
fn guardar_curso(db: State<Db>, curso: Curso) -> Result<(), String> {
    let datos = serde_json::to_string(&curso).map_err(texto)?;
    db.0.lock()
        .map_err(texto)?
        .execute(
            "INSERT INTO cursos (id, periodo, datos) VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET periodo = ?2, datos = ?3, actualizado = unixepoch()",
            params![curso.id, curso.periodo, datos],
        )
        .map(|_| ())
        .map_err(texto)
}

#[tauri::command]
fn eliminar_curso(db: State<Db>, id: String) -> Result<(), String> {
    db.0.lock()
        .map_err(texto)?
        .execute("DELETE FROM cursos WHERE id = ?1", params![id])
        .map(|_| ())
        .map_err(texto)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let con = Connection::open(dir.join("libreta.db"))?;
            con.execute_batch(ESQUEMA)?;
            app.manage(Db(Mutex::new(con)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![listar_cursos, guardar_curso, eliminar_curso])
        .run(tauri::generate_context!())
        .expect("error al iniciar Libreta");
}
