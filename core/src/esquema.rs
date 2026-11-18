//! Esquemas de calificación: la estructura y configuración de un curso
//! (árbol de evaluaciones, pesos, candados, eliminación de menores, escala)
//! **sin notas**, para exportarla a un archivo e importarla en otro curso.
//!
//! Formatos de archivo (JSON):
//! - `nojalo/esquema`: un solo esquema.
//! - `nojalo/paquete`: varios esquemas, por ejemplo todos los cursos de un ciclo.
//!
//! Al importar también se acepta un curso completo (el JSON interno de Nojalo):
//! se le quitan las notas.

use serde::{Deserialize, Serialize};

use crate::modelo::*;

pub const FORMATO_ESQUEMA: &str = "nojalo/esquema";
pub const FORMATO_PAQUETE: &str = "nojalo/paquete";
/// Versión del formato que escribe esta app (y la más nueva que sabe leer).
pub const VERSION_FORMATO: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Esquema {
    pub formato: String,
    pub version: u32,
    pub nombre: String,
    pub creditos: f64,
    pub escala_max: f64,
    pub aprobatoria: f64,
    pub decimales: u8,
    pub raiz: Nodo,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Paquete {
    pub formato: String,
    pub version: u32,
    /// Ciclo de origen (informativo).
    #[serde(default)]
    pub ciclo: String,
    pub esquemas: Vec<Esquema>,
}

fn sin_notas(n: &mut Nodo) {
    match &mut n.tipo {
        Tipo::Nota { valor } => *valor = None,
        Tipo::Grupo { hijos, .. } => hijos.iter_mut().for_each(sin_notas),
    }
}

/// Reasigna los ids en preorden empezando en `siguiente` (la raíz recibe el primero).
fn renumerar(n: &mut Nodo, siguiente: &mut Id) {
    n.id = *siguiente;
    *siguiente += 1;
    if let Tipo::Grupo { hijos, .. } = &mut n.tipo {
        for h in hijos {
            renumerar(h, siguiente);
        }
    }
}

fn finito_no_negativo(x: f64) -> bool {
    x.is_finite() && x >= 0.0
}

fn validar_nodo(n: &Nodo, ruta: &str) -> Result<(), String> {
    let donde = if ruta.is_empty() { n.nombre.clone() } else { format!("{ruta} › {}", n.nombre) };
    if let Peso::Grupo(p) | Peso::Final(p) = n.peso {
        if !finito_no_negativo(p) {
            return Err(format!("«{donde}»: el peso debe ser un número mayor o igual a 0"));
        }
    }
    if let Some(c) = &n.candado {
        if !finito_no_negativo(c.minimo) || !finito_no_negativo(c.tope) {
            return Err(format!("«{donde}»: el candado tiene valores inválidos"));
        }
    }
    if let Tipo::Nota { valor: Some(v) } = n.tipo {
        if !v.is_finite() {
            return Err(format!("«{donde}»: nota inválida"));
        }
    }
    n.hijos().iter().try_for_each(|h| validar_nodo(h, &donde))
}

impl Esquema {
    /// Comprueba que el esquema se pueda usar sin romper el motor.
    pub fn validar(&self) -> Result<(), String> {
        if self.version > VERSION_FORMATO {
            return Err(format!(
                "es de una versión más nueva del formato (v{}); actualiza Nojalo",
                self.version
            ));
        }
        if !(self.escala_max.is_finite() && self.escala_max > 0.0) {
            return Err("la nota máxima de la escala debe ser mayor que 0".into());
        }
        if !finito_no_negativo(self.aprobatoria) || self.aprobatoria > self.escala_max {
            return Err("la nota aprobatoria debe estar entre 0 y la nota máxima".into());
        }
        if !finito_no_negativo(self.creditos) {
            return Err("los créditos deben ser un número mayor o igual a 0".into());
        }
        if !self.raiz.es_grupo() {
            return Err("la raíz del esquema debe ser un grupo".into());
        }
        validar_nodo(&self.raiz, "")
    }

    pub fn a_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("un esquema siempre se puede serializar")
    }
}

impl Paquete {
    pub fn a_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("un paquete siempre se puede serializar")
    }
}

impl Curso {
    /// Estructura y configuración del curso, sin notas.
    pub fn esquema(&self) -> Esquema {
        let mut raiz = self.raiz.clone();
        sin_notas(&mut raiz);
        Esquema {
            formato: FORMATO_ESQUEMA.into(),
            version: VERSION_FORMATO,
            nombre: self.nombre.clone(),
            creditos: self.creditos,
            escala_max: self.escala_max,
            aprobatoria: self.aprobatoria,
            decimales: self.decimales,
            raiz,
        }
    }

    /// Curso nuevo (sin notas) a partir de un esquema.
    pub fn desde_esquema(id: impl Into<String>, periodo: &str, e: Esquema) -> Curso {
        let mut c = Curso::nuevo(id, &e.nombre, periodo);
        c.aplicar_esquema(e);
        c
    }

    /// Reemplaza la estructura y configuración por las del esquema.
    /// Conserva id, nombre y ciclo; **las notas actuales se pierden**.
    pub fn aplicar_esquema(&mut self, e: Esquema) {
        let mut raiz = e.raiz;
        sin_notas(&mut raiz);
        let mut siguiente = ID_RAIZ;
        renumerar(&mut raiz, &mut siguiente);
        self.raiz = raiz;
        self.siguiente_id = siguiente;
        self.creditos = e.creditos;
        self.escala_max = e.escala_max;
        self.aprobatoria = e.aprobatoria;
        self.decimales = e.decimales.min(4);
    }
}

/// Empaqueta los esquemas de varios cursos (por ejemplo, de un ciclo).
pub fn paquete<'a>(ciclo: &str, cursos: impl IntoIterator<Item = &'a Curso>) -> Paquete {
    Paquete {
        formato: FORMATO_PAQUETE.into(),
        version: VERSION_FORMATO,
        ciclo: ciclo.into(),
        esquemas: cursos.into_iter().map(Curso::esquema).collect(),
    }
}

/// Lee uno o varios esquemas de un archivo JSON: un esquema, un paquete o un
/// curso completo de Nojalo (al que se le quitan las notas). Todos se validan.
pub fn leer_esquemas(json: &str) -> Result<Vec<Esquema>, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("El archivo no es un JSON válido ({e})."))?;
    let formato = v.get("formato").and_then(|f| f.as_str()).unwrap_or("").to_string();

    let esquemas = match formato.as_str() {
        FORMATO_ESQUEMA => {
            vec![serde_json::from_value::<Esquema>(v).map_err(|e| format!("Esquema incompleto o dañado ({e})."))?]
        }
        FORMATO_PAQUETE => {
            let p: Paquete =
                serde_json::from_value(v).map_err(|e| format!("Paquete incompleto o dañado ({e})."))?;
            if p.version > VERSION_FORMATO {
                return Err(format!(
                    "El paquete es de una versión más nueva del formato (v{}); actualiza Nojalo.",
                    p.version
                ));
            }
            p.esquemas
        }
        "" if v.get("raiz").is_some() && v.get("siguiente_id").is_some() => {
            let c: Curso = serde_json::from_value(v).map_err(|e| format!("Curso incompleto o dañado ({e})."))?;
            vec![c.esquema()]
        }
        "" => return Err("No es un esquema de Nojalo: falta el campo «formato».".into()),
        otro => return Err(format!("Formato desconocido: «{otro}».")),
    };

    if esquemas.is_empty() {
        return Err("El paquete no contiene esquemas.".into());
    }
    for e in &esquemas {
        e.validar().map_err(|m| format!("Esquema «{}»: {m}.", e.nombre))?;
    }
    Ok(esquemas)
}
