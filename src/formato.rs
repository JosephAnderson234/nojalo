//! Lectura y formato de números y nombres para mostrar en la UI.

pub fn num(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', ".");
    if t.is_empty() {
        None
    } else {
        t.parse().ok().filter(|x: &f64| x.is_finite())
    }
}

pub fn corto(x: f64) -> String {
    let r = (x * 100.0).round() / 100.0;
    format!("{r}")
}

pub fn fmt(x: f64, dec: u8) -> String {
    format!("{:.*}", dec as usize, x)
}

pub fn pct(x: f64) -> String {
    let p = x * 100.0;
    if (p - p.round()).abs() < 0.05 {
        format!("{p:.0}%")
    } else {
        format!("{p:.1}%")
    }
}

pub fn nombre_ciclo(p: &str) -> String {
    if p.is_empty() {
        "Sin ciclo".into()
    } else {
        p.into()
    }
}

/// Nombre de archivo seguro a partir de un título: «Cálculo I» → `calculo-i`.
pub fn nombre_archivo(titulo: &str, sufijo: &str) -> String {
    let mut s = String::new();
    for ch in titulo.trim().to_lowercase().chars() {
        let ch = match ch {
            'á' | 'à' | 'ä' => 'a',
            'é' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'ï' => 'i',
            'ó' | 'ò' | 'ö' => 'o',
            'ú' | 'ù' | 'ü' => 'u',
            'ñ' => 'n',
            c => c,
        };
        if ch.is_ascii_alphanumeric() {
            s.push(ch);
        } else if !s.ends_with('-') && !s.is_empty() {
            s.push('-');
        }
    }
    let s = s.trim_end_matches('-');
    format!("{}{sufijo}", if s.is_empty() { "nojalo" } else { s })
}
