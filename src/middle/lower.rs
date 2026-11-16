//! Rebaixamento da AST para a IR. Pressupõe uma AST aprovada por `verify_ir`.

use crate::{
    ast::{Command, Comparator, Condition, Contract, Span, Type, Value},
    symbol_table::SymbolTable,
    verify::value_type,
};

use super::ir::*;

pub fn lower(contract: &Contract) -> ContratoIR {
    let symbols = SymbolTable::from_contract(contract).expect("símbolos validados pela tipagem");

    let variaveis = contract
        .variaveis
        .iter()
        .map(|v| VariavelIR {
            nome: v.nome.clone(),
            tipo: v.tipo.clone(),
            inicial: operand(&v.inicial, Some(&v.tipo), &symbols, &v.origem),
            origem: v.origem.clone(),
        })
        .collect();

    let funcoes = contract
        .funcoes
        .iter()
        .map(|f| FuncaoIR {
            nome: f.nome.clone(),
            quando: f.quando.clone(),
            retorna: f.retorna.clone(),
            // Conservador até o passe de efeitos calcular o valor real.
            mutabilidade: Mutabilidade::Escrita,
            protegida_contra_reentrancia: false,
            instrucoes: f
                .passos
                .iter()
                .map(|a| {
                    Instrucao::nova(
                        operation(&a.comando, f.retorna.as_ref(), &symbols, &a.origem),
                        a.origem.clone(),
                    )
                })
                .collect(),
            origem: f.origem.clone(),
        })
        .collect();

    ContratoIR {
        versao_ir: VERSAO_IR,
        nome: contract.nome.clone(),
        estados: contract
            .estados
            .iter()
            .map(|e| EstadoIR {
                nome: e.nome.clone(),
                origem: e.origem.clone(),
            })
            .collect(),
        variaveis,
        funcoes,
        erros: Vec::new(),
        trava_reentrancia: false,
        passes_aplicados: Vec::new(),
        origem: contract.origem.clone(),
    }
}

fn operation(
    command: &Command,
    retorna: Option<&Type>,
    symbols: &SymbolTable,
    span: &Span,
) -> Operacao {
    let numero = Some(&Type::Numero);
    match command {
        Command::Definir { destino, valor } => Operacao::Atribuir {
            destino: destino.clone(),
            valor: operand(valor, symbols.lookup(destino), symbols, span),
        },
        Command::Somar { destino, valor } => Operacao::Somar {
            destino: destino.clone(),
            valor: operand(valor, numero, symbols, span),
        },
        Command::Subtrair { destino, valor } => Operacao::Subtrair {
            destino: destino.clone(),
            valor: operand(valor, numero, symbols, span),
        },
        Command::Exigir { condicao } => Operacao::Exigir {
            condicao: condition(condicao, symbols, span),
        },
        Command::Transferir { valor, destino } => Operacao::Transferir {
            valor: operand(valor, numero, symbols, span),
            destino: operand(destino, Some(&Type::Endereco), symbols, span),
        },
        Command::MudarEstado { estado } => Operacao::MudarEstado {
            estado: estado.clone(),
        },
        Command::Retornar { valor } => Operacao::Retornar {
            valor: operand(valor, retorna, symbols, span),
        },
    }
}

fn condition(condicao: &Condition, symbols: &SymbolTable, span: &Span) -> CondicaoIR {
    match condicao {
        Condition::Valor { valor } => CondicaoIR::Valor {
            valor: operand(valor, Some(&Type::Logico), symbols, span),
        },
        Condition::Comparacao {
            esquerda,
            operador,
            direita,
        } => {
            let expected = match operador {
                Comparator::Igual | Comparator::Diferente => {
                    let right = value_type(direita, symbols, span).ok();
                    if right == Some(Type::Endereco) {
                        right
                    } else {
                        value_type(esquerda, symbols, span).ok()
                    }
                }
                _ => Some(Type::Numero),
            };
            CondicaoIR::Comparacao {
                esquerda: operand(esquerda, expected.as_ref(), symbols, span),
                operador: *operador,
                direita: operand(direita, expected.as_ref(), symbols, span),
            }
        }
    }
}

/// Literais de texto em posição de endereco recebem o tipo `endereco`.
fn operand(value: &Value, expected: Option<&Type>, symbols: &SymbolTable, span: &Span) -> Operando {
    let tipo = match (value, expected) {
        (Value::Texto(_), Some(Type::Endereco)) => Type::Endereco,
        _ => value_type(value, symbols, span).expect("referência validada pela tipagem"),
    };
    Operando {
        valor: value.clone(),
        tipo,
    }
}
