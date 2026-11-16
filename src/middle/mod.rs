//! Middle-end (etapa 2): análise estática da IR em passes isolados.
//!
//! Pipeline: tipagem (`verify_ir` sobre a AST) → rebaixamento para `ContratoIR`
//! → máquina de estados → Checks-Effects-Interactions → reentrância →
//! tratamento de erros. A IR é validada após cada etapa.

pub mod ir;
pub mod lower;
pub mod passes;
pub mod validate;

use std::fmt;

use serde::Serialize;

use crate::{ast::Contract, ast::Span, verify::verify_ir};
use ir::ContratoIR;

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Severidade {
    Erro,
    Aviso,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Diagnostico {
    pub severidade: Severidade,
    pub passe: String,
    pub linha: usize,
    pub coluna: usize,
    pub mensagem: String,
}

impl Diagnostico {
    pub fn erro(passe: &str, origem: &Span, mensagem: impl Into<String>) -> Self {
        Self::novo(Severidade::Erro, passe, origem, mensagem)
    }

    pub fn aviso(passe: &str, origem: &Span, mensagem: impl Into<String>) -> Self {
        Self::novo(Severidade::Aviso, passe, origem, mensagem)
    }

    fn novo(
        severidade: Severidade,
        passe: &str,
        origem: &Span,
        mensagem: impl Into<String>,
    ) -> Self {
        Self {
            severidade,
            passe: passe.to_owned(),
            linha: origem.linha,
            coluna: origem.coluna,
            mensagem: mensagem.into(),
        }
    }
}

impl fmt::Display for Diagnostico {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severidade = match self.severidade {
            Severidade::Erro => "erro",
            Severidade::Aviso => "aviso",
        };
        write!(
            f,
            "{severidade} [{}] {}:{}: {}",
            self.passe, self.linha, self.coluna, self.mensagem
        )
    }
}

#[derive(Debug)]
pub struct Analise {
    pub ir: ContratoIR,
    pub avisos: Vec<Diagnostico>,
}

/// Executa o middle-end. Em caso de falha, devolve todos os diagnósticos
/// acumulados até o passe que encontrou erro (avisos incluídos).
pub fn analyze(contract: &Contract) -> Result<Analise, Vec<Diagnostico>> {
    verify_ir(contract).map_err(|erros| {
        erros
            .into_iter()
            .map(|e| Diagnostico {
                severidade: Severidade::Erro,
                passe: passes::TIPAGEM.to_owned(),
                linha: e.linha,
                coluna: e.coluna,
                mensagem: e.mensagem,
            })
            .collect::<Vec<_>>()
    })?;

    let mut ir = lower::lower(contract);
    ir.passes_aplicados.push(passes::TIPAGEM.to_owned());
    check_ir(&ir, passes::TIPAGEM)?;

    let mut diagnosticos = Vec::new();
    for (nome, passe) in passes::PIPELINE {
        ir = passe(ir, &mut diagnosticos);
        if diagnosticos
            .iter()
            .any(|d| d.severidade == Severidade::Erro)
        {
            return Err(diagnosticos);
        }
        ir.passes_aplicados.push(nome.to_owned());
        check_ir(&ir, nome)?;
    }

    Ok(Analise {
        ir,
        avisos: diagnosticos,
    })
}

/// Falhas aqui indicam defeito do compilador, não do programa: identificam o
/// passe que introduziu a regressão.
pub fn check_ir(ir: &ContratoIR, depois_de: &str) -> Result<(), Vec<Diagnostico>> {
    validate::validate(ir).map_err(|anomalias| {
        anomalias
            .into_iter()
            .map(|a| {
                Diagnostico::erro(
                    "verificacao_ir",
                    &a.origem,
                    format!(
                        "IR inválida após o passe '{depois_de}' (defeito interno): {}",
                        a.mensagem
                    ),
                )
            })
            .collect()
    })
}
