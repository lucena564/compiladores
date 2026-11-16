use std::collections::HashSet;

use crate::{
    ast::{Command, Comparator, Condition, Contract, Function, Span, Type, Value},
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

    if let Some(estado) = &function.quando {
        if !symbols.contains_state(estado) {
            errors.push(IRError::from_span(
                &function.origem,
                format!("estado '{}' não foi declarado", estado),
            ));
        }
    }

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

            Command::Somar { destino, valor }
            | Command::Subtrair { destino, valor } => {
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
                            "não é possível alterar '{}' aritmeticamente porque ela não é do tipo numero",
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

            Command::Exigir { condicao } => {
                if let Err(error) =
                    verify_condition(condicao, symbols, &action.origem)
                {
                    errors.push(error);
                }
            }

            Command::Transferir { valor, destino } => {
                for (value, expected) in
                    [(valor, Type::Numero), (destino, Type::Endereco)]
                {
                    if let Err(error) = verify_value_type(
                        value,
                        &expected,
                        symbols,
                        &action.origem,
                    ) {
                        errors.push(error);
                    }
                }
            }

            Command::MudarEstado { estado } => {
                if !symbols.contains_state(estado) {
                    errors.push(IRError::from_span(
                        &action.origem,
                        format!("estado '{}' não foi declarado", estado),
                    ));
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

fn verify_condition(
    condicao: &Condition,
    symbols: &SymbolTable,
    span: &Span,
) -> Result<(), IRError> {
    match condicao {
        Condition::Valor { valor } => {
            verify_value_type(valor, &Type::Logico, symbols, span)
        }

        Condition::Comparacao {
            esquerda,
            operador,
            direita,
        } => {
            match operador {
                Comparator::Igual | Comparator::Diferente => {
                    let left_type = value_type(esquerda, symbols, span)?;
                    let right_type = value_type(direita, symbols, span)?;
                    // Um literal de texto comparado a um endereco é lido como endereco.
                    let expected = if right_type == Type::Endereco {
                        right_type
                    } else {
                        left_type
                    };
                    verify_value_type(esquerda, &expected, symbols, span)?;
                    verify_value_type(direita, &expected, symbols, span)
                }
                _ => {
                    verify_value_type(esquerda, &Type::Numero, symbols, span)?;
                    verify_value_type(direita, &Type::Numero, symbols, span)
                }
            }
        }
    }
}

/// Tipo de um valor; referências são resolvidas pela tabela de símbolos.
pub fn value_type(
    value: &Value,
    symbols: &SymbolTable,
    span: &Span,
) -> Result<Type, IRError> {
    match value {
        Value::Numero(_) => Ok(Type::Numero),
        Value::Texto(_) => Ok(Type::Texto),
        Value::Logico(_) => Ok(Type::Logico),
        Value::Referencia(name) => symbols.lookup(name).cloned().ok_or_else(|| {
            IRError::from_span(
                span,
                format!("variável '{}' não foi declarada", name),
            )
        }),
    }
}

fn verify_value_type(
    value: &Value,
    expected_type: &Type,
    symbols: &SymbolTable,
    span: &Span,
) -> Result<(), IRError> {
    // Literais de endereco chegam como texto e precisam ter o formato EVM.
    if let (Value::Texto(text), Type::Endereco) = (value, expected_type) {
        if !is_address_literal(text) {
            return Err(IRError::from_span(
                span,
                format!(
                    "endereço inválido \"{}\": esperado 0x seguido de 40 dígitos hexadecimais",
                    text
                ),
            ));
        }
        return Ok(());
    }

    let actual_type = value_type(value, symbols, span)?;

    verify_compatible_types(&actual_type, expected_type, span)
}

pub fn is_address_literal(text: &str) -> bool {
    text.len() == 42
        && text.starts_with("0x")
        && text[2..].chars().all(|c| c.is_ascii_hexdigit())
}

fn verify_compatible_types(
    actual_type: &Type,
    expected_type: &Type,
    span: &Span,
) -> Result<(), IRError> {
    if actual_type == expected_type {
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
