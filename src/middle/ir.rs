//! IR do middle-end: AST com tipos resolvidos, categorias CEI e erros anotados.

use serde::Serialize;

use crate::ast::{Comparator, Span, Type, Value};

pub const VERSAO_IR: u32 = 1;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ContratoIR {
    pub versao_ir: u32,
    pub nome: String,
    /// O primeiro estado é o inicial.
    pub estados: Vec<EstadoIR>,
    pub variaveis: Vec<VariavelIR>,
    pub funcoes: Vec<FuncaoIR>,
    /// Catálogo enxuto de erros customizados: apenas os efetivamente usados.
    pub erros: Vec<ErroIR>,
    pub trava_reentrancia: bool,
    pub passes_aplicados: Vec<String>,
    pub origem: Span,
}

impl ContratoIR {
    pub fn passou(&self, passe: &str) -> bool {
        self.passes_aplicados.iter().any(|p| p == passe)
    }

    pub fn variavel(&self, nome: &str) -> Option<&VariavelIR> {
        self.variaveis.iter().find(|v| v.nome == nome)
    }

    pub fn possui_estado(&self, nome: &str) -> bool {
        self.estados.iter().any(|e| e.nome == nome)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EstadoIR {
    pub nome: String,
    pub origem: Span,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VariavelIR {
    pub nome: String,
    pub tipo: Type,
    pub inicial: Operando,
    pub origem: Span,
}

/// Valor acompanhado do tipo resolvido.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Operando {
    pub valor: Value,
    pub tipo: Type,
}

impl Operando {
    pub fn referencia(&self) -> Option<&str> {
        match &self.valor {
            Value::Referencia(nome) => Some(nome),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Mutabilidade {
    /// Não lê nem escreve estado (`pure`).
    Pura,
    /// Apenas lê estado (`view`).
    Leitura,
    /// Escreve estado ou interage com outras contas.
    Escrita,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FuncaoIR {
    pub nome: String,
    pub quando: Option<String>,
    pub retorna: Option<Type>,
    pub mutabilidade: Mutabilidade,
    pub protegida_contra_reentrancia: bool,
    pub instrucoes: Vec<Instrucao>,
    pub origem: Span,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum Categoria {
    Verificacao,
    Efeito,
    Interacao,
    Retorno,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Instrucao {
    pub operacao: Operacao,
    pub categoria: Categoria,
    /// Inserida pelo compilador, sem correspondência direta no fonte.
    pub injetada: bool,
    pub erro: Option<ErroLocal>,
    pub origem: Span,
}

impl Instrucao {
    pub fn nova(operacao: Operacao, origem: Span) -> Self {
        Self {
            categoria: operacao.categoria(),
            operacao,
            injetada: false,
            erro: None,
            origem,
        }
    }

    pub fn injetada(operacao: Operacao, origem: Span) -> Self {
        Self {
            injetada: true,
            ..Self::nova(operacao, origem)
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum Operacao {
    Exigir { condicao: CondicaoIR },
    ExigirEstado { estado: String },
    Atribuir { destino: String, valor: Operando },
    Somar { destino: String, valor: Operando },
    Subtrair { destino: String, valor: Operando },
    MudarEstado { estado: String },
    Transferir { valor: Operando, destino: Operando },
    Retornar { valor: Operando },
}

impl Operacao {
    pub fn categoria(&self) -> Categoria {
        match self {
            Operacao::Exigir { .. } | Operacao::ExigirEstado { .. } => Categoria::Verificacao,
            Operacao::Atribuir { .. }
            | Operacao::Somar { .. }
            | Operacao::Subtrair { .. }
            | Operacao::MudarEstado { .. } => Categoria::Efeito,
            Operacao::Transferir { .. } => Categoria::Interacao,
            Operacao::Retornar { .. } => Categoria::Retorno,
        }
    }

    /// Variável de estado escrita pela operação, se houver.
    pub fn escreve(&self) -> Option<&str> {
        match self {
            Operacao::Atribuir { destino, .. }
            | Operacao::Somar { destino, .. }
            | Operacao::Subtrair { destino, .. } => Some(destino),
            _ => None,
        }
    }

    /// Operandos lidos pela operação.
    pub fn operandos(&self) -> Vec<&Operando> {
        match self {
            Operacao::Exigir { condicao } => condicao.operandos(),
            Operacao::Atribuir { valor, .. }
            | Operacao::Somar { valor, .. }
            | Operacao::Subtrair { valor, .. }
            | Operacao::Retornar { valor } => vec![valor],
            Operacao::Transferir { valor, destino } => vec![valor, destino],
            Operacao::ExigirEstado { .. } | Operacao::MudarEstado { .. } => vec![],
        }
    }

    /// Indica se a operação lê o armazenamento do contrato.
    pub fn le_estado(&self) -> bool {
        matches!(
            self,
            Operacao::ExigirEstado { .. } | Operacao::Somar { .. } | Operacao::Subtrair { .. }
        ) || self.operandos().iter().any(|o| o.referencia().is_some())
    }

    pub fn descricao(&self) -> String {
        match self {
            Operacao::Exigir { .. } => "exigir".into(),
            Operacao::ExigirEstado { estado } => format!("exigir estado {estado}"),
            Operacao::Atribuir { destino, .. } => format!("definir {destino}"),
            Operacao::Somar { destino, .. } => format!("somar em {destino}"),
            Operacao::Subtrair { destino, .. } => format!("subtrair de {destino}"),
            Operacao::MudarEstado { estado } => format!("mudar para {estado}"),
            Operacao::Transferir { .. } => "transferir".into(),
            Operacao::Retornar { .. } => "retornar".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "forma", rename_all = "snake_case")]
pub enum CondicaoIR {
    Valor {
        valor: Operando,
    },
    Comparacao {
        esquerda: Operando,
        operador: Comparator,
        direita: Operando,
    },
}

impl CondicaoIR {
    pub fn operandos(&self) -> Vec<&Operando> {
        match self {
            CondicaoIR::Valor { valor } => vec![valor],
            CondicaoIR::Comparacao {
                esquerda, direita, ..
            } => vec![esquerda, direita],
        }
    }
}

/// Classes de erro do tratamento unificado; o back-end as mapeia para custom errors.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TipoErro {
    EstadoInvalido,
    CondicaoNaoAtendida,
    SaldoInsuficiente,
    TransferenciaFalhou,
    ChamadaReentrante,
}

/// Ponto de reversão: classe do erro e código único no contrato.
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct ErroLocal {
    pub tipo: TipoErro,
    pub codigo: u32,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TipoParametro {
    Numero,
    Endereco,
    Estado,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ParametroErro {
    pub nome: &'static str,
    pub tipo: TipoParametro,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ErroIR {
    pub tipo: TipoErro,
    pub parametros: Vec<ParametroErro>,
    pub descricao: &'static str,
}
