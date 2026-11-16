//! Passe de máquina de estados: valida transições e alcançabilidade e injeta
//! a guarda `exigir estado` no início das funções com `quando:`.

use std::collections::{HashSet, VecDeque};

use super::{super::ir::*, MAQUINA_DE_ESTADOS};
use crate::middle::Diagnostico;

pub fn run(mut ir: ContratoIR, diagnosticos: &mut Vec<Diagnostico>) -> ContratoIR {
    let mut diag = |origem: &crate::ast::Span, mensagem: String, erro: bool| {
        diagnosticos.push(if erro {
            Diagnostico::erro(MAQUINA_DE_ESTADOS, origem, mensagem)
        } else {
            Diagnostico::aviso(MAQUINA_DE_ESTADOS, origem, mensagem)
        });
    };

    let mut arestas: Vec<(String, String)> = Vec::new();

    for funcao in &ir.funcoes {
        let transicoes: Vec<&Instrucao> = funcao
            .instrucoes
            .iter()
            .filter(|i| matches!(i.operacao, Operacao::MudarEstado { .. }))
            .collect();

        if transicoes.len() > 1 {
            diag(
                &transicoes[1].origem,
                format!(
                    "função '{}' muda de estado mais de uma vez; apenas uma transição por função é permitida",
                    funcao.nome
                ),
                true,
            );
        }

        for transicao in transicoes {
            let Operacao::MudarEstado { estado: destino } = &transicao.operacao else {
                unreachable!()
            };
            match &funcao.quando {
                Some(origem) => {
                    if origem == destino {
                        diag(
                            &transicao.origem,
                            format!(
                                "transição redundante: '{}' já está no estado '{}'",
                                funcao.nome, destino
                            ),
                            false,
                        );
                    }
                    arestas.push((origem.clone(), destino.clone()));
                }
                // Sem guarda, a função pode ser chamada em qualquer estado.
                None => {
                    for origem in &ir.estados {
                        arestas.push((origem.nome.clone(), destino.clone()));
                    }
                }
            }
        }
    }

    if let Some(inicial) = ir.estados.first() {
        let mut alcancaveis = HashSet::from([inicial.nome.clone()]);
        let mut fila = VecDeque::from([inicial.nome.clone()]);
        while let Some(atual) = fila.pop_front() {
            for (_, destino) in arestas.iter().filter(|(o, _)| *o == atual) {
                if alcancaveis.insert(destino.clone()) {
                    fila.push_back(destino.clone());
                }
            }
        }

        for estado in &ir.estados {
            if alcancaveis.contains(&estado.nome) {
                continue;
            }
            let afetadas: Vec<&str> = ir
                .funcoes
                .iter()
                .filter(|f| f.quando.as_ref() == Some(&estado.nome))
                .map(|f| f.nome.as_str())
                .collect();
            let complemento = if afetadas.is_empty() {
                String::new()
            } else {
                format!(
                    "; funções que nunca poderão executar: {}",
                    afetadas.join(", ")
                )
            };
            diag(
                &estado.origem,
                format!(
                    "estado '{}' é inalcançável a partir do estado inicial '{}'{}",
                    estado.nome, inicial.nome, complemento
                ),
                true,
            );
        }
    }

    for funcao in &mut ir.funcoes {
        if let Some(estado) = &funcao.quando {
            let guarda = Instrucao::injetada(
                Operacao::ExigirEstado {
                    estado: estado.clone(),
                },
                funcao.origem.clone(),
            );
            funcao.instrucoes.insert(0, guarda);
        }
    }

    ir
}
