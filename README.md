# Contrato Simples — etapa 1

Contrato Simples é uma linguagem declarativa em português, com estrutura visual
semelhante a YAML, destinada à especificação simplificada de contratos. O front-end
em Rust reconhece arquivos `.contrato` e produz uma árvore sintática abstrata
(AST), exportada em JSON. A geração de Solidity pertence às etapas posteriores.

A [fundamentação e análise de conformidade](docs/etapa-1.md) apresenta o planejamento,
o contrato da AST e a comparação com o relatório de referência.

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

- Seções na ordem `contrato`, `variaveis` (opcional), `funcoes`.
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
- Sem parâmetros, expressões aritméticas gerais, condições, laços ou chamadas externas.

A linguagem utiliza gramática própria e um vocabulário formal restrito. A
semelhança visual com YAML não implica compatibilidade com seus recursos gerais,
como âncoras, aliases, tags ou estruturas arbitrárias.

## Organização

| Arquivo | Responsabilidade |
|---|---|
| `src/grammar.pest` | Vocabulário e gramática; gera o parser |
| `src/lib.rs` | Entrada pública e construção da AST |
| `src/ast.rs` | Estruturas sintáticas compartilhadas |
| `src/main.rs` | CLI de leitura de arquivo e saída JSON |
| `examples/` | Contador e cadastro |
| `tests/frontend.rs` | Testes do front-end |
| `docs/etapa-1.md` | Análise, planejamento e limites da entrega |

A aceitação sintática indica conformidade com a estrutura da linguagem. Referências
inexistentes, declarações duplicadas e incompatibilidades de tipos ainda podem
estar presentes na AST e dependem de análise semântica posterior.

O parser é gerado automaticamente com Pest, que reúne reconhecimento léxico e
sintático em uma gramática PEG. Essa escolha adapta a previsão do relatório de
um scanner por expressões regulares e de uma gramática livre de contexto. A
tabela de símbolos, o passe `VerifyIR()` e as verificações de segurança ainda
não estão implementados; a comparação detalhada está no documento teórico.
