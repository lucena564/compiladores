# Fundamentação e análise de conformidade da etapa 1

## 1. Objetivo e delimitação

A primeira etapa do projeto corresponde ao front-end do transpilador: recebe uma
especificação textual de contrato e produz uma árvore sintática abstrata
(Abstract Syntax Tree — AST). Essa representação registra a organização do
programa de maneira independente de sua apresentação textual, estabelecendo a
interface inicial com as etapas subsequentes.

Nesta entrega, a linguagem de origem é denominada Contrato Simples e utiliza a
extensão `.contrato`. Sua finalidade é permitir a descrição de contratos por meio
de um vocabulário restrito em português e de uma estrutura visual semelhante à
utilizada em YAML. O escopo confirmado compreende a definição da linguagem, o
reconhecimento de sua sintaxe e a construção da AST.

A geração de código Solidity pertence ao back-end e permanece fora desta etapa.
No projeto completo, o arquivo `.sol` produzido ainda deverá ser compilado para
execução em ambiente compatível com a Ethereum Virtual Machine (EVM). Portanto,
a aceitação de um arquivo pelo front-end demonstra sua conformidade sintática,
mas não comprova a correção ou a segurança do contrato correspondente.

## 2. Fundamentação do front-end

O reconhecimento léxico identifica elementos básicos da linguagem, como nomes,
palavras reservadas e literais. A análise sintática estabelece como esses
componentes podem ser combinados para formar declarações, funções e comandos.
Embora essas funções sejam conceitualmente distintas, a implementação adotada
as reúne em uma única gramática, processada pela biblioteca Pest em Rust.

Pest utiliza gramáticas de expressões de análise, conhecidas como Parsing
Expression Grammars (PEG). Nesse formalismo, as alternativas são avaliadas por
escolha ordenada: uma alternativa posterior é examinada quando a anterior falha.
Esse comportamento deve ser considerado ao definir a ordem das regras. A
[documentação oficial do Pest](https://pest.rs/book/grammars/syntax.html) descreve
os operadores empregados na gramática do projeto.

Essa escolha representa uma adaptação da arquitetura sugerida no relatório,
que prevê um scanner baseado em expressões regulares e o mapeamento de uma
gramática livre de contexto para um gerador de parser. O projeto mantém o parser
gerado automaticamente em Rust, mas não implementa aquela separação literal.
As regras lexicais atuais estão incorporadas à PEG; não há um fluxo independente
de tokens nem um scanner por expressões regulares. PEG e gramática livre de
contexto não são formalismos intercambiáveis, ainda que possam descrever
construções sintáticas semelhantes neste subconjunto da linguagem.

Após o reconhecimento, o programa percorre as estruturas produzidas pelo parser
e constrói a AST. Essa construção ocorre em `src/lib.rs`, separadamente da
gramática. Não são realizadas verificações de tipos, resolução de nomes ou
análises de fluxo nessa conversão. Essa separação preserva o papel do front-end
e evita antecipar responsabilidades atribuídas à análise semântica.

## 3. Definição da linguagem

Contrato Simples adota uma linguagem específica de domínio, com um conjunto
reduzido de construções. O documento declara um contrato, uma seção opcional de
variáveis e uma seção obrigatória de funções. A ordem fixa das seções e a
indentação explícita tornam sua estrutura previsível, tanto para o autor do
arquivo quanto para o reconhecedor.

A aparência se aproxima de YAML, mas a linguagem possui gramática própria.
Não oferece recursos gerais de YAML, como âncoras, aliases ou tags. A proximidade
com a linguagem humana decorre do vocabulário em português; o texto continua
formal e não admite interpretações livres de frases naturais.

As variáveis declaram nome, tipo e valor inicial. Os tipos disponíveis são
`numero`, `texto`, `logico` e `endereco`. As funções possuem nome, indicação
opcional de retorno e uma sequência não vazia de passos. Nesta versão, os
comandos disponíveis são `definir`, `somar` e `retornar`, sem parâmetros,
condições, laços ou chamadas externas.

Os valores podem ser inteiros não negativos, textos entre aspas, booleanos ou
referências por nome. Números são preservados como sequências decimais na AST,
para que o front-end não imponha um limite decorrente de um tipo inteiro da
implementação. O intervalo permitido na linguagem de destino será uma decisão
posterior. Da mesma forma, a declaração de um endereço registra a intenção do
autor, sem validar seu formato ou sua compatibilidade com o valor informado.

A indentação exige dois espaços para variáveis e nomes de funções, quatro para
os campos das funções e seis para os comandos. Identificadores utilizam letras
ASCII, dígitos e sublinhado, com restrições para o primeiro caractere e para
palavras reservadas. Textos aceitam Unicode. As regras completas e os exemplos
de uso estão no [README](../README.md); a especificação executável é
[src/grammar.pest](../src/grammar.pest).

## 4. Planejamento e implementação

O desenvolvimento foi organizado em cinco atividades, cada uma associada a uma
condição observável de conclusão.

| Atividade | Critério de aceite | Evidência no projeto |
|---|---|---|
| Definir o subconjunto da linguagem | Especificar vocabulário, estrutura e indentação | `README.md` e `examples/` |
| Formalizar o reconhecimento | Aceitar apenas documentos completos conformes à gramática | `src/grammar.pest`, com delimitadores de início e fim |
| Construir a AST | Preservar declarações, valores, ordem dos comandos e origem | `src/ast.rs` e `src/lib.rs` |
| Disponibilizar a execução | Ler um arquivo e produzir JSON ou diagnóstico localizado | `src/main.rs` |
| Verificar o front-end | Exercitar aceitação, rejeição e separação de responsabilidades | `tests/frontend.rs` |

Essas atividades estão implementadas. A interface de linha de comando emite a
AST em JSON pela saída padrão e encaminha os erros para a saída de diagnóstico.
Entradas rejeitadas pelo parser incluem indicação de linha e coluna. O parser
interrompe a análise no erro reportado; não há recuperação para acumular vários
diagnósticos em uma única execução.

## 5. Contrato inicial da representação sintática

A entrada pública da biblioteca é `parse(&str) -> Result<Contract, String>`.
O resultado bem-sucedido contém uma AST cujo formato JSON possui `versao_ast: 1`.
A serialização permite inspecionar o resultado; as estruturas Rust constituem a
interface programática para os próximos módulos.

| Estrutura | Informação representada |
|---|---|
| `Contract` | Nome do contrato, versão da AST, variáveis, funções e origem |
| `Variable` | Nome, tipo declarado, valor inicial e origem |
| `Function` | Nome, retorno declarado opcional, comandos e origem |
| `Action` | Comando e origem |
| `Command` | Atribuição, soma ou retorno, com seus operandos |
| `Value` | Número, texto, booleano ou referência |
| `Span` | Intervalo de bytes UTF-8, linha e coluna de origem |

Para árvores produzidas pelo parser, a gramática garante um contrato por arquivo,
a presença de pelo menos uma função e de pelo menos um comando em cada função.
Quando a seção de variáveis existe, ela contém pelo menos uma declaração. Os
vetores preservam a ordem do programa fonte. O fim de cada intervalo de origem é
exclusivo; a quebra final acrescentada internamente, quando necessária, não
ultrapassa o comprimento original nos intervalos exportados.

Essas propriedades descrevem a saída do parser. Os tipos públicos também
permitem construir árvores diretamente em Rust, conforme a estratégia proposta
no relatório para viabilizar trabalho independente do parser. Entretanto, uma
árvore construída manualmente não recebe automaticamente essas garantias.
Ainda não existe um validador estrutural independente, e o JSON não possui
interface de desserialização nesta entrega.

A AST é uma representação intermediária sintática inicial. Não contém tabela
de símbolos, tipos inferidos, efeitos ou informações de fluxo de controle. O
contrato está documentado e implementado como proposta de integração, mas sua
aprovação conjunta pelos quatro integrantes da equipe não foi demonstrada.

## 6. Comparação com o relatório de referência

A comparação considera o PDF original e o escopo restrito à fase 1. A numeração
das fases do sistema deve ser distinguida das responsabilidades dos integrantes:
a engenharia da IR, por exemplo, é atribuída ao membro 2 e inclui atividades de
integração necessárias desde o início do projeto.

| Item previsto no relatório | Situação atual | Avaliação |
|---|---|---|
| Reconhecimento léxico | Regras de nomes, literais e palavras reservadas na PEG | Função atendida; scanner separado por expressões regulares ausente |
| Parser gerado automaticamente em Rust | Derivação do parser por `pest_derive` | Atendido |
| Mapeamento de gramática livre de contexto | Gramática executável PEG | Adaptação; não corresponde literalmente ao formalismo solicitado |
| Verificação da boa formação sintática | Consumo integral da entrada e rejeição de construções inválidas | Atendido no subconjunto definido |
| Conversão do texto para AST | Estruturas públicas e exportação JSON | Atendido |
| Contrato inicial da IR | Estruturas e propriedades documentadas | Base implementada; acordo coletivo da equipe não comprovado |
| Árvores construídas independentemente do parser | Tipos públicos instanciáveis em Rust | Possível; testes dos futuros passes ainda não realizados |
| Tabela de símbolos e escopos | Não implementados | Pendência da engenharia da IR/semântica |
| Passe `VerifyIR()` | Não implementado | Pendência de validação estrutural da IR |
| Ações simples associadas à sintaxe | Construção da AST após o reconhecimento | Adaptação de organização; sem avaliações semânticas no parser |
| Pré-dimensionamento a partir do scanning | Sem contagem prévia de declarações | Estratégia proposta ainda não implementada |
| Passes semânticos isolados | Não implementados | Fora da fase 1 autorizada |
| CEI, reentrância, máquinas de estado e tratamento de erros | Não implementados | Fora da fase 1 autorizada |
| Geração Solidity ou bytecode EVM | Não implementada | Fora da fase 1 autorizada |

Assim, o objetivo funcional da fase 1 está atendido no subconjunto escolhido,
mas a conformidade literal com todas as decisões técnicas do relatório é
parcial. Scanner por expressões regulares e gramática livre de contexto são as
duas divergências centrais. As demais pendências foram registradas para orientar
a continuidade, sem ampliar a implementação desta entrega.

## 7. Precisões teóricas sobre as diretrizes do relatório

A recomendação de pré-dimensionar estruturas pode reduzir realocações, mas não
permite afirmar que toda alocação terá complexidade constante O(1). A contagem
prévia de declarações exige processamento da entrada, e a inicialização ou
movimentação de elementos pode depender da quantidade de dados. Além disso,
pré-alocar capacidade não elimina a possibilidade de esgotamento de memória.
Por isso, o relatório deve tratar essa estratégia como uma possibilidade de
redução de realocações, cuja utilidade depende de medição e do modelo de custo.

O uso de Rust contribui para a segurança de memória, mas não substitui a
verificação das propriedades da representação intermediária. Em uma implementação
baseada em estruturas próprias e vetores, `VerifyIR()` deverá examinar as
invariantes efetivamente existentes, em vez de pressupor ponteiros soltos.
Verificações de tipos não resolvidos também precisam considerar o estágio do
pipeline: uma AST recém-produzida pode legitimamente conter referências ainda
não resolvidas.

Da mesma forma, a existência de uma AST compartilhada reduz o acoplamento entre
módulos, mas não garante que futuras alterações de gramática serão transparentes
às demais etapas. Essa estabilidade depende da preservação do contrato da
representação e da compatibilidade das mudanças. Expressões como “blindado” ou
“seguro” devem ser associadas a propriedades explícitas, análises implementadas
e evidências de validação, que ainda não fazem parte desta fase.

## 8. Verificação e limites da evidência

A suíte contém sete testes automatizados. Ela cobre exemplos de contratos,
comandos e literais, preservação de ordem e origem, comentários, Unicode em
textos, quebras LF/CRLF, ausência de quebra final, rejeição de entradas inválidas
e comportamento da CLI diante de arquivo inexistente. Um teste confirma que
nomes não resolvidos e tipos incompatíveis permanecem representáveis, preservando
a distinção entre aceitação sintática e validação semântica.

Os resultados demonstram o funcionamento dos casos exercitados. Não constituem
prova formal da gramática, cobertura exaustiva de entradas, benchmark de memória
ou validação de contratos em Solidity. A versão atual estabelece uma base pequena
e verificável para o desenvolvimento posterior, mantendo explícitos os limites
da implementação e as divergências em relação ao documento de referência.
