//! Utilidades compartidas por los tests de integración.

pub fn cerca(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}
