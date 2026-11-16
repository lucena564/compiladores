# Fluxo de teste manual da etapa 2 (middle-end)

Este guia mostra como verificar, passo a passo, o funcionamento do middle-end.
A descrição técnica completa está em [parte2.md](parte2.md).

## 1. Preparar o ambiente

### 1.1 Instalar o Rust (uma vez só)

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
cargo --version
```

Quando o instalador perguntar, escolha a opção padrão (`1`). Se o `cargo` não
for encontrado em um terminal novo, rode `source "$HOME/.cargo/env"` de novo ou
reabra o terminal.

### 1.2 Instalar o `jq` (opcional)

O `jq` serve para filtrar o JSON e deixá-lo mais legível. No macOS ele
geralmente já vem instalado (`which jq`). Se não vier:

```sh
brew install jq
```

### 1.3 Entrar na pasta do projeto

```sh
cd ./compiladores
```

Todos os comandos abaixo partem dessa pasta. A primeira compilação demora
alguns segundos, porque o Cargo baixa as dependências.

## 2. Rodar os testes automáticos

```sh
cargo test
```

O esperado é que três blocos terminem com `test result: ok`:

| Arquivo | Testes | O que cobre |
|---|---|---|
| `tests/frontend.rs` | 7 | Parser e AST (etapa 1) |
| `tests/ir.rs` | 10 | Tabela de símbolos e `verify_ir` |
| `tests/middle.rs` | 20 | Middle-end: passes, injeções e validação da IR |

## 3. Ver o compilador funcionando

### 3.1 Contrato correto

```sh
cargo run -- --ir examples/cofre.contrato
```

A IR completa aparece em JSON. Na função `sacar`, repare em três coisas:

- as instruções com `"injetada": true`: a guarda de estado e a verificação de
  saldo `saldo >= 100`, inseridas pelo compilador;
- o campo `"erro"` em cada ponto que pode reverter, com um código único;
- o catálogo `"erros"` no final, com apenas os erros usados.

Para ver só as instruções de `sacar`, de forma resumida:

```sh
cargo run -q -- --ir examples/cofre.contrato | jq '.funcoes[0].instrucoes[] | {op: .operacao.tipo, injetada, erro}'
```

O esperado são seis instruções, nesta ordem:

| # | Operação | `injetada` | Erro |
|---|---|---|---|
| 1 | `exigir_estado` | `true` | `EstadoInvalido` (código 1) |
| 2 | `exigir` (`saldo > 0`) | `false` | `CondicaoNaoAtendida` (código 2) |
| 3 | `exigir` (`saldo >= 100`) | `true` | `SaldoInsuficiente` (código 3) |
| 4 | `somar` | `false` | — |
| 5 | `subtrair` | `false` | — |
| 6 | `transferir` | `false` | `TransferenciaFalhou` (código 4) |

A verificação de saldo (3) foi injetada **antes** do `somar` (4), mesmo tendo
sido gerada a partir do `subtrair` (5). Isso acontece porque o `somar` não altera
`saldo`, então é seguro subir a verificação para junto das demais.

Outros resumos úteis:

```sh
# Mutabilidade e proteção de cada função
cargo run -q -- --ir examples/cofre.contrato | jq '.funcoes[] | {nome, mutabilidade, protegida_contra_reentrancia}'

# Catálogo de erros
cargo run -q -- --ir examples/cofre.contrato | jq '.erros[].tipo'

# Passes aplicados
cargo run -q -- --ir examples/cofre.contrato | jq '.passes_aplicados'
```

### 3.2 Bug de reentrância

```sh
cargo run -- --ir examples/invalidos/reentrancia.contrato
```

Esperado:

```
erro [checks_effects_interactions] 11:1: violação de Checks-Effects-Interactions em 'sacar': ...
```

### 3.3 Estado inalcançável

```sh
cargo run -- --ir examples/invalidos/estado_inalcancavel.contrato
```

Esperado:

```
erro [maquina_de_estados] 5:1: estado 'Pago' é inalcançável a partir do estado inicial 'Pendente'; funções que nunca poderão executar: reembolsar
```

### 3.4 Só a AST, como na etapa 1

```sh
cargo run -- examples/cofre.contrato
```

Sem `--ir`, a saída é a AST, sem tipos resolvidos, instruções injetadas ou
erros. Comparar essa saída com a do item 3.1 mostra o que o middle-end
acrescentou.

## 4. Quebrar o contrato de propósito

Edite `examples/cofre.contrato` e, depois de cada mudança, rode:

```sh
cargo run -- --ir examples/cofre.contrato
```

Desfaça cada alteração antes de testar a próxima.

| Mude isto em `cofre.contrato` | O que deve acontecer |
|---|---|
| Mova `- transferir 100 para dono` para antes de `- subtrair 100 de saldo` | Erro `[checks_effects_interactions]`: efeito após interação (risco de reentrância) |
| Troque `quando: Aberto` por `quando: Fechado` | Erro `[tipagem]`: o estado `Fechado` não foi declarado |
| Apague a função `encerrar` inteira (as 4 linhas) | Erro `[maquina_de_estados]`: o estado `Encerrado` fica inalcançável |
| Troque o endereço por `"0x123"` | Erro `[tipagem]`: endereço inválido |
| Troque `- somar 1 em saques` por `- somar "x" em saques` | Erro `[tipagem]`: tipo incompatível |
| Coloque `- exigir saldo maior ou igual a 100` antes de `- subtrair 100 de saldo` | Compila; a verificação de saldo injetada some, porque ficou redundante |
| Troque `- somar 1 em saques` por `- somar 1 em saldo` | Compila; a verificação injetada não sobe mais, porque `saldo` foi alterado antes |
| Adicione `- somar 1 em saques` depois de `- retornar saldo` | Erro `[checks_effects_interactions]`: passo inalcançável após `retornar` |
| Na função `encerrar`, adicione `- exigir saldo maior que 0` depois de `- mudar para Encerrado` | Erro `[checks_effects_interactions]`: a verificação deve vir antes dos efeitos |
| Na função `encerrar`, troque `mudar para Encerrado` por `mudar para Aberto` | Aviso de transição redundante e erro de estado inalcançável (`Encerrado`) |

Nos dois casos que compilam, use o comando `jq` do item 3.1 para ver a
diferença nas instruções de `sacar`.

Ao terminar, restaure o exemplo original:

```sh
git checkout examples/cofre.contrato
```

## 5. Escrever um contrato do zero (opcional)

Crie um arquivo, por exemplo `/tmp/meu.contrato`:

```yaml
contrato: Doacao
estados:
  - Aberta
  - Fechada
variaveis:
  arrecadado: numero = 500
  beneficiario: endereco = "0x2222222222222222222222222222222222222222"
funcoes:
  repassar:
    quando: Aberta
    passos:
      - subtrair 500 de arrecadado
      - mudar para Fechada
      - transferir 500 para beneficiario
  total:
    retorna: numero
    passos:
      - retornar arrecadado
```

E rode:

```sh
cargo run -- --ir /tmp/meu.contrato
```

Esse contrato deve compilar. Depois, experimente inverter a ordem dos passos
de `repassar` para ver os erros de CEI.

## 6. Solução de problemas

| Sintoma | Causa provável | Solução |
|---|---|---|
| `zsh: command not found: cargo` | O Rust não está instalado, ou o terminal não carregou o ambiente | Faça o item 1.1, ou rode `source "$HOME/.cargo/env"` |
| `zsh: command not found: jq` | O `jq` não está instalado | `brew install jq`, ou rode os comandos sem o trecho `\| jq ...` |
| `error: unexpected argument` ou a mensagem de uso | Faltou o `--` depois de `cargo run` | Use `cargo run -- --ir arquivo.contrato` |
| `Erro de sintaxe` com `-->` | A indentação ou a palavra-chave está errada | Use 2 espaços para funções, 4 para `quando`/`retorna`/`passos` e 6 para os comandos |
