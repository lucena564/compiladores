# Contrato Simples — etapas 1 e 2

Contrato Simples é uma linguagem declarativa em português, com estrutura visual
semelhante a YAML, destinada à especificação simplificada de contratos. O front-end
em Rust reconhece arquivos `.contrato` e produz uma árvore sintática abstrata
(AST), exportada em JSON. O middle-end analisa essa árvore em passes isolados
(tipagem, máquina de estados, Checks-Effects-Interactions, reentrância e
tratamento de erros) e emite uma IR validada. A geração de Solidity pertence à
etapa 3.

A [fundamentação da etapa 1](docs/etapa-1.md) apresenta o planejamento, o contrato
da AST e a comparação com o relatório de referência. A
[documentação da etapa 2](docs/parte2.md) descreve o middle-end.

```yaml
contrato: Contador
variaveis:
  total: numero = 0
funcoes:
  incrementar:
    passos:
      - somar 1 em total
  consultar:
    retorna: numero
    passos:
      - retornar total
```

## Executar

A execução requer Rust e Cargo na versão estável. Na primeira compilação, Cargo
baixa as dependências; `Cargo.lock` registra as versões utilizadas.

```sh
cargo run -- examples/contador.contrato
cargo run -- examples/cadastro.contrato
cargo run -- --ir examples/cofre.contrato                    # middle-end: IR validada
cargo run -- --ir examples/invalidos/reentrancia.contrato    # erro semântico
cargo test --locked
```

Para salvar a AST após uma execução bem-sucedida:

```sh
mkdir -p output
cargo run -- examples/contador.contrato > output/contador.ast.json
```

A AST é emitida em stdout. Os diagnósticos são emitidos em stderr, com código
de saída 1 em caso de falha.
O comando com redirecionamento pode criar arquivo vazio se houver erro: confira
o código de saída antes de consumir a AST.

## Regras da linguagem

| Elemento | Sintaxe |
|---|---|
| Contrato | `contrato: Nome` |
| Variável | `nome: tipo = valor`, dentro de `variaveis` |
| Função | `nome:`, dentro de `funcoes` |
| Retorno declarado, opcional | `retorna: tipo`, antes de `passos` |
| Atribuição | `- definir nome como valor` |
| Soma | `- somar valor em nome` |
| Retorno | `- retornar valor` |
| Estados, opcional | `estados:` seguido de `  - Nome`; o primeiro é o inicial |
| Guarda de estado, opcional | `quando: Estado`, antes de `retorna` |
| Subtração | `- subtrair valor de nome` |
| Verificação | `- exigir valor` ou `- exigir valor comparador valor` |
| Transferência | `- transferir valor para endereco` |
| Transição | `- mudar para Estado` |

- Comparadores: `igual a`, `diferente de`, `maior que`, `menor que`,
  `maior ou igual a`, `menor ou igual a`.
- Seções na ordem `contrato`, `estados` (opcional), `variaveis` (opcional), `funcoes`.
- Pelo menos uma função e um passo por função. Se presente, `variaveis` exige uma declaração.
- Indentação: declarações/funções com 2 espaços; `retorna`/`passos` com 4; comandos com 6.
- Identificadores: letras ASCII ou `_` no início; depois letras, dígitos ou `_`.
  Diferenciam maiúsculas/minúsculas; palavras reservadas não podem ser nomes.
- Tipos: `numero`, `texto`, `logico`, `endereco`.
- Valores: inteiro não negativo sem zeros à esquerda, texto entre aspas duplas,
  `verdadeiro`, `falso` ou nome de referência. Texto aceita Unicode e escapes
  `\"`, `\\`, `\n`, `\r`, `\t`.
- Comentários começam com `#` fora de textos; linhas vazias são aceitas.
- Quebras LF e CRLF são aceitas, inclusive última linha sem quebra.
- Sem parâmetros, expressões aritméticas gerais, desvios condicionais ou laços.

A linguagem utiliza gramática própria e um vocabulário formal restrito. A
semelhança visual com YAML não implica compatibilidade com seus recursos gerais,
como âncoras, aliases, tags ou estruturas arbitrárias.

## Organização

| Arquivo | Responsabilidade |
|---|---|
| `src/grammar.pest` | Vocabulário e gramática; gera o parser |
| `src/lib.rs` | Entrada pública e construção da AST |
| `src/ast.rs` | Estruturas sintáticas compartilhadas |
| `src/symbol_table.rs` | Tabela de símbolos (variáveis e estados) |
| `src/verify.rs` | `verify_ir`: resolução de nomes e tipagem da AST |
| `src/middle/` | Middle-end: IR, rebaixamento, passes e validação |
| `src/main.rs` | CLI de leitura de arquivo e saída JSON |
| `examples/` | Contador, cadastro, cofre e exemplos inválidos |
| `tests/` | Testes do front-end, da IR e do middle-end |
| `docs/etapa-1.md` | Análise, planejamento e limites da etapa 1 |
| `docs/parte2.md` | Middle-end: passes, regras e decisões da etapa 2 |

A aceitação sintática indica conformidade com a estrutura da linguagem. Referências
inexistentes, declarações duplicadas, incompatibilidades de tipos e violações de
segurança são detectadas pelo middle-end (`--ir`), não pelo parser.

O parser é gerado automaticamente com Pest, que reúne reconhecimento léxico e
sintático em uma gramática PEG. Essa escolha adapta a previsão do relatório de
um scanner por expressões regulares e de uma gramática livre de contexto.
