//! Exportar e importar esquemas (estructura sin notas).

use nojalo_core::*;

fn notas(n: &Nodo) -> Vec<Option<f64>> {
    let mut v = vec![];
    n.recorrer(&mut |x| {
        if let Tipo::Nota { valor } = x.tipo {
            v.push(valor)
        }
    });
    v
}

#[test]
fn esquema_no_lleva_notas() {
    let c = Curso::ejemplo("x", "2026-2");
    let e = c.esquema();
    let v = notas(&e.raiz);
    assert_eq!(v.len(), 9);
    assert!(v.iter().all(Option::is_none));
    assert_eq!(e.formato, FORMATO_ESQUEMA);
    assert!(e.validar().is_ok());
}

#[test]
fn esquema_ida_y_vuelta_conserva_reglas() {
    let original = Curso::ejemplo("x", "2026-2");
    let leidos = leer_esquemas(&original.esquema().a_json()).unwrap();
    assert_eq!(leidos.len(), 1);
    let c = Curso::desde_esquema("y", "2027-1", leidos.into_iter().next().unwrap());
    assert_eq!(c.id, "y");
    assert_eq!(c.periodo, "2027-1");
    assert_eq!(c.nombre, original.nombre);
    assert_eq!(c.creditos, original.creditos);
    // Mismos pesos efectivos: la estructura se conserva aunque cambien los ids.
    let pesos = |c: &Curso| {
        let p = resolver_pesos(&c.raiz);
        let mut v = vec![];
        c.raiz.recorrer(&mut |n| v.push((n.nombre.clone(), (p[&n.id].final_ * 1e6).round() as i64)));
        v
    };
    assert_eq!(pesos(&c), pesos(&original));
    // Candado y eliminación de menores sobreviven.
    let (mut candados, mut elimina) = (0, 0);
    c.raiz.recorrer(&mut |n| {
        candados += n.candado.is_some() as usize;
        if let Tipo::Grupo { eliminar_menores, .. } = n.tipo {
            elimina += eliminar_menores;
        }
    });
    assert_eq!((candados, elimina), (1, 1));
    // Sin notas: todo pendiente.
    assert_eq!(informe(&c).actual, None);
}

#[test]
fn aplicar_esquema_renumera_ids_y_permite_editar() {
    let mut e = Curso::ejemplo("x", "p").esquema();
    // Ids repetidos, como en un archivo editado a mano.
    if let Tipo::Grupo { hijos, .. } = &mut e.raiz.tipo {
        for h in hijos.iter_mut() {
            h.id = 77;
        }
    }
    let mut c = Curso::nuevo("y", "Mi curso", "p");
    c.aplicar_esquema(e);
    assert_eq!(c.nombre, "Mi curso");
    assert_eq!(c.raiz.id, ID_RAIZ);
    let mut ids = vec![];
    c.raiz.recorrer(&mut |n| ids.push(n.id));
    let mut unicos = ids.clone();
    unicos.sort();
    unicos.dedup();
    assert_eq!(ids.len(), unicos.len());
    assert_eq!(c.siguiente_id as usize, ids.len());
    let nuevo = c.agregar(ID_RAIZ, false).unwrap();
    assert!(!ids.contains(&nuevo));
}

#[test]
fn paquete_de_ciclo() {
    let a = Curso::ejemplo("a", "2026-2");
    let mut b = Curso::nuevo("b", "Cálculo", "2026-2");
    b.agregar(ID_RAIZ, false);
    let p = paquete("2026-2", [&a, &b]);
    let leidos = leer_esquemas(&p.a_json()).unwrap();
    let nombres: Vec<&str> = leidos.iter().map(|e| e.nombre.as_str()).collect();
    assert_eq!(nombres, [a.nombre.as_str(), "Cálculo"]);
}

#[test]
fn importa_un_curso_completo_sin_sus_notas() {
    let c = Curso::ejemplo("x", "p");
    let e = leer_esquemas(&serde_json::to_string(&c).unwrap()).unwrap().remove(0);
    assert!(notas(&e.raiz).iter().all(Option::is_none));
}

#[test]
fn rechaza_archivos_invalidos() {
    assert!(leer_esquemas("no es json").is_err());
    assert!(leer_esquemas("{}").is_err());
    assert!(leer_esquemas(r#"{"formato":"otra/cosa"}"#).is_err());
    assert!(leer_esquemas(r#"{"formato":"nojalo/paquete","version":1,"esquemas":[]}"#).is_err());

    let base = || Curso::ejemplo("x", "p").esquema();

    let mut e = base();
    e.version = VERSION_FORMATO + 1;
    assert!(leer_esquemas(&e.a_json()).unwrap_err().contains("versión"));

    let mut e = base();
    e.escala_max = 0.0;
    assert!(leer_esquemas(&e.a_json()).is_err());

    let mut e = base();
    e.aprobatoria = 30.0;
    assert!(leer_esquemas(&e.a_json()).is_err());

    let mut e = base();
    e.raiz = Nodo::nota(0, "solo", Peso::Auto, None);
    assert!(leer_esquemas(&e.a_json()).is_err());

    let mut e = base();
    if let Tipo::Grupo { hijos, .. } = &mut e.raiz.tipo {
        hijos[0].peso = Peso::Final(-5.0);
    }
    assert!(leer_esquemas(&e.a_json()).is_err());
}
