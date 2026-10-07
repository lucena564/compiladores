use std::collections::HashSet;

use crate::{
    ast::{Command, Contract, Function, Span, Type, Value},
    symbol_table::SymbolTable,
};

#[derive(Debug, PartialEq)]
pub struct IRError {
    pub linha: usize,
    pub coluna: usize,
    pub mensagem: String,
}

impl IRError {
    fn from_span(span: &Span, mensagem: impl Into<String>) -> Self {
        Self {
            linha: span.linha,
            coluna: span.coluna,
            mensagem: mensagem.into(),
        }
    }
}

pub fn verify_ir(contract: &Contract) -> Result<(), Vec<IRError>> {
    let mut errors = Vec::new();

    // Verificação básica do contrato.
    if contract.nome.is_empty() {
        errors.push(IRError::from_span(
            &contract.origem,
            "contrato deve possuir um nome",
        ));
    }

    // Construção da tabela de símbolos.
    let symbols = match SymbolTable::from_contract(contract) {
        Ok(table) => table,
        Err(symbol_errors) => {
            for error in symbol_errors {
                errors.push(IRError::from_span(
                    &contract.origem,
                    error.mensagem,
                ));
            }

            return Err(errors);
        }
    };

    // Verificação das inicializações das variáveis.
    for variable in &contract.variaveis {
        if let Err(error) = verify_value_type(
            &variable.inicial,
            &variable.tipo,
            &symbols,
            &variable.origem,
        ) {
            errors.push(error);
        }
    }

    // Verificação de nomes de funções duplicados.
    let mut function_names = HashSet::new();

    for function in &contract.funcoes {
        if !function_names.insert(&function.nome) {
            errors.push(IRError::from_span(
                &function.origem,
                format!(
                    "função '{}' declarada mais de uma vez",
                    function.nome
                ),
            ));
        }
    }

    // Verificação das funções.
    for function in &contract.funcoes {
        verify_function(function, &symbols, &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn verify_function(
    function: &Function,
    symbols: &SymbolTable,
    errors: &mut Vec<IRError>,
) {
    let mut has_return = false;

    for action in &function.passos {
        match &action.comando {
            Command::Definir { destino, valor } => {
                let Some(destination_type) = symbols.lookup(destino) else {
                    errors.push(IRError::from_span(
                        &action.origem,
                        format!(
                            "variável '{}' não foi declarada",
                            destino
                        ),
                    ));
                    continue;
                };

                if let Err(error) = verify_value_type(
                    valor,
                    destination_type,
                    symbols,
                    &action.origem,
                ) {
                    errors.push(error);
                }
            }

            Command::Somar { destino, valor } => {
                let Some(destination_type) = symbols.lookup(destino) else {
                    errors.push(IRError::from_span(
                        &action.origem,
                        format!(
                            "variável '{}' não foi declarada",
                            destino
                        ),
                    ));
                    continue;
                };

                if *destination_type != Type::Numero {
                    errors.push(IRError::from_span(
                        &action.origem,
                        format!(
                            "não é possível somar em '{}' porque ela não é do tipo numero",
                            destino
                        ),
                    ));
                }

                if let Err(error) = verify_value_type(
                    valor,
                    &Type::Numero,
                    symbols,
                    &action.origem,
                ) {
                    errors.push(error);
                }
            }

            Command::Retornar { valor } => {
                has_return = true;

                match &function.retorna {
                    Some(return_type) => {
                        if let Err(error) = verify_value_type(
                            valor,
                            return_type,
                            symbols,
                            &action.origem,
                        ) {
                            errors.push(error);
                        }
                    }

                    None => {
                        errors.push(IRError::from_span(
                            &action.origem,
                            format!(
                                "função '{}' não possui tipo de retorno",
                                function.nome
                            ),
                        ));
                    }
                }
            }
        }
    }

    // Se a função declara retorno, deve possuir pelo menos um retornar.
    if function.retorna.is_some() && !has_return {
        errors.push(IRError::from_span(
            &function.origem,
            format!(
                "função '{}' declara um retorno, mas não possui comando retornar",
                function.nome
            ),
        ));
    }
}

fn verify_value_type(
    value: &Value,
    expected_type: &Type,
    symbols: &SymbolTable,
    span: &Span,
) -> Result<(), IRError> {
    let actual_type = match value {
        Value::Numero(_) => Type::Numero,

        Value::Texto(_) => Type::Texto,

        Value::Logico(_) => Type::Logico,

        Value::Referencia(name) => {
            let Some(tipo) = symbols.lookup(name) else {
                return Err(IRError::from_span(
                    span,
                    format!(
                        "variável '{}' não foi declarada",
                        name
                    ),
                ));
            };

            return verify_compatible_types(
                tipo,
                expected_type,
                span,
            );
        }
    };

    verify_compatible_types(
        &actual_type,
        expected_type,
        span,
    )
}

fn verify_compatible_types(
    actual_type: &Type,
    expected_type: &Type,
    span: &Span,
) -> Result<(), IRError> {
    // No parser atual, valores do tipo endereco são
    // representados como Value::Texto.
    let compatible = match (actual_type, expected_type) {
        (Type::Texto, Type::Endereco) => true,
        _ => actual_type == expected_type,
    };

    if compatible {
        Ok(())
    } else {
        Err(IRError::from_span(
            span,
            format!(
                "tipo incompatível: esperado {:?}, encontrado {:?}",
                expected_type, actual_type
            ),
        ))
    }
}