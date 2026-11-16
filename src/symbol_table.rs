use std::collections::HashMap;

use crate::ast::{Contract, Type};

#[derive(Debug, PartialEq)]
pub struct SymbolError {
    pub nome: String,
    pub mensagem: String,
}

/// Variáveis e estados vivem em espaços de nomes separados.
#[derive(Debug, Default)]
pub struct SymbolTable {
    variaveis: HashMap<String, Type>,
    estados: Vec<String>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(
        &mut self,
        nome: &str,
        tipo: Type,
    ) -> Result<(), SymbolError> {
        if self.variaveis.contains_key(nome) {
            return Err(SymbolError {
                nome: nome.to_owned(),
                mensagem: format!(
                    "variável '{}' declarada mais de uma vez",
                    nome
                ),
            });
        }

        self.variaveis.insert(nome.to_owned(), tipo);

        Ok(())
    }

    pub fn insert_state(&mut self, nome: &str) -> Result<(), SymbolError> {
        if self.contains_state(nome) {
            return Err(SymbolError {
                nome: nome.to_owned(),
                mensagem: format!("estado '{}' declarado mais de uma vez", nome),
            });
        }

        self.estados.push(nome.to_owned());

        Ok(())
    }

    pub fn lookup(&self, nome: &str) -> Option<&Type> {
        self.variaveis.get(nome)
    }

    pub fn contains(&self, nome: &str) -> bool {
        self.variaveis.contains_key(nome)
    }

    pub fn contains_state(&self, nome: &str) -> bool {
        self.estados.iter().any(|estado| estado == nome)
    }

    /// Estados na ordem de declaração; o primeiro é o inicial.
    pub fn states(&self) -> &[String] {
        &self.estados
    }

    pub fn len(&self) -> usize {
        self.variaveis.len()
    }

    pub fn is_empty(&self) -> bool {
        self.variaveis.is_empty()
    }

    pub fn from_contract(
        contract: &Contract,
    ) -> Result<Self, Vec<SymbolError>> {
        let mut table = Self {
            variaveis: HashMap::with_capacity(contract.variaveis.len()),
            estados: Vec::with_capacity(contract.estados.len()),
        };
        let mut errors = Vec::new();

        for state in &contract.estados {
            if let Err(error) = table.insert_state(&state.nome) {
                errors.push(error);
            }
        }

        for variable in &contract.variaveis {
            if let Err(error) =
                table.insert(&variable.nome, variable.tipo.clone())
            {
                errors.push(error);
            }
        }

        if errors.is_empty() {
            Ok(table)
        } else {
            Err(errors)
        }
    }
}
