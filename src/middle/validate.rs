//! VerifyIR do middle-end. Executado após cada passe para localizar regressões:
//! confere invariantes estruturais e as garantias dos passes já aplicados.

use std::collections::HashSet;

use crate::{
    ast::{Comparator, Span, Type, Value},
    verify::is_address_literal,
};

use super::{ir::*, passes};

pub struct Anomalia {
    pub origem: Span,
    pub mensagem: String,
}

pub fn validate(ir: &ContratoIR) -> Result<(), Vec<Anomalia>> {
    let mut v = Validador {
        ir,
        anomalias: Vec::new(),
    };
    v.contrato();
    for funcao in &ir.funcoes {
        v.funcao(funcao);
    }
    if v.anomalias.is_empty() {
        Ok(())
    } else {
        Err(v.anomalias)
    }
}

struct Validador<'a> {
    ir: &'a ContratoIR,
    anomalias: Vec<Anomalia>,
}

impl Validador<'_> {
    fn erro(&mut self, origem: &Span, mensagem: impl Into<String>) {
        self.anomalias.push(Anomalia {
            origem: origem.clone(),
            mensagem: mensagem.into(),
        });
    }

    fn contrato(&mut self) {
        let ir = self.ir;
        if ir.versao_ir != VERSAO_IR {
            self.erro(
                &ir.origem,
                format!("versão de IR inesperada: {}", ir.versao_ir),
            );
        }
        if ir.nome.is_empty() {
            self.erro(&ir.origem, "contrato sem nome");
        }

        let mut nomes = HashSet::new();
        for estado in &ir.estados {
            if !nomes.insert(&estado.nome) {
                self.erro(
                    &estado.origem,
                    format!("estado '{}' duplicado", estado.nome),
                );
            }
        }
        let mut nomes = HashSet::new();
        for variavel in &ir.variaveis {
            if !nomes.insert(&variavel.nome) {
                self.erro(
                    &variavel.origem,
                    format!("variável '{}' duplicada", variavel.nome),
                );
            }
            self.operando(&variavel.inicial, &variavel.origem);
            if variavel.inicial.tipo != variavel.tipo {
                self.erro(
                    &variavel.origem,
                    format!("inicialização de '{}' com tipo divergente", variavel.nome),
                );
            }
        }
        let mut nomes = HashSet::new();
        for funcao in &ir.funcoes {
            if !nomes.insert(&funcao.nome) {
                self.erro(
                    &funcao.origem,
                    format!("função '{}' duplicada", funcao.nome),
                );
            }
        }

        let mut catalogo = HashSet::new();
        for erro in &ir.erros {
            if !catalogo.insert(erro.tipo) {
                self.erro(
                    &ir.origem,
                    format!("erro {:?} duplicado no catálogo", erro.tipo),
                );
            }
        }

        if ir.passou(passes::REENTRANCIA) {
            let esperado = ir.funcoes.iter().any(|f| f.protegida_contra_reentrancia);
            if ir.trava_reentrancia != esperado {
                self.erro(
                    &ir.origem,
                    "trava de reentrância do contrato inconsistente com as funções",
                );
            }
        }

        if ir.passou(passes::TRATAMENTO_DE_ERROS) {
            let mut usados: HashSet<TipoErro> = ir
                .funcoes
                .iter()
                .flat_map(|f| &f.instrucoes)
                .filter_map(|i| i.erro.map(|e| e.tipo))
                .collect();
            if ir.trava_reentrancia {
                usados.insert(TipoErro::ChamadaReentrante);
            }
            if usados != catalogo {
                self.erro(
                    &ir.origem,
                    "catálogo de erros não corresponde aos erros usados",
                );
            }
            let mut codigos = HashSet::new();
            for instrucao in ir.funcoes.iter().flat_map(|f| &f.instrucoes) {
                if let Some(erro) = instrucao.erro {
                    if !codigos.insert(erro.codigo) {
                        self.erro(
                            &instrucao.origem,
                            format!("código de erro {} repetido", erro.codigo),
                        );
                    }
                }
            }
        }
    }

    fn funcao(&mut self, funcao: &FuncaoIR) {
        let ir = self.ir;

        if let Some(estado) = &funcao.quando {
            if !ir.possui_estado(estado) {
                self.erro(&funcao.origem, format!("estado '{estado}' não declarado"));
            }
        }

        for instrucao in &funcao.instrucoes {
            self.instrucao(funcao, instrucao);
        }

        if ir.passou(passes::MAQUINA_DE_ESTADOS) {
            let guardas: Vec<usize> = funcao
                .instrucoes
                .iter()
                .enumerate()
                .filter(|(_, i)| matches!(i.operacao, Operacao::ExigirEstado { .. }))
                .map(|(n, _)| n)
                .collect();
            let esperado = match &funcao.quando {
                Some(estado) => {
                    guardas == [0]
                        && funcao.instrucoes[0].operacao
                            == Operacao::ExigirEstado {
                                estado: estado.clone(),
                            }
                }
                None => guardas.is_empty(),
            };
            if !esperado {
                self.erro(
                    &funcao.origem,
                    format!(
                        "guarda de estado ausente ou fora do início em '{}'",
                        funcao.nome
                    ),
                );
            }
        }

        if ir.passou(passes::CHECKS_EFFECTS_INTERACTIONS) {
            let mut fase = Categoria::Verificacao;
            for (n, instrucao) in funcao.instrucoes.iter().enumerate() {
                if instrucao.categoria == Categoria::Retorno && n + 1 != funcao.instrucoes.len() {
                    self.erro(&instrucao.origem, "retornar não é a última instrução");
                }
                if instrucao.injetada {
                    continue;
                }
                if instrucao.categoria < fase {
                    self.erro(
                        &instrucao.origem,
                        format!("ordem CEI violada em '{}'", funcao.nome),
                    );
                }
                fase = instrucao.categoria;
            }
            if funcao.mutabilidade != passes::cei::mutabilidade(funcao) {
                self.erro(
                    &funcao.origem,
                    format!("mutabilidade de '{}' inconsistente", funcao.nome),
                );
            }
        }

        if ir.passou(passes::REENTRANCIA) {
            let interage = funcao
                .instrucoes
                .iter()
                .any(|i| i.categoria == Categoria::Interacao);
            if funcao.protegida_contra_reentrancia != interage {
                self.erro(
                    &funcao.origem,
                    format!("proteção de reentrância de '{}' inconsistente", funcao.nome),
                );
            }
        }
    }

    fn instrucao(&mut self, funcao: &FuncaoIR, instrucao: &Instrucao) {
        let ir = self.ir;
        let origem = &instrucao.origem;

        if instrucao.categoria != instrucao.operacao.categoria() {
            self.erro(origem, "categoria da instrução diverge da operação");
        }
        if instrucao.injetada && instrucao.categoria != Categoria::Verificacao {
            self.erro(origem, "apenas verificações podem ser injetadas");
        }
        for operando in instrucao.operacao.operandos() {
            self.operando(operando, origem);
        }

        match &instrucao.operacao {
            Operacao::Exigir { condicao } => match condicao {
                CondicaoIR::Valor { valor } => self.tipo(valor, &Type::Logico, origem),
                CondicaoIR::Comparacao {
                    esquerda,
                    operador,
                    direita,
                } => {
                    if esquerda.tipo != direita.tipo {
                        self.erro(origem, "comparação entre tipos diferentes");
                    }
                    if !matches!(operador, Comparator::Igual | Comparator::Diferente) {
                        self.tipo(esquerda, &Type::Numero, origem);
                    }
                }
            },
            Operacao::ExigirEstado { estado } | Operacao::MudarEstado { estado } => {
                if !ir.possui_estado(estado) {
                    self.erro(origem, format!("estado '{estado}' não declarado"));
                }
            }
            Operacao::Atribuir { destino, valor } => match ir.variavel(destino) {
                Some(v) => self.tipo(valor, &v.tipo.clone(), origem),
                None => self.erro(origem, format!("destino '{destino}' não resolvido")),
            },
            Operacao::Somar { destino, valor } | Operacao::Subtrair { destino, valor } => {
                match ir.variavel(destino) {
                    Some(v) if v.tipo == Type::Numero => {}
                    Some(_) => self.erro(origem, format!("destino '{destino}' não é numero")),
                    None => self.erro(origem, format!("destino '{destino}' não resolvido")),
                }
                self.tipo(valor, &Type::Numero, origem);
            }
            Operacao::Transferir { valor, destino } => {
                self.tipo(valor, &Type::Numero, origem);
                self.tipo(destino, &Type::Endereco, origem);
            }
            Operacao::Retornar { valor } => match &funcao.retorna {
                Some(tipo) => self.tipo(valor, tipo, origem),
                None => self.erro(origem, "retornar em função sem tipo de retorno"),
            },
        }

        match instrucao.erro {
            Some(erro) => {
                if !ir.erros.iter().any(|e| e.tipo == erro.tipo) {
                    self.erro(origem, format!("erro {:?} ausente do catálogo", erro.tipo));
                }
            }
            None => {
                let pode_falhar = matches!(
                    instrucao.categoria,
                    Categoria::Verificacao | Categoria::Interacao
                );
                if pode_falhar && ir.passou(passes::TRATAMENTO_DE_ERROS) {
                    self.erro(origem, "instrução que pode reverter sem erro associado");
                }
            }
        }
    }

    fn operando(&mut self, operando: &Operando, origem: &Span) {
        let coerente = match &operando.valor {
            Value::Numero(_) => operando.tipo == Type::Numero,
            Value::Logico(_) => operando.tipo == Type::Logico,
            Value::Texto(texto) => match operando.tipo {
                Type::Texto => true,
                Type::Endereco => is_address_literal(texto),
                _ => false,
            },
            Value::Referencia(nome) => match self.ir.variavel(nome) {
                Some(v) => v.tipo == operando.tipo,
                None => {
                    self.erro(origem, format!("referência '{nome}' não resolvida"));
                    return;
                }
            },
        };
        if !coerente {
            self.erro(
                origem,
                format!(
                    "operando com tipo não resolvido ou incoerente: {:?}",
                    operando.valor
                ),
            );
        }
    }

    fn tipo(&mut self, operando: &Operando, esperado: &Type, origem: &Span) {
        if operando.tipo != *esperado {
            self.erro(
                origem,
                format!("esperado {esperado:?}, encontrado {:?}", operando.tipo),
            );
        }
    }
}
