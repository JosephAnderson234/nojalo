//! Modelo de datos: un curso es un árbol de grupos y notas.

use serde::{Deserialize, Serialize};

pub type Id = u32;

/// Cuánto pesa un nodo dentro de su grupo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "modo", content = "valor")]
pub enum Peso {
    /// Reparte en partes iguales lo que dejan libre los hermanos con peso explícito.
    Auto,
    /// Porcentaje relativo al grupo padre (los hermanos deberían sumar 100%).
    Grupo(f64),
    /// Porcentaje relativo a la nota final del curso; se convierte a peso local
    /// dividiendo por el peso efectivo del grupo padre.
    Final(f64),
}

/// Sobre qué nota actúa un candado cuando se rompe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alcance {
    /// Limita la nota del grupo que contiene al nodo.
    Grupo,
    /// Limita la nota final del curso.
    Final,
}

/// "Nota mínima": si el nodo queda por debajo de `minimo`, la nota del
/// alcance elegido no puede superar `tope`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Candado {
    pub minimo: f64,
    pub tope: f64,
    pub alcance: Alcance,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Tipo {
    Nota {
        valor: Option<f64>,
    },
    Grupo {
        hijos: Vec<Nodo>,
        /// Cuántas de las notas más bajas del grupo se descartan.
        eliminar_menores: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Nodo {
    pub id: Id,
    pub nombre: String,
    pub peso: Peso,
    pub candado: Option<Candado>,
    pub tipo: Tipo,
}

impl Nodo {
    pub fn nota(id: Id, nombre: &str, peso: Peso, valor: Option<f64>) -> Self {
        Nodo { id, nombre: nombre.into(), peso, candado: None, tipo: Tipo::Nota { valor } }
    }

    pub fn grupo(id: Id, nombre: &str, peso: Peso, hijos: Vec<Nodo>) -> Self {
        Nodo {
            id,
            nombre: nombre.into(),
            peso,
            candado: None,
            tipo: Tipo::Grupo { hijos, eliminar_menores: 0 },
        }
    }

    pub fn con_candado(mut self, minimo: f64, tope: f64, alcance: Alcance) -> Self {
        self.candado = Some(Candado { minimo, tope, alcance });
        self
    }

    pub fn eliminando(mut self, n: usize) -> Self {
        if let Tipo::Grupo { eliminar_menores, .. } = &mut self.tipo {
            *eliminar_menores = n;
        }
        self
    }

    pub fn es_grupo(&self) -> bool {
        matches!(self.tipo, Tipo::Grupo { .. })
    }

    pub fn hijos(&self) -> &[Nodo] {
        match &self.tipo {
            Tipo::Grupo { hijos, .. } => hijos,
            Tipo::Nota { .. } => &[],
        }
    }

    /// Recorre el subárbol en preorden.
    pub fn recorrer<'a>(&'a self, f: &mut impl FnMut(&'a Nodo)) {
        f(self);
        for h in self.hijos() {
            h.recorrer(f);
        }
    }

    fn buscar(&self, id: Id) -> Option<&Nodo> {
        if self.id == id {
            return Some(self);
        }
        self.hijos().iter().find_map(|h| h.buscar(id))
    }

    fn buscar_mut(&mut self, id: Id) -> Option<&mut Nodo> {
        if self.id == id {
            return Some(self);
        }
        if let Tipo::Grupo { hijos, .. } = &mut self.tipo {
            for h in hijos {
                if let Some(n) = h.buscar_mut(id) {
                    return Some(n);
                }
            }
        }
        None
    }

    /// Lista de hermanos que contiene al nodo `id`.
    fn hermanos_mut(&mut self, id: Id) -> Option<&mut Vec<Nodo>> {
        if let Tipo::Grupo { hijos, .. } = &mut self.tipo {
            if hijos.iter().any(|h| h.id == id) {
                return Some(hijos);
            }
            for h in hijos {
                if let Some(v) = h.hermanos_mut(id) {
                    return Some(v);
                }
            }
        }
        None
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Curso {
    pub id: String,
    pub nombre: String,
    pub periodo: String,
    pub creditos: f64,
    /// Nota máxima de la escala (20 en Perú, 10, 100, 7...).
    pub escala_max: f64,
    /// Nota final mínima para aprobar.
    pub aprobatoria: f64,
    /// Decimales al mostrar.
    pub decimales: u8,
    pub raiz: Nodo,
    pub siguiente_id: Id,
}

pub const ID_RAIZ: Id = 0;

impl Curso {
    pub fn nuevo(id: impl Into<String>, nombre: &str, periodo: &str) -> Self {
        Curso {
            id: id.into(),
            nombre: nombre.into(),
            periodo: periodo.into(),
            creditos: 3.0,
            escala_max: 20.0,
            aprobatoria: 10.5,
            decimales: 2,
            raiz: Nodo::grupo(ID_RAIZ, "Nota final", Peso::Auto, vec![]),
            siguiente_id: 1,
        }
    }

    /// Curso de muestra con grupos anidados, pesos mixtos, candado y eliminación.
    pub fn ejemplo(id: impl Into<String>, periodo: &str) -> Self {
        use Peso::*;
        let mut c = Curso::nuevo(id, "Algoritmos y Estructuras de Datos", periodo);
        c.creditos = 4.0;
        let practicas = Nodo::grupo(
            3,
            "Prácticas calificadas",
            Grupo(60.0),
            vec![
                Nodo::nota(4, "PC1", Auto, Some(14.0)),
                Nodo::nota(5, "PC2", Auto, Some(8.0)),
                Nodo::nota(6, "PC3", Auto, Some(16.0)),
                Nodo::nota(7, "PC4", Auto, None),
            ],
        )
        .eliminando(1);
        let labs = Nodo::grupo(
            8,
            "Laboratorios",
            Grupo(40.0),
            vec![
                Nodo::nota(9, "Lab 1", Auto, Some(18.0)),
                Nodo::nota(10, "Lab 2", Auto, Some(15.0)),
                Nodo::nota(11, "Lab 3", Auto, None),
            ],
        );
        c.raiz = Nodo::grupo(
            ID_RAIZ,
            "Nota final",
            Auto,
            vec![
                Nodo::grupo(1, "Evaluación continua", Final(40.0), vec![practicas, labs]),
                Nodo::nota(2, "Examen parcial", Final(25.0), Some(12.0)),
                Nodo::nota(12, "Examen final", Final(35.0), None).con_candado(
                    10.5,
                    10.0,
                    Alcance::Final,
                ),
            ],
        );
        c.siguiente_id = 13;
        c
    }

    pub fn buscar(&self, id: Id) -> Option<&Nodo> {
        self.raiz.buscar(id)
    }

    pub fn buscar_mut(&mut self, id: Id) -> Option<&mut Nodo> {
        self.raiz.buscar_mut(id)
    }

    pub fn hijos_ids(&self, id: Id) -> Vec<Id> {
        self.buscar(id).map(|n| n.hijos().iter().map(|h| h.id).collect()).unwrap_or_default()
    }

    /// Agrega una nota o un grupo vacío al final del grupo `padre`.
    pub fn agregar(&mut self, padre: Id, grupo: bool) -> Option<Id> {
        let id = self.siguiente_id;
        let Some(Nodo { tipo: Tipo::Grupo { hijos, .. }, .. }) = self.raiz.buscar_mut(padre) else {
            return None;
        };
        let nombre = if grupo { "Nuevo grupo".to_string() } else { format!("Nota {}", hijos.len() + 1) };
        hijos.push(if grupo {
            Nodo::grupo(id, &nombre, Peso::Auto, vec![])
        } else {
            Nodo::nota(id, &nombre, Peso::Auto, None)
        });
        self.siguiente_id += 1;
        Some(id)
    }

    pub fn eliminar(&mut self, id: Id) -> bool {
        match self.raiz.hermanos_mut(id) {
            Some(v) => {
                v.retain(|h| h.id != id);
                true
            }
            None => false,
        }
    }

    /// Mueve un nodo entre sus hermanos (`-1` sube, `1` baja).
    pub fn mover(&mut self, id: Id, delta: isize) -> bool {
        let Some(v) = self.raiz.hermanos_mut(id) else { return false };
        let i = v.iter().position(|h| h.id == id).unwrap();
        let j = i as isize + delta;
        if j < 0 || j as usize >= v.len() {
            return false;
        }
        v.swap(i, j as usize);
        true
    }
}
