use serde::Serialize;

/// Posição em bytes UTF-8 no texto original; fim exclusivo.
#[derive(Debug, Serialize, PartialEq)]
pub struct Span {
    pub inicio: usize,
    pub fim: usize,
    pub linha: usize,
    pub coluna: usize,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Contract {
    pub versao_ast: u32,
    pub nome: String,
    pub variaveis: Vec<Variable>,
    pub funcoes: Vec<Function>,
    pub origem: Span,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Type {
    Numero,
    Texto,
    Logico,
    Endereco,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Variable {
    pub nome: String,
    pub tipo: Type,
    pub inicial: Value,
    pub origem: Span,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Function {
    pub nome: String,
    pub retorna: Option<Type>,
    pub passos: Vec<Action>,
    pub origem: Span,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Action {
    pub comando: Command,
    pub origem: Span,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(tag = "operacao", rename_all = "snake_case")]
pub enum Command {
    Definir { destino: String, valor: Value },
    Somar { destino: String, valor: Value },
    Retornar { valor: Value },
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(tag = "categoria", content = "conteudo", rename_all = "snake_case")]
pub enum Value {
    /// Preservado como decimal textual: não impõe limite u64 nem valida uint256.
    Numero(String),
    Texto(String),
    Logico(bool),
    Referencia(String),
}
