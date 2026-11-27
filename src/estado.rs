//! Estado global de la UI: cursos cargados, selección y avisos, más las
//! operaciones que los modifican y persisten.

use leptos::prelude::*;
use leptos::task::spawn_local;
use nojalo_core::*;

use crate::api;

#[derive(Clone, Copy)]
pub struct Global {
    pub cursos: RwSignal<Vec<Curso>>,
    pub sel: RwSignal<Option<String>>,
    /// Ciclo abierto en la vista de ciclo; si hay uno, tiene prioridad sobre el curso.
    pub ciclo: RwSignal<Option<String>>,
    pub error: RwSignal<Option<String>>,
    /// Aviso de que algo salió bien (se cierra solo).
    pub aviso: RwSignal<Option<String>>,
    /// Diálogo de confirmación abierto.
    pub confirmacion: RwSignal<Option<Confirmacion>>,
    /// Guardados en curso (para el indicador «Guardando…»).
    pub guardando: RwSignal<u32>,
    /// Si ya se guardó algo en esta sesión (para mostrar «Guardado»).
    pub guardado: RwSignal<bool>,
}

impl Global {
    pub fn nuevo() -> Self {
        Global {
            cursos: RwSignal::new(vec![]),
            sel: RwSignal::new(None),
            ciclo: RwSignal::new(None),
            error: RwSignal::new(None),
            aviso: RwSignal::new(None),
            confirmacion: RwSignal::new(None),
            guardando: RwSignal::new(0),
            guardado: RwSignal::new(false),
        }
    }

    /// Carga los cursos guardados y abre el primero.
    pub fn cargar(&self) {
        let Global { cursos, sel, error, .. } = *self;
        spawn_local(async move {
            match api::listar().await {
                Ok(v) => {
                    sel.set(v.first().map(|c| c.id.clone()));
                    cursos.set(v);
                }
                Err(e) => error.set(Some(format!("No se pudieron cargar los cursos: {e}"))),
            }
        });
    }

    /// Aplica un cambio al curso seleccionado y lo persiste.
    pub fn editar(&self, f: impl FnOnce(&mut Curso)) {
        let Some(id) = self.sel.get_untracked() else { return };
        let mut copia = None;
        self.cursos.update(|v| {
            if let Some(c) = v.iter_mut().find(|c| c.id == id) {
                f(c);
                copia = Some(c.clone());
            }
        });
        if let Some(c) = copia {
            self.guardar(c);
        }
    }

    /// Aplica un cambio a todos los cursos que cumplen `pred` y los persiste.
    pub fn editar_donde(&self, pred: impl Fn(&Curso) -> bool, f: impl Fn(&mut Curso)) {
        let mut cambiados = vec![];
        self.cursos.update(|v| {
            for c in v.iter_mut().filter(|c| pred(c)) {
                f(c);
                cambiados.push(c.clone());
            }
        });
        for c in cambiados {
            self.guardar(c);
        }
    }

    pub fn editar_nodo(&self, id: Id, f: impl FnOnce(&mut Nodo)) {
        self.editar(|c| {
            if let Some(n) = c.buscar_mut(id) {
                f(n)
            }
        })
    }

    fn guardar(&self, c: Curso) {
        let Global { error, guardando, guardado, .. } = *self;
        guardando.update(|n| *n += 1);
        spawn_local(async move {
            let r = api::guardar(&c).await;
            guardando.update(|n| *n = n.saturating_sub(1));
            match r {
                Ok(()) => guardado.set(true),
                Err(e) => error.set(Some(format!("No se pudo guardar «{}»: {e}. Vuelve a editar un campo para reintentar.", c.nombre))),
            }
        });
    }

    pub fn crear(&self, c: Curso) {
        self.ciclo.set(None);
        self.sel.set(Some(c.id.clone()));
        self.cursos.update(|v| v.push(c.clone()));
        self.guardar(c);
    }

    /// Muestra el diálogo de confirmación; `accion` corre solo si se acepta.
    pub fn confirmar(
        &self,
        titulo: &str,
        mensaje: String,
        boton: &'static str,
        accion: impl Fn() + Send + Sync + 'static,
    ) {
        self.confirmacion.set(Some(Confirmacion {
            titulo: titulo.into(),
            mensaje,
            boton,
            accion: Callback::new(move |()| accion()),
        }));
    }

    pub fn eliminar_curso(&self, id: String) {
        self.cursos.update(|v| v.retain(|x| x.id != id));
        if self.sel.get_untracked().as_deref() == Some(id.as_str()) {
            self.sel.set(self.cursos.with_untracked(|v| v.first().map(|c| c.id.clone())));
        }
        let Global { error, aviso, .. } = *self;
        spawn_local(async move {
            match api::eliminar(&id).await {
                Ok(()) => aviso.set(Some("Curso eliminado.".into())),
                Err(e) => error.set(Some(format!("No se pudo eliminar el curso: {e}"))),
            }
        });
    }

    /// Mueve todos los cursos de `viejo` a `nuevo` y abre el ciclo resultante.
    pub fn renombrar_ciclo(&self, viejo: String, nuevo: String) {
        self.editar_donde(|c| c.periodo == viejo, |c| c.periodo = nuevo.clone());
        self.abrir_ciclo(nuevo);
    }

    /// Abre un ciclo vacío con el nombre que sigue al más reciente.
    pub fn nuevo_ciclo(&self) {
        let existentes = self.cursos.with_untracked(|v| ciclos(v));
        self.abrir_ciclo(ciclo_siguiente(&existentes, &periodo_actual()));
    }

    pub fn abrir_curso(&self, id: String) {
        self.ciclo.set(None);
        self.sel.set(Some(id));
    }

    pub fn abrir_ciclo(&self, ciclo: String) {
        self.ciclo.set(Some(ciclo));
    }

    /// Ciclo donde caen los cursos nuevos: el ciclo abierto, el del curso
    /// abierto o, si no hay ninguno, el semestre actual.
    pub fn ciclo_destino(&self) -> String {
        if let Some(c) = self.ciclo.get_untracked() {
            return c;
        }
        let sel = self.sel.get_untracked();
        self.cursos
            .with_untracked(|v| v.iter().find(|c| Some(&c.id) == sel.as_ref()).map(|c| c.periodo.clone()))
            .unwrap_or_else(periodo_actual)
    }

    /// Crea cursos (sin notas) a partir de los esquemas de un archivo.
    pub fn importar(&self, texto: &str, ciclo: String) {
        let esquemas = match leer_esquemas(texto) {
            Ok(e) => e,
            Err(e) => return self.error.set(Some(format!("No se pudo importar: {e}"))),
        };
        let base = nuevo_id();
        let nuevos: Vec<Curso> = esquemas
            .into_iter()
            .enumerate()
            .map(|(i, e)| Curso::desde_esquema(format!("{base}{i}"), &ciclo, e))
            .collect();
        let n = nuevos.len();
        let primero = nuevos[0].id.clone();
        self.cursos.update(|v| v.extend(nuevos.iter().cloned()));
        for c in nuevos {
            self.guardar(c);
        }
        let donde = if ciclo.is_empty() { "sin ciclo".to_string() } else { format!("en {ciclo}") };
        if n == 1 {
            self.abrir_curso(primero);
            self.aviso.set(Some(format!("Esquema importado {donde}. Ya puedes registrar tus notas.")));
        } else {
            self.sel.set(Some(primero));
            self.abrir_ciclo(ciclo);
            self.aviso.set(Some(format!("{n} esquemas importados {donde}.")));
        }
    }

    /// Guarda un archivo (diálogo nativo en escritorio, descarga en el navegador).
    pub fn exportar(&self, nombre: String, contenido: String) {
        let Global { error, aviso, .. } = *self;
        spawn_local(async move {
            match api::exportar(&nombre, &contenido).await {
                Ok(true) => aviso.set(Some("Esquema exportado.".into())),
                Ok(false) => {}
                Err(e) => error.set(Some(format!("No se pudo exportar: {e}"))),
            }
        });
    }
}

/// Id único para un curso nuevo.
pub fn nuevo_id() -> String {
    format!("c{:x}{:04x}", js_sys::Date::now() as u64, (js_sys::Math::random() * 65535.0) as u32)
}

/// Semestre en curso, por ejemplo `2026-2`.
pub fn periodo_actual() -> String {
    let d = js_sys::Date::new_0();
    format!("{}-{}", d.get_full_year(), if d.get_month() < 6 { 1 } else { 2 })
}

/// Pregunta pendiente del diálogo de confirmación de la app.
///
/// No se usa `window.confirm`: en Tauri el plugin de diálogos lo reemplaza por
/// una versión asíncrona que, llamada de forma síncrona, siempre devuelve `false`.
#[derive(Clone)]
pub struct Confirmacion {
    pub titulo: String,
    pub mensaje: String,
    pub boton: &'static str,
    pub accion: Callback<()>,
}
