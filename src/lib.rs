pub mod ast;
pub mod symbol_table;

use ast::*;
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct LanguageParser;

fn span(pair: &Pair<'_, Rule>, original_len: usize) -> Span {
    let s = pair.as_span();
    let (linha, coluna) = s.start_pos().line_col();
    Span {
        inicio: s.start().min(original_len),
        fim: s.end().min(original_len),
        linha,
        coluna,
    }
}

fn kind(pair: Pair<'_, Rule>) -> Type {
    match pair.as_str() {
        "numero" => Type::Numero,
        "texto" => Type::Texto,
        "logico" => Type::Logico,
        "endereco" => Type::Endereco,
        _ => unreachable!("tipo garantido pela gramática"),
    }
}

fn value(pair: Pair<'_, Rule>) -> Value {
    match pair.as_rule() {
        Rule::number => Value::Numero(pair.as_str().to_owned()),
        Rule::boolean => Value::Logico(pair.as_str() == "verdadeiro"),
        Rule::identifier => Value::Referencia(pair.as_str().to_owned()),
        Rule::string => Value::Texto(
            serde_json::from_str(pair.as_str()).expect("escapes garantidos pela gramática"),
        ),
        _ => unreachable!("valor garantido pela gramática"),
    }
}

/// Reconhece apenas sintaxe. Referências e compatibilidade de tipos ficam para a etapa 2.
pub fn parse(source: &str) -> Result<Contract, String> {
    // Aceita a última linha sem quebra, sem deslocar os bytes anteriores.
    let normalized = if source.ends_with('\n') {
        source.to_owned()
    } else {
        format!("{source}\n")
    };
    let mut parsed = LanguageParser::parse(Rule::document, &normalized)
        .map_err(|e| format!("Erro de sintaxe:\n{e}"))?;
    let document = parsed.next().expect("documento reconhecido");
    let mut contract = Contract {
        versao_ast: 1,
        nome: String::new(),
        variaveis: Vec::new(),
        funcoes: Vec::new(),
        origem: span(&document, source.len()),
    };
    for section in document.into_inner() {
        match section.as_rule() {
            Rule::contract => {
                contract.nome = section.into_inner().next().unwrap().as_str().to_owned();
            }
            Rule::variables => {
                for variable in section.into_inner() {
                    let origem = span(&variable, source.len());
                    let mut parts = variable.into_inner();
                    contract.variaveis.push(Variable {
                        nome: parts.next().unwrap().as_str().to_owned(),
                        tipo: kind(parts.next().unwrap()),
                        inicial: value(parts.next().unwrap()),
                        origem,
                    });
                }
            }
            Rule::functions => {
                for function in section.into_inner() {
                    let origem = span(&function, source.len());
                    let mut parts = function.into_inner();
                    let mut function = Function {
                        nome: parts.next().unwrap().as_str().to_owned(),
                        retorna: None,
                        passos: Vec::new(),
                        origem,
                    };
                    for part in parts {
                        if part.as_rule() == Rule::return_type {
                            function.retorna = Some(kind(part.into_inner().next().unwrap()));
                            continue;
                        }
                        let origem = span(&part, source.len());
                        let command = part.into_inner().next().unwrap();
                        let rule = command.as_rule();
                        let mut operands = command.into_inner();
                        let comando = match rule {
                            Rule::assign => Command::Definir {
                                destino: operands.next().unwrap().as_str().to_owned(),
                                valor: value(operands.next().unwrap()),
                            },
                            Rule::add => {
                                let valor = value(operands.next().unwrap());
                                Command::Somar {
                                    destino: operands.next().unwrap().as_str().to_owned(),
                                    valor,
                                }
                            }
                            Rule::return_value => Command::Retornar {
                                valor: value(operands.next().unwrap()),
                            },
                            _ => unreachable!("comando garantido pela gramática"),
                        };
                        function.passos.push(Action { comando, origem });
                    }
                    contract.funcoes.push(function);
                }
            }
            _ => {}
        }
    }
    Ok(contract)
}
