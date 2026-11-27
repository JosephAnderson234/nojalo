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
