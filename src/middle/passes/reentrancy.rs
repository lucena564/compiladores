//! Passe de reentrância: marca funções com interações externas para receberem
//! trava (mutex) no back-end e alerta sobre valores transferidos já alterados.

use std::collections::HashSet;

use super::{super::ir::*, REENTRANCIA};
use crate::{ast::Value, middle::Diagnostico};

pub fn run(mut ir: ContratoIR, diagnosticos: &mut Vec<Diagnostico>) -> ContratoIR {
    for funcao in &mut ir.funcoes {
        let mut alteradas: HashSet<&str> = HashSet::new();
        let mut interage = false;

        for instrucao in &funcao.instrucoes {
            if let Some(destino) = instrucao.operacao.escreve() {
                alteradas.insert(destino);
            }
            let Operacao::Transferir { valor, .. } = &instrucao.operacao else {
                continue;
            };
            interage = true;

            if let Some(nome) = valor.referencia().filter(|n| alteradas.contains(n)) {
                diagnosticos.push(Diagnostico::aviso(
                    REENTRANCIA,
                    &instrucao.origem,
                    format!(
                        "'{}' transfere '{nome}' depois de alterá-la nesta função; \
                         o valor enviado será o já atualizado",
                        funcao.nome
                    ),
                ));
            }
            if valor.valor == Value::Numero("0".into()) {
                diagnosticos.push(Diagnostico::aviso(
                    REENTRANCIA,
                    &instrucao.origem,
                    format!("'{}' transfere valor zero", funcao.nome),
                ));
            }
        }

        funcao.protegida_contra_reentrancia = interage;
    }

    ir.trava_reentrancia = ir.funcoes.iter().any(|f| f.protegida_contra_reentrancia);
    ir
}
