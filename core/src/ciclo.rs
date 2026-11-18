//! Ciclos académicos: agrupación de cursos y promedio ponderado por créditos.
//!
//! Promedio ponderado = Σ(nota × créditos) / Σ créditos

use crate::modelo::Curso;
use crate::motor::{informe, Estado};

const EPS: f64 = 1e-9;

/// Un curso dentro del resumen de su ciclo.
#[derive(Clone, Debug, PartialEq)]
pub struct NotaCiclo {
    pub id: String,
    pub nombre: String,
    pub creditos: f64,
    /// Nota actual (solo con lo rendido); `None` si aún no tiene notas.
    pub nota: Option<f64>,
    /// Peor y mejor nota final posibles.
    pub minimo: f64,
    pub maximo: f64,
    pub estado: Estado,
    pub decimales: u8,
    pub escala_max: f64,
}

impl NotaCiclo {
    /// Aporte del curso al numerador: nota × créditos.
    pub fn puntos(&self) -> Option<f64> {
        self.nota.map(|n| n * self.creditos)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResumenCiclo {
    pub cursos: Vec<NotaCiclo>,
    /// Σ créditos de todos los cursos del ciclo.
    pub creditos: f64,
    /// Σ créditos de los cursos que ya tienen nota (denominador del ponderado).
    pub creditos_con_nota: f64,
    /// Σ(nota × créditos) de los cursos con nota (numerador del ponderado).
    pub puntos: f64,
    /// Σ(nota × créditos) / Σ créditos, solo con los cursos que tienen nota.
    pub ponderado: Option<f64>,
    /// Ponderado con todos los cursos en su peor y su mejor caso.
    pub ponderado_minimo: Option<f64>,
    pub ponderado_maximo: Option<f64>,
    /// Créditos de cursos ya asegurados (aprobados pase lo que pase).
    pub creditos_aprobados: f64,
    pub creditos_jalados: f64,
}

/// Resumen y promedio ponderado de un conjunto de cursos.
/// Los cursos con 0 créditos aparecen en la lista pero no pesan en el promedio.
pub fn resumen_ciclo<'a>(cursos: impl IntoIterator<Item = &'a Curso>) -> ResumenCiclo {
    let mut r = ResumenCiclo {
        cursos: vec![],
        creditos: 0.0,
        creditos_con_nota: 0.0,
        puntos: 0.0,
        ponderado: None,
        ponderado_minimo: None,
        ponderado_maximo: None,
        creditos_aprobados: 0.0,
        creditos_jalados: 0.0,
    };
    let (mut pmin, mut pmax) = (0.0, 0.0);
    for c in cursos {
        let i = informe(c);
        let fila = NotaCiclo {
            id: c.id.clone(),
            nombre: c.nombre.clone(),
            creditos: c.creditos,
            nota: i.actual,
            minimo: i.minimo,
            maximo: i.maximo,
            estado: i.estado,
            decimales: c.decimales,
            escala_max: c.escala_max,
        };
        r.creditos += c.creditos;
        pmin += i.minimo * c.creditos;
        pmax += i.maximo * c.creditos;
        if let Some(p) = fila.puntos() {
            r.puntos += p;
            r.creditos_con_nota += c.creditos;
        }
        match i.estado {
            Estado::Aprobado => r.creditos_aprobados += c.creditos,
            Estado::Desaprobado => r.creditos_jalados += c.creditos,
            Estado::EnJuego => {}
        }
        r.cursos.push(fila);
    }
    if r.creditos_con_nota > EPS {
        r.ponderado = Some(r.puntos / r.creditos_con_nota);
    }
    if r.creditos > EPS {
        r.ponderado_minimo = Some(pmin / r.creditos);
        r.ponderado_maximo = Some(pmax / r.creditos);
    }
    r
}

/// Ciclos distintos, del más reciente al más antiguo ("" = sin ciclo, al final).
pub fn ciclos(cursos: &[Curso]) -> Vec<String> {
    let mut v: Vec<String> = cursos.iter().map(|c| c.periodo.clone()).collect();
    ordenar_ciclos(&mut v);
    v
}

/// Ordena del más reciente al más antiguo, deja "" al final y quita repetidos.
pub fn ordenar_ciclos(v: &mut Vec<String>) {
    v.sort_by(|a, b| match (a.is_empty(), b.is_empty()) {
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        _ => b.cmp(a),
    });
    v.dedup();
}

/// `"2026-2"` → `(2026, 2)`.
fn anio_y_numero(ciclo: &str) -> Option<(i32, u32)> {
    let (a, n) = ciclo.trim().split_once('-')?;
    Some((a.trim().parse().ok()?, n.trim().parse().ok()?))
}

/// Nombre sugerido para un ciclo nuevo: el que sigue al más reciente con forma
/// `AAAA-N` (dos ciclos por año: `2026-1` → `2026-2` → `2027-1`). Si no hay
/// ninguno así, usa `actual`. Nunca repite un nombre existente.
pub fn ciclo_siguiente(existentes: &[String], actual: &str) -> String {
    let base = existentes
        .iter()
        .filter_map(|c| anio_y_numero(c))
        .max()
        .map(|(a, n)| if n >= 2 { format!("{}-1", a + 1) } else { format!("{a}-{}", n + 1) })
        .unwrap_or_else(|| actual.to_string());
    let mut nombre = base.clone();
    let mut i = 2;
    while existentes.iter().any(|c| *c == nombre) {
        nombre = format!("{base} ({i})");
        i += 1;
    }
    nombre
}

/// Cursos de un ciclo, en el orden en que están guardados.
pub fn cursos_del_ciclo<'a>(cursos: &'a [Curso], ciclo: &'a str) -> impl Iterator<Item = &'a Curso> + 'a {
    cursos.iter().filter(move |c| c.periodo == ciclo)
}
