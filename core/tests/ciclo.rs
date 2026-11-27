//! Promedio ponderado por créditos y orden de ciclos.

mod comun;

use comun::cerca;
use nojalo_core::*;

fn con_nota(id: &str, ciclo: &str, creditos: f64, nota: f64) -> Curso {
    let mut c = Curso::nuevo(id, id, ciclo);
    c.creditos = creditos;
    c.raiz = Nodo::grupo(ID_RAIZ, "Nota final", Peso::Auto, vec![Nodo::nota(1, "única", Peso::Auto, Some(nota))]);
    c.siguiente_id = 2;
    c
}

#[test]
fn ponderado_clasico_por_creditos() {
    // (14·4 + 11·3 + 17·2) / (4 + 3 + 2) = 123 / 9
    let v = [con_nota("a", "c", 4.0, 14.0), con_nota("b", "c", 3.0, 11.0), con_nota("d", "c", 2.0, 17.0)];
    let r = resumen_ciclo(&v);
    assert!(cerca(r.puntos, 123.0));
    assert!(cerca(r.creditos, 9.0));
    assert!(cerca(r.ponderado.unwrap(), 123.0 / 9.0));
    assert!(cerca(r.creditos_aprobados, 9.0));
    assert_eq!(ponderado(&v), r.ponderado);
}

#[test]
fn ponderado_ignora_cursos_sin_nota_y_sin_creditos() {
    let mut sin_nota = Curso::nuevo("s", "s", "c");
    sin_nota.creditos = 5.0;
    sin_nota.agregar(ID_RAIZ, false);
    let v = [con_nota("a", "c", 3.0, 16.0), sin_nota, con_nota("z", "c", 0.0, 2.0)];
    let r = resumen_ciclo(&v);
    assert_eq!(r.cursos.len(), 3);
    assert!(cerca(r.creditos_con_nota, 3.0));
    assert!(cerca(r.ponderado.unwrap(), 16.0));
    // Proyección: el curso sin notas va de 0 a 20.
    assert!(cerca(r.ponderado_minimo.unwrap(), 16.0 * 3.0 / 8.0));
    assert!(cerca(r.ponderado_maximo.unwrap(), (16.0 * 3.0 + 20.0 * 5.0) / 8.0));
    assert_eq!(resumen_ciclo(&[] as &[Curso]).ponderado, None);
}

#[test]
fn ciclos_ordenados_del_mas_reciente() {
    let v = [
        con_nota("a", "2025-2", 1.0, 1.0),
        con_nota("b", "", 1.0, 1.0),
        con_nota("c", "2026-1", 1.0, 1.0),
        con_nota("d", "2025-2", 1.0, 1.0),
    ];
    assert_eq!(ciclos(&v), ["2026-1", "2025-2", ""]);
    assert_eq!(cursos_del_ciclo(&v, "2025-2").count(), 2);
}

#[test]
fn sugiere_el_ciclo_siguiente() {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    assert_eq!(ciclo_siguiente(&s(&["2026-1"]), "x"), "2026-2");
    assert_eq!(ciclo_siguiente(&s(&["2025-1", "2026-2"]), "x"), "2027-1");
    assert_eq!(ciclo_siguiente(&s(&[]), "2026-2"), "2026-2");
    assert_eq!(ciclo_siguiente(&s(&["Verano"]), "2026-2"), "2026-2");
    assert_eq!(ciclo_siguiente(&s(&["Verano", "2026-2"]), "2026-2"), "2027-1");
    assert_eq!(ciclo_siguiente(&s(&["Verano"]), "Verano"), "Verano (2)");
}
