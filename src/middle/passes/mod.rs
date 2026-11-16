//! Passes do middle-end. Cada passe recebe a IR, acumula diagnósticos e devolve
//! a IR atualizada; o pipeline interrompe a compilação se houver erro.

pub mod cei;
pub mod errors;
pub mod reentrancy;
pub mod state_machine;

use super::{ir::ContratoIR, Diagnostico};

pub const TIPAGEM: &str = "tipagem";
pub const MAQUINA_DE_ESTADOS: &str = "maquina_de_estados";
pub const CHECKS_EFFECTS_INTERACTIONS: &str = "checks_effects_interactions";
pub const REENTRANCIA: &str = "reentrancia";
pub const TRATAMENTO_DE_ERROS: &str = "tratamento_de_erros";

pub type Passe = fn(ContratoIR, &mut Vec<Diagnostico>) -> ContratoIR;

/// Ordem de execução após a tipagem e o rebaixamento.
pub const PIPELINE: [(&str, Passe); 4] = [
    (MAQUINA_DE_ESTADOS, state_machine::run),
    (CHECKS_EFFECTS_INTERACTIONS, cei::run),
    (REENTRANCIA, reentrancy::run),
    (TRATAMENTO_DE_ERROS, errors::run),
];
