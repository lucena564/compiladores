use std::collections::HashMap;

use crate::ast::{Contract, Type};

#[derive(Debug, PartialEq)]
pub struct SymbolError {
    pub nome: String,
    pub mensagem: String,
}

#[derive(Debug, Default)]
pub struct SymbolTable {
    variaveis: HashMap<String, Type>,
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

    pub fn lookup(&self, nome: &str) -> Option<&Type> {
        self.variaveis.get(nome)
    }

    pub fn contains(&self, nome: &str) -> bool {
        self.variaveis.contains_key(nome)
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
        let mut table = Self::new();
        let mut errors = Vec::new();

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