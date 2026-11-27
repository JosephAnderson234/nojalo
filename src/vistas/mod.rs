//! Pantallas de la app. Cada una lee el estado con `expect_context::<Global>()`.

mod barra;
mod bienvenida;
mod ciclo;
mod curso;

pub use barra::Barra;
pub use bienvenida::Bienvenida;
pub use ciclo::VistaCiclo;
pub use curso::Editor;
