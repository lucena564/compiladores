//! Passe de tratamento unificado de erros: injeta verificações de saldo antes
//! de `subtrair`, associa cada ponto de reversão a um erro customizado com
//! código único e monta o catálogo enxuto de erros do contrato.

use std::collections::BTreeSet;

use super::super::ir::*;
use crate::{
    ast::{Comparator, Type, Value},
    middle::Diagnostico,
};

pub fn run(mut ir: ContratoIR, _diagnosticos: &mut Vec<Diagnostico>) -> ContratoIR {
    for funcao in &mut ir.funcoes {
        inject_balance_checks(funcao);
    }

    let mut codigo = 0;
    let mut usados = BTreeSet::new();
    for instrucao in ir.funcoes.iter_mut().flat_map(|f| &mut f.instrucoes) {
        let tipo = match (&instrucao.operacao, instrucao.erro) {
            (_, Some(erro)) => Some(erro.tipo),
            (Operacao::ExigirEstado { .. }, None) => Some(TipoErro::EstadoInvalido),
            (Operacao::Exigir { .. }, None) => Some(TipoErro::CondicaoNaoAtendida),
            (Operacao::Transferir { .. }, None) => Some(TipoErro::TransferenciaFalhou),
            _ => None,
        };
        if let Some(tipo) = tipo {
            codigo += 1;
            instrucao.erro = Some(ErroLocal { tipo, codigo });
            usados.insert(tipo);
        }
    }
    if ir.trava_reentrancia {
        usados.insert(TipoErro::ChamadaReentrante);
    }

    ir.erros = usados.into_iter().map(declaration).collect();
    ir
}

/// Garante `destino >= valor` antes de cada `subtrair`, evitando underflow com
/// um erro descritivo. A verificação sobe até o bloco de verificações quando
/// nenhum efeito intermediário altera as variáveis envolvidas, preservando a
/// forma CEI; é omitida se uma verificação equivalente já existir.
fn inject_balance_checks(funcao: &mut FuncaoIR) {
    let mut n = 0;
    while n < funcao.instrucoes.len() {
        let Operacao::Subtrair { destino, valor } = &funcao.instrucoes[n].operacao else {
            n += 1;
            continue;
        };
        let condicao = CondicaoIR::Comparacao {
            esquerda: Operando {
                valor: Value::Referencia(destino.clone()),
                tipo: Type::Numero,
            },
            operador: Comparator::MaiorOuIgual,
            direita: valor.clone(),
        };
        let lidas: Vec<&str> = [Some(destino.as_str()), valor.referencia()]
            .into_iter()
            .flatten()
            .collect();

        let anteriores = &funcao.instrucoes[..n];
        let altera = |i: &Instrucao| i.operacao.escreve().is_some_and(|v| lidas.contains(&v));
        let redundante = anteriores.iter().rev().take_while(|i| !altera(i)).any(
            |i| matches!(&i.operacao, Operacao::Exigir { condicao: c } if equivalent(c, &condicao)),
        );
        let alvo = n - anteriores
            .iter()
            .rev()
            .take_while(|i| !altera(i) && i.categoria != Categoria::Verificacao)
            .count();

        if redundante {
            n += 1;
            continue;
        }

        let mut verificacao = Instrucao::injetada(
            Operacao::Exigir { condicao },
            funcao.instrucoes[n].origem.clone(),
        );
        verificacao.erro = Some(ErroLocal {
            tipo: TipoErro::SaldoInsuficiente,
            codigo: 0,
        });
        funcao.instrucoes.insert(alvo, verificacao);
        n += 2;
    }
}

/// `a >= b` equivale a `b <= a`.
fn equivalent(existente: &CondicaoIR, injetada: &CondicaoIR) -> bool {
    if existente == injetada {
        return true;
    }
    match (existente, injetada) {
        (
            CondicaoIR::Comparacao {
                esquerda: e1,
                operador: Comparator::MenorOuIgual,
                direita: d1,
            },
            CondicaoIR::Comparacao {
                esquerda: e2,
                operador: Comparator::MaiorOuIgual,
                direita: d2,
            },
        ) => e1 == d2 && d1 == e2,
        _ => false,
    }
}

fn declaration(tipo: TipoErro) -> ErroIR {
    use TipoParametro::*;
    let p = |nome, tipo| ParametroErro { nome, tipo };
    let (parametros, descricao) = match tipo {
        TipoErro::EstadoInvalido => (
            vec![p("esperado", Estado), p("atual", Estado)],
            "função chamada fora do estado exigido por 'quando'",
        ),
        TipoErro::CondicaoNaoAtendida => (
            vec![p("codigo", Numero)],
            "condição de 'exigir' falsa; o código identifica o passo",
        ),
        TipoErro::SaldoInsuficiente => (
            vec![p("disponivel", Numero), p("necessario", Numero)],
            "subtração resultaria em valor negativo",
        ),
        TipoErro::TransferenciaFalhou => (
            vec![p("destino", Endereco), p("valor", Numero)],
            "a chamada de transferência retornou falha",
        ),
        TipoErro::ChamadaReentrante => (
            vec![],
            "função protegida chamada durante sua própria execução",
        ),
    };
    ErroIR {
        tipo,
        parametros,
        descricao,
    }
}
