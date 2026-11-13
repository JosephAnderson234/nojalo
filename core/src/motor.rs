//! Evaluación del árbol: pesos efectivos, eliminación de menores, candados,
//! escenarios (actual / peor caso / mejor caso) y nota necesaria para aprobar.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::modelo::*;

const EPS: f64 = 1e-9;

/// Cómo se rellenan las notas que aún no tienen valor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Escenario {
    /// Solo cuenta lo rendido; los pesos se renormalizan sobre lo evaluado.
    Actual,
    /// Lo pendiente vale 0.
    Pesimista,
    /// Lo pendiente vale la nota máxima.
    Optimista,
    /// Lo pendiente vale `x`.
    Uniforme(f64),
}

impl Escenario {
    fn rellenar(self, max: f64) -> Option<f64> {
        match self {
            Escenario::Actual => None,
            Escenario::Pesimista => Some(0.0),
            Escenario::Optimista => Some(max),
            Escenario::Uniforme(x) => Some(x),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PesoResuelto {
    /// Fracción (0..1) dentro del grupo padre.
    pub local: f64,
    /// Fracción (0..1) de la nota final del curso.
    pub final_: f64,
}

/// Pesos locales de los hijos de un grupo cuyo peso efectivo en el final es `share`.
fn pesos_locales(hijos: &[Nodo], share: f64) -> Vec<f64> {
    let mut autos = 0usize;
    let explicitos: Vec<Option<f64>> = hijos
        .iter()
        .map(|h| match h.peso {
            Peso::Auto => {
                autos += 1;
                None
            }
            Peso::Grupo(p) => Some(p / 100.0),
            Peso::Final(p) => Some(if share > EPS { p / 100.0 / share } else { 0.0 }),
        })
        .collect();
    let fijos: f64 = explicitos.iter().flatten().sum();
    let resto = if autos > 0 { (1.0 - fijos).max(0.0) / autos as f64 } else { 0.0 };
    explicitos.into_iter().map(|p| p.unwrap_or(resto)).collect()
}

/// Resuelve el peso local y el peso efectivo sobre el final de cada nodo.
pub fn resolver_pesos(raiz: &Nodo) -> HashMap<Id, PesoResuelto> {
    fn visitar(n: &Nodo, share: f64, out: &mut HashMap<Id, PesoResuelto>) {
        let hijos = n.hijos();
        for (h, local) in hijos.iter().zip(pesos_locales(hijos, share)) {
            let final_ = local * share;
            out.insert(h.id, PesoResuelto { local, final_ });
            visitar(h, final_, out);
        }
    }
    let mut out = HashMap::new();
    out.insert(raiz.id, PesoResuelto { local: 1.0, final_: 1.0 });
    visitar(raiz, 1.0, &mut out);
    out
}

#[derive(Clone, Debug, Default)]
struct Traza {
    nota: Option<f64>,
    eliminado: bool,
    candado_roto: Option<bool>,
}

struct Corrida {
    final_: Option<f64>,
    traza: HashMap<Id, Traza>,
}

struct Ctx<'a> {
    esc: Escenario,
    max: f64,
    pesos: &'a HashMap<Id, PesoResuelto>,
    topes_final: Vec<f64>,
    traza: HashMap<Id, Traza>,
}

fn evaluar(n: &Nodo, cx: &mut Ctx) -> Option<f64> {
    let nota = match &n.tipo {
        Tipo::Nota { valor } => valor.map(|v| v.clamp(0.0, cx.max)).or(cx.esc.rellenar(cx.max)),
        Tipo::Grupo { hijos, eliminar_menores } => {
            let mut items: Vec<(Id, f64, f64)> = Vec::new(); // (id, nota, peso local)
            let mut faltantes = 0usize;
            let mut topes_grupo: Vec<f64> = Vec::new();
            for h in hijos {
                let nota_h = evaluar(h, cx);
                if let Some(c) = &h.candado {
                    let roto = nota_h.map(|x| x < c.minimo - EPS);
                    cx.traza.entry(h.id).or_default().candado_roto = roto;
                    if roto == Some(true) {
                        match c.alcance {
                            Alcance::Grupo => topes_grupo.push(c.tope),
                            Alcance::Final => cx.topes_final.push(c.tope),
                        }
                    }
                }
                match nota_h {
                    Some(x) => items.push((h.id, x, cx.pesos.get(&h.id).map_or(0.0, |p| p.local))),
                    None => faltantes += 1,
                }
            }
            // Con notas pendientes no se sabe aún cuál será la menor: solo se
            // descartan las que seguro saldrán (las pendientes podrían ser las eliminadas).
            let quitar = if cx.esc == Escenario::Actual {
                eliminar_menores.saturating_sub(faltantes)
            } else {
                *eliminar_menores
            }
            .min(items.len().saturating_sub(1));
            if quitar > 0 {
                let mut orden: Vec<usize> = (0..items.len()).collect();
                orden.sort_by(|&a, &b| items[a].1.total_cmp(&items[b].1).then(a.cmp(&b)));
                let mut fuera: Vec<usize> = orden[..quitar].to_vec();
                fuera.sort_unstable_by(|a, b| b.cmp(a));
                for i in fuera {
                    let (id, ..) = items.remove(i);
                    cx.traza.entry(id).or_default().eliminado = true;
                }
            }
            let peso_total: f64 = items.iter().map(|i| i.2).sum();
            let promedio = if items.is_empty() {
                None
            } else if peso_total > EPS {
                Some(items.iter().map(|i| i.1 * i.2).sum::<f64>() / peso_total)
            } else {
                Some(items.iter().map(|i| i.1).sum::<f64>() / items.len() as f64)
            };
            promedio.map(|p| topes_grupo.iter().fold(p, |a, t| a.min(*t)))
        }
    };
    cx.traza.entry(n.id).or_default().nota = nota;
    nota
}

fn correr(curso: &Curso, esc: Escenario, pesos: &HashMap<Id, PesoResuelto>) -> Corrida {
    let mut cx = Ctx {
        esc,
        max: curso.escala_max,
        pesos,
        topes_final: Vec::new(),
        traza: HashMap::new(),
    };
    let nota = evaluar(&curso.raiz, &mut cx);
    let final_ = nota.map(|n| cx.topes_final.iter().fold(n, |a, t| a.min(*t)));
    Corrida { final_, traza: cx.traza }
}

/// Nota final si todo lo pendiente saliera `x`.
pub fn simular(curso: &Curso, x: f64) -> Option<f64> {
    correr(curso, Escenario::Uniforme(x), &resolver_pesos(&curso.raiz)).final_
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EstadoCandado {
    Cumple,
    Roto,
    Pendiente,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Estado {
    /// Aun sacando 0 en todo lo que falta, aprueba.
    Aprobado,
    /// Aun sacando el máximo en todo lo que falta, no aprueba.
    Desaprobado,
    EnJuego,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InformeNodo {
    pub actual: Option<f64>,
    pub minimo: f64,
    pub maximo: f64,
    pub peso: PesoResuelto,
    pub eliminado: bool,
    pub candado: Option<EstadoCandado>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Informe {
    pub nodos: HashMap<Id, InformeNodo>,
    /// Nota final con lo rendido hasta ahora (pesos renormalizados).
    pub actual: Option<f64>,
    pub minimo: f64,
    pub maximo: f64,
    pub estado: Estado,
    /// Promedio que hace falta en todo lo pendiente para aprobar.
    pub necesaria: Option<f64>,
    /// Fracción (0..1) del peso del curso que ya tiene nota.
    pub avance: f64,
    pub advertencias: Vec<String>,
}

pub fn informe(curso: &Curso) -> Informe {
    let pesos = resolver_pesos(&curso.raiz);
    let act = correr(curso, Escenario::Actual, &pesos);
    let pes = correr(curso, Escenario::Pesimista, &pesos);
    let opt = correr(curso, Escenario::Optimista, &pesos);

    let mut nodos = HashMap::new();
    let mut avance = 0.0;
    let mut advertencias = Vec::new();
    curso.raiz.recorrer(&mut |n| {
        let t = act.traza.get(&n.id).cloned().unwrap_or_default();
        let nota_de = |c: &Corrida| c.traza.get(&n.id).and_then(|t| t.nota).unwrap_or(0.0);
        let peso = pesos.get(&n.id).copied().unwrap_or_default();
        nodos.insert(
            n.id,
            InformeNodo {
                actual: t.nota,
                minimo: nota_de(&pes),
                maximo: nota_de(&opt),
                peso,
                eliminado: t.eliminado,
                candado: n.candado.as_ref().map(|_| match t.candado_roto {
                    Some(true) => EstadoCandado::Roto,
                    Some(false) => EstadoCandado::Cumple,
                    None => EstadoCandado::Pendiente,
                }),
            },
        );
        match &n.tipo {
            Tipo::Nota { valor } => {
                if let Some(v) = valor {
                    avance += peso.final_;
                    if *v < 0.0 || *v > curso.escala_max {
                        advertencias.push(format!(
                            "«{}» tiene {} pero la escala va de 0 a {}",
                            n.nombre, v, curso.escala_max
                        ));
                    }
                }
            }
            Tipo::Grupo { hijos, eliminar_menores } => {
                if hijos.is_empty() {
                    if n.id != ID_RAIZ {
                        advertencias.push(format!("«{}» no tiene evaluaciones", n.nombre));
                    }
                    return;
                }
                let suma: f64 = hijos.iter().filter_map(|h| pesos.get(&h.id)).map(|p| p.local).sum();
                if (suma - 1.0).abs() > 1e-6 {
                    advertencias.push(format!(
                        "Los pesos dentro de «{}» suman {:.1}% (se normalizan al promediar)",
                        n.nombre,
                        suma * 100.0
                    ));
                }
                if *eliminar_menores >= hijos.len() {
                    advertencias.push(format!(
                        "«{}» elimina {} notas pero solo tiene {}",
                        n.nombre,
                        eliminar_menores,
                        hijos.len()
                    ));
                }
            }
        }
    });

    let minimo = pes.final_.unwrap_or(0.0);
    let maximo = opt.final_.unwrap_or(0.0);
    let apr = curso.aprobatoria;
    let estado = if minimo >= apr - EPS {
        Estado::Aprobado
    } else if maximo < apr - EPS {
        Estado::Desaprobado
    } else {
        Estado::EnJuego
    };
    // La nota final es monótona en x (promedios, mínimos y candados lo son),
    // así que basta una bisección.
    let necesaria = (estado == Estado::EnJuego).then(|| {
        let (mut lo, mut hi) = (0.0, curso.escala_max);
        for _ in 0..60 {
            let mid = (lo + hi) / 2.0;
            if correr(curso, Escenario::Uniforme(mid), &pesos).final_.unwrap_or(0.0) >= apr - EPS {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        hi
    });

    Informe {
        nodos,
        actual: act.final_,
        minimo,
        maximo,
        estado,
        necesaria,
        avance,
        advertencias,
    }
}

/// Promedio ponderado por créditos de varios cursos (ignora los que no tienen nota).
pub fn ponderado<'a>(cursos: impl IntoIterator<Item = &'a Curso>) -> Option<f64> {
    let (mut s, mut w) = (0.0, 0.0);
    for c in cursos {
        if let Some(n) = informe(c).actual {
            s += n * c.creditos;
            w += c.creditos;
        }
    }
    (w > EPS).then(|| s / w)
}
