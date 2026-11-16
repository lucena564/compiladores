//! Passe Checks-Effects-Interactions: impõe a ordem verificações → efeitos →
//! interações, rejeita passos após `retornar` e calcula a mutabilidade.

use super::{super::ir::*, CHECKS_EFFECTS_INTERACTIONS};
use crate::middle::Diagnostico;

pub fn run(mut ir: ContratoIR, diagnosticos: &mut Vec<Diagnostico>) -> ContratoIR {
    for funcao in &mut ir.funcoes {
        verify_order(funcao, diagnosticos);
        funcao.mutabilidade = mutabilidade(funcao);
    }
    ir
}

fn verify_order(funcao: &FuncaoIR, diagnosticos: &mut Vec<Diagnostico>) {
    let mut ultimo_efeito: Option<&Instrucao> = None;
    let mut primeira_interacao: Option<&Instrucao> = None;
    let mut retorno: Option<&Instrucao> = None;

    for instrucao in &funcao.instrucoes {
        let mut erro = |mensagem: String| {
            diagnosticos.push(Diagnostico::erro(
                CHECKS_EFFECTS_INTERACTIONS,
                &instrucao.origem,
                mensagem,
            ))
        };

        if let Some(r) = retorno {
            erro(format!(
                "passo '{}' é inalcançável: ocorre após 'retornar' (linha {})",
                instrucao.operacao.descricao(),
                r.origem.linha
            ));
            continue;
        }

        match instrucao.categoria {
            Categoria::Verificacao => {
                if let Some(anterior) = primeira_interacao.or(ultimo_efeito) {
                    erro(format!(
                        "violação de Checks-Effects-Interactions em '{}': a verificação deve vir antes de '{}' (linha {})",
                        funcao.nome,
                        anterior.operacao.descricao(),
                        anterior.origem.linha
                    ));
                }
            }
            Categoria::Efeito => {
                if let Some(interacao) = primeira_interacao {
                    erro(format!(
                        "violação de Checks-Effects-Interactions em '{}': o efeito '{}' ocorre após a interação externa da linha {}; \
                         uma chamada reentrante observaria o estado antigo. Altere o estado antes de interagir",
                        funcao.nome,
                        instrucao.operacao.descricao(),
                        interacao.origem.linha
                    ));
                }
                ultimo_efeito = Some(instrucao);
            }
            Categoria::Interacao => {
                primeira_interacao.get_or_insert(instrucao);
            }
            Categoria::Retorno => retorno = Some(instrucao),
        }
    }
}

pub fn mutabilidade(funcao: &FuncaoIR) -> Mutabilidade {
    let instrucoes = &funcao.instrucoes;
    if instrucoes
        .iter()
        .any(|i| matches!(i.categoria, Categoria::Efeito | Categoria::Interacao))
    {
        Mutabilidade::Escrita
    } else if instrucoes.iter().any(|i| i.operacao.le_estado()) {
        Mutabilidade::Leitura
    } else {
        Mutabilidade::Pura
    }
}
