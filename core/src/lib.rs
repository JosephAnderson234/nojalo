//! Motor de calificaciones de Nojalo.
//!
//! Un [`Curso`] es un árbol: los [`Nodo`]s son notas o grupos, cada uno con un
//! [`Peso`] (automático, % del grupo o % de la nota final), un [`Candado`]
//! opcional (nota mínima) y, en los grupos, cuántas notas bajas se eliminan.
//!
//! Los [`Esquema`]s exportan e importan la estructura de un curso sin notas, y
//! [`resumen_ciclo`] calcula el promedio ponderado por créditos de un ciclo.

mod ciclo;
mod esquema;
mod modelo;
mod motor;

pub use ciclo::*;
pub use esquema::*;
pub use modelo::*;
pub use motor::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn cerca(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn pesos_mixtos_grupo_y_final() {
        let c = Curso::ejemplo("x", "2026-2");
        let p = resolver_pesos(&c.raiz);
        assert!(cerca(p[&1].final_, 0.40)); // Evaluación continua
        assert!(cerca(p[&3].final_, 0.24)); // Prácticas = 60% de 40%
        assert!(cerca(p[&8].final_, 0.16)); // Labs = 40% de 40%
        assert!(cerca(p[&4].final_, 0.06)); // PC1 = 1/4 de 24%
        assert!(cerca(p[&12].final_, 0.35));
    }

    #[test]
    fn peso_final_dentro_de_un_grupo_se_convierte_a_local() {
        let mut c = Curso::nuevo("x", "c", "p");
        c.raiz = Nodo::grupo(
            0,
            "f",
            Peso::Auto,
            vec![
                Nodo::grupo(
                    1,
                    "g",
                    Peso::Final(40.0),
                    vec![
                        Nodo::nota(2, "a", Peso::Final(10.0), Some(20.0)),
                        Nodo::nota(3, "b", Peso::Auto, Some(10.0)),
                    ],
                ),
                Nodo::nota(4, "c", Peso::Auto, Some(10.0)),
            ],
        );
        let p = resolver_pesos(&c.raiz);
        assert!(cerca(p[&2].local, 0.25));
        assert!(cerca(p[&3].local, 0.75));
        assert!(cerca(p[&4].local, 0.60));
        // g = .25*20 + .75*10 = 12.5 ; final = .4*12.5 + .6*10 = 11
        assert!(cerca(informe(&c).actual.unwrap(), 11.0));
    }

    #[test]
    fn escenarios_del_ejemplo() {
        let c = Curso::ejemplo("x", "2026-2");
        let i = informe(&c);
        // Con PC4 pendiente no se elimina ninguna práctica todavía.
        assert!(cerca(i.nodos[&3].actual.unwrap(), 38.0 / 3.0));
        assert!(!i.nodos[&5].eliminado);
        // (.4*14.2 + .25*12) / .65
        assert!(cerca(i.actual.unwrap(), 8.68 / 0.65));
        // Peor caso: el final en 0 rompe el candado (tope 10), pero 7.8 < 10.
        assert!(cerca(i.minimo, 7.8));
        assert!(cerca(i.maximo, 0.4 * (10.0 + 0.4 * 53.0 / 3.0) + 3.0 + 7.0));
        assert_eq!(i.estado, Estado::EnJuego);
        // El candado del final obliga a sacar al menos 10.5 en lo pendiente.
        assert!((i.necesaria.unwrap() - 10.5).abs() < 1e-6);
        assert_eq!(i.nodos[&12].candado, Some(EstadoCandado::Pendiente));
    }

    #[test]
    fn elimina_la_menor_cuando_el_grupo_esta_completo() {
        let mut c = Curso::ejemplo("x", "2026-2");
        if let Tipo::Nota { valor } = &mut c.buscar_mut(7).unwrap().tipo {
            *valor = Some(12.0);
        }
        let i = informe(&c);
        assert!(i.nodos[&5].eliminado); // PC2 = 8
        assert!(cerca(i.nodos[&3].actual.unwrap(), 14.0));
    }

    #[test]
    fn candado_roto_limita_la_final() {
        let mut c = Curso::ejemplo("x", "2026-2");
        for (id, v) in [(7, 20.0), (11, 20.0), (12, 9.0)] {
            if let Tipo::Nota { valor } = &mut c.buscar_mut(id).unwrap().tipo {
                *valor = Some(v);
            }
        }
        let i = informe(&c);
        assert_eq!(i.nodos[&12].candado, Some(EstadoCandado::Roto));
        assert!(cerca(i.actual.unwrap(), 10.0));
        assert_eq!(i.estado, Estado::Desaprobado);
    }

    #[test]
    fn candado_de_alcance_grupo() {
        let mut c = Curso::nuevo("x", "c", "p");
        c.raiz = Nodo::grupo(
            0,
            "f",
            Peso::Auto,
            vec![
                Nodo::grupo(
                    1,
                    "lab",
                    Peso::Auto,
                    vec![
                        Nodo::nota(2, "informe", Peso::Auto, Some(18.0)),
                        Nodo::nota(3, "sustentación", Peso::Auto, Some(6.0))
                            .con_candado(8.0, 5.0, Alcance::Grupo),
                    ],
                ),
                Nodo::nota(4, "teoría", Peso::Auto, Some(16.0)),
            ],
        );
        let i = informe(&c);
        assert!(cerca(i.nodos[&1].actual.unwrap(), 5.0));
        assert!(cerca(i.actual.unwrap(), 10.5));
    }

    #[test]
    fn editar_arbol() {
        let mut c = Curso::nuevo("x", "c", "p");
        let g = c.agregar(ID_RAIZ, true).unwrap();
        let a = c.agregar(g, false).unwrap();
        let b = c.agregar(g, false).unwrap();
        assert_eq!(c.hijos_ids(g), vec![a, b]);
        assert!(c.mover(b, -1));
        assert_eq!(c.hijos_ids(g), vec![b, a]);
        assert!(!c.mover(b, -1));
        assert!(c.eliminar(a));
        assert_eq!(c.hijos_ids(g), vec![b]);
        assert!(c.agregar(b, false).is_none()); // una nota no tiene hijos
    }

    #[test]
    fn json_ida_y_vuelta() {
        let c = Curso::ejemplo("x", "2026-2");
        let s = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<Curso>(&s).unwrap(), c);
    }

    // ───────── esquemas ─────────

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

    // ───────── ciclos ─────────

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
}
