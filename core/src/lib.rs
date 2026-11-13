//! Motor de calificaciones de Nojalo.
//!
//! Un [`Curso`] es un árbol: los [`Nodo`]s son notas o grupos, cada uno con un
//! [`Peso`] (automático, % del grupo o % de la nota final), un [`Candado`]
//! opcional (nota mínima) y, en los grupos, cuántas notas bajas se eliminan.

mod modelo;
mod motor;

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
}
