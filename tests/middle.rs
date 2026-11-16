use contrato_simples::{
    ast::{Command, Comparator, Condition, Value},
    middle::{
        analyze,
        ir::{Categoria, ContratoIR, Mutabilidade, Operacao, TipoErro},
        lower::lower,
        passes::{self, cei, errors, reentrancy, state_machine},
        validate::validate,
        Diagnostico, Severidade,
    },
    parse,
};

const COFRE: &str = include_str!("../examples/cofre.contrato");
const COUNTER: &str = include_str!("../examples/contador.contrato");
const ENDERECO: &str = "\"0x1111111111111111111111111111111111111111\"";

fn ir_of(source: &str) -> ContratoIR {
    analyze(&parse(source).unwrap()).unwrap().ir
}

fn errors_of(source: &str) -> Vec<Diagnostico> {
    analyze(&parse(source).unwrap()).unwrap_err()
}

fn has_error(diagnosticos: &[Diagnostico], passe: &str, trecho: &str) -> bool {
    diagnosticos.iter().any(|d| {
        d.severidade == Severidade::Erro && d.passe == passe && d.mensagem.contains(trecho)
    })
}

/// Contrato de uma função com os passos dados, saldo numérico e um endereço.
fn single_function(passos: &[&str]) -> String {
    let passos: String = passos.iter().map(|p| format!("      - {p}\n")).collect();
    format!(
        "contrato: Teste\nvariaveis:\n  saldo: numero = 10\n  dono: endereco = {ENDERECO}\n  ativo: logico = verdadeiro\nfuncoes:\n  f:\n    passos:\n{passos}"
    )
}

fn operations(ir: &ContratoIR, funcao: &str) -> Vec<(Operacao, bool)> {
    ir.funcoes
        .iter()
        .find(|f| f.nome == funcao)
        .unwrap()
        .instrucoes
        .iter()
        .map(|i| (i.operacao.clone(), i.injetada))
        .collect()
}

#[test]
fn parses_new_commands() {
    let ast = parse(COFRE).unwrap();
    assert_eq!(ast.versao_ast, 2);
    assert_eq!(ast.estados.len(), 2);
    assert_eq!(ast.funcoes[0].quando.as_deref(), Some("Aberto"));
    assert_eq!(
        ast.funcoes[0].passos[0].comando,
        Command::Exigir {
            condicao: Condition::Comparacao {
                esquerda: Value::Referencia("saldo".into()),
                operador: Comparator::Maior,
                direita: Value::Numero("0".into()),
            }
        }
    );
    assert_eq!(
        ast.funcoes[0].passos[3].comando,
        Command::Transferir {
            valor: Value::Numero("100".into()),
            destino: Value::Referencia("dono".into()),
        }
    );
    assert_eq!(
        ast.funcoes[1].passos[0].comando,
        Command::MudarEstado {
            estado: "Encerrado".into()
        }
    );

    for (texto, operador) in [
        ("igual a", Comparator::Igual),
        ("diferente de", Comparator::Diferente),
        ("menor que", Comparator::Menor),
        ("maior ou igual a", Comparator::MaiorOuIgual),
        ("menor ou igual a", Comparator::MenorOuIgual),
    ] {
        let ast = parse(&single_function(&[&format!("exigir saldo {texto} 1")])).unwrap();
        let Command::Exigir {
            condicao: Condition::Comparacao { operador: lido, .. },
        } = &ast.funcoes[0].passos[0].comando
        else {
            panic!("{texto}");
        };
        assert_eq!(*lido, operador);
    }
}

#[test]
fn keyword_prefixes_are_reserved() {
    // "retorna" é prefixo de "retornar"; ambos devem ser reservados.
    assert!(parse(&COUNTER.replace("total", "retornar")).is_err());
    assert!(parse(&COUNTER.replace("total", "retorna")).is_err());
}

#[test]
fn full_pipeline_accepts_examples_and_records_passes() {
    for source in [
        COFRE,
        COUNTER,
        include_str!("../examples/cadastro.contrato"),
    ] {
        let ir = ir_of(source);
        assert_eq!(
            ir.passes_aplicados,
            [
                passes::TIPAGEM,
                passes::MAQUINA_DE_ESTADOS,
                passes::CHECKS_EFFECTS_INTERACTIONS,
                passes::REENTRANCIA,
                passes::TRATAMENTO_DE_ERROS,
            ]
        );
        assert!(validate(&ir).is_ok());
    }
}

#[test]
fn rejects_invalid_examples() {
    let reentrancia = errors_of(include_str!("../examples/invalidos/reentrancia.contrato"));
    assert!(has_error(
        &reentrancia,
        passes::CHECKS_EFFECTS_INTERACTIONS,
        "após a interação externa"
    ));
    let estados = errors_of(include_str!(
        "../examples/invalidos/estado_inalcancavel.contrato"
    ));
    assert!(has_error(
        &estados,
        passes::MAQUINA_DE_ESTADOS,
        "'Pago' é inalcançável"
    ));
}

#[test]
fn typing_errors_stop_before_semantic_passes() {
    let diagnosticos = errors_of(&COFRE.replace("quando: Aberto", "quando: Fechado"));
    assert!(has_error(
        &diagnosticos,
        passes::TIPAGEM,
        "estado 'Fechado' não foi declarado"
    ));
    assert!(diagnosticos.iter().all(|d| d.passe == passes::TIPAGEM));

    let diagnosticos = errors_of(&COFRE.replace("mudar para Encerrado", "mudar para Fim"));
    assert!(has_error(&diagnosticos, passes::TIPAGEM, "'Fim'"));
}

#[test]
fn typing_checks_new_commands() {
    for (passo, trecho) in [
        ("exigir saldo", "esperado Logico"),
        ("exigir ativo maior que 1", "esperado Numero"),
        ("exigir saldo igual a verdadeiro", "esperado Numero"),
        ("subtrair 1 de dono", "não é do tipo numero"),
        ("transferir verdadeiro para dono", "esperado Numero"),
        ("transferir 1 para saldo", "esperado Endereco"),
        ("transferir 1 para \"0x12\"", "endereço inválido"),
    ] {
        let diagnosticos = errors_of(&single_function(&[passo]));
        assert!(
            has_error(&diagnosticos, passes::TIPAGEM, trecho),
            "{passo}: {diagnosticos:?}"
        );
    }
    // Literal válido e comparação entre endereços são aceitos.
    ir_of(&single_function(&[
        &format!("exigir dono diferente de {ENDERECO}"),
        &format!("transferir 1 para {ENDERECO}"),
    ]));
}

#[test]
fn text_variable_is_not_an_address() {
    let source = "contrato: T\nvariaveis:\n  nome: texto = \"x\"\nfuncoes:\n  f:\n    passos:\n      - transferir 1 para nome\n";
    assert!(has_error(
        &errors_of(source),
        passes::TIPAGEM,
        "esperado Endereco"
    ));
}

#[test]
fn state_guard_is_injected_first() {
    let ir = ir_of(COFRE);
    let ops = operations(&ir, "encerrar");
    assert_eq!(
        ops[0],
        (
            Operacao::ExigirEstado {
                estado: "Aberto".into()
            },
            true
        )
    );
    assert!(operations(&ir, "consultar_saldo")
        .iter()
        .all(|(op, _)| !matches!(op, Operacao::ExigirEstado { .. })));
}

#[test]
fn state_machine_rules() {
    let duas = COFRE.replace(
        "      - mudar para Encerrado",
        "      - mudar para Encerrado\n      - mudar para Aberto",
    );
    assert!(has_error(
        &errors_of(&duas),
        passes::MAQUINA_DE_ESTADOS,
        "mais de uma vez"
    ));

    // Sem guarda, a transição vale a partir de qualquer estado.
    let sem_guarda = include_str!("../examples/invalidos/estado_inalcancavel.contrato").replace(
        "      - definir valor como 0",
        "      - definir valor como 0\n  quitar:\n    passos:\n      - mudar para Pago",
    );
    ir_of(&sem_guarda);

    let redundante =
        analyze(&parse(&COFRE.replace("mudar para Encerrado", "mudar para Aberto")).unwrap());
    let diagnosticos = match redundante {
        Ok(analise) => analise.avisos,
        Err(erros) => erros,
    };
    assert!(diagnosticos
        .iter()
        .any(|d| d.severidade == Severidade::Aviso && d.mensagem.contains("redundante")));
}

#[test]
fn cei_order_is_enforced() {
    let transferir = format!("transferir 1 para {ENDERECO}");
    let casos = [
        (
            vec!["subtrair 1 de saldo", "exigir ativo"],
            "a verificação deve vir antes",
        ),
        (
            vec![transferir.as_str(), "exigir ativo"],
            "a verificação deve vir antes",
        ),
        (
            vec![transferir.as_str(), "definir ativo como falso"],
            "após a interação externa",
        ),
    ];
    for (passos, trecho) in casos {
        let diagnosticos = errors_of(&single_function(&passos));
        assert!(
            has_error(&diagnosticos, passes::CHECKS_EFFECTS_INTERACTIONS, trecho),
            "{passos:?}"
        );
    }

    let apos_retorno = COUNTER.replace(
        "      - retornar total",
        "      - retornar total\n      - somar 1 em total",
    );
    assert!(has_error(
        &errors_of(&apos_retorno),
        passes::CHECKS_EFFECTS_INTERACTIONS,
        "inalcançável"
    ));

    ir_of(&single_function(&[
        "exigir ativo",
        "definir ativo como falso",
        &format!("transferir 1 para {ENDERECO}"),
        &format!("transferir 2 para {ENDERECO}"),
    ]));
}

#[test]
fn mutability_is_inferred() {
    let source = "contrato: M\nvariaveis:\n  x: numero = 0\nfuncoes:\n  pura:\n    retorna: numero\n    passos:\n      - retornar 7\n  leitura:\n    retorna: numero\n    passos:\n      - retornar x\n  escrita:\n    passos:\n      - somar 1 em x\n";
    let ir = ir_of(source);
    let m: Vec<Mutabilidade> = ir.funcoes.iter().map(|f| f.mutabilidade).collect();
    assert_eq!(
        m,
        [
            Mutabilidade::Pura,
            Mutabilidade::Leitura,
            Mutabilidade::Escrita
        ]
    );
}

#[test]
fn reentrancy_lock_marks_only_interacting_functions() {
    let ir = ir_of(COFRE);
    assert!(ir.trava_reentrancia);
    let protegidas: Vec<&str> = ir
        .funcoes
        .iter()
        .filter(|f| f.protegida_contra_reentrancia)
        .map(|f| f.nome.as_str())
        .collect();
    assert_eq!(protegidas, ["sacar"]);

    let ir = ir_of(COUNTER);
    assert!(!ir.trava_reentrancia);
    assert!(ir.erros.is_empty());
}

#[test]
fn reentrancy_warns_about_altered_transfer_value() {
    let analise = analyze(
        &parse(&single_function(&[
            "definir saldo como 0",
            "transferir saldo para dono",
        ]))
        .unwrap(),
    )
    .unwrap();
    assert!(analise
        .avisos
        .iter()
        .any(|d| d.passe == passes::REENTRANCIA && d.mensagem.contains("depois de alterá-la")));
}

#[test]
fn balance_check_is_hoisted_before_unrelated_effects() {
    let ops = operations(&ir_of(COFRE), "sacar");
    let categorias: Vec<&str> = ops
        .iter()
        .map(|(op, injetada)| match (op, injetada) {
            (Operacao::ExigirEstado { .. }, _) => "guarda",
            (Operacao::Exigir { .. }, true) => "saldo",
            (Operacao::Exigir { .. }, false) => "exigir",
            (Operacao::Somar { .. }, _) => "somar",
            (Operacao::Subtrair { .. }, _) => "subtrair",
            (Operacao::Transferir { .. }, _) => "transferir",
            _ => "outro",
        })
        .collect();
    assert_eq!(
        categorias,
        [
            "guarda",
            "exigir",
            "saldo",
            "somar",
            "subtrair",
            "transferir"
        ]
    );
}

#[test]
fn balance_check_stays_after_write_to_same_variable() {
    let ops = operations(
        &ir_of(&single_function(&[
            "somar 5 em saldo",
            "subtrair 12 de saldo",
        ])),
        "f",
    );
    assert!(matches!(ops[0].0, Operacao::Somar { .. }));
    assert!(matches!(
        (&ops[1].0, ops[1].1),
        (Operacao::Exigir { .. }, true)
    ));
    assert!(matches!(ops[2].0, Operacao::Subtrair { .. }));
}

#[test]
fn redundant_balance_check_is_not_injected() {
    for condicao in ["saldo maior ou igual a 3", "3 menor ou igual a saldo"] {
        let ir = ir_of(&single_function(&[
            &format!("exigir {condicao}"),
            "subtrair 3 de saldo",
        ]));
        let ops = operations(&ir, "f");
        assert_eq!(
            ops.iter().filter(|(_, injetada)| *injetada).count(),
            0,
            "{condicao}"
        );
    }
}

#[test]
fn every_revert_point_has_unique_error_code() {
    let ir = ir_of(COFRE);
    let pontos: Vec<(TipoErro, u32)> = ir
        .funcoes
        .iter()
        .flat_map(|f| &f.instrucoes)
        .filter_map(|i| i.erro.map(|e| (e.tipo, e.codigo)))
        .collect();
    assert_eq!(
        pontos,
        [
            (TipoErro::EstadoInvalido, 1),
            (TipoErro::CondicaoNaoAtendida, 2),
            (TipoErro::SaldoInsuficiente, 3),
            (TipoErro::TransferenciaFalhou, 4),
            (TipoErro::EstadoInvalido, 5),
        ]
    );
    let catalogo: Vec<TipoErro> = ir.erros.iter().map(|e| e.tipo).collect();
    assert_eq!(
        catalogo,
        [
            TipoErro::EstadoInvalido,
            TipoErro::CondicaoNaoAtendida,
            TipoErro::SaldoInsuficiente,
            TipoErro::TransferenciaFalhou,
            TipoErro::ChamadaReentrante,
        ]
    );
}

#[test]
fn passes_run_in_isolation_on_lowered_ir() {
    let mut ir = lower(&parse(COFRE).unwrap());
    let mut diagnosticos = Vec::new();
    ir = state_machine::run(ir, &mut diagnosticos);
    ir = cei::run(ir, &mut diagnosticos);
    ir = reentrancy::run(ir, &mut diagnosticos);
    ir = errors::run(ir, &mut diagnosticos);
    assert!(diagnosticos.is_empty());
    let mut esperado = ir_of(COFRE);
    esperado.passes_aplicados.clear();
    assert_eq!(ir, esperado);
}

#[test]
fn validator_detects_regressions() {
    let valid = ir_of(COFRE);

    let mut sem_guarda = valid.clone();
    sem_guarda.funcoes[1].instrucoes.remove(0);
    assert!(validate(&sem_guarda).is_err());

    let mut categoria_errada = valid.clone();
    categoria_errada.funcoes[0].instrucoes[1].categoria = Categoria::Efeito;
    assert!(validate(&categoria_errada).is_err());

    let mut sem_catalogo = valid.clone();
    sem_catalogo.erros.clear();
    assert!(validate(&sem_catalogo).is_err());

    let mut sem_trava = valid.clone();
    sem_trava.funcoes[0].protegida_contra_reentrancia = false;
    assert!(validate(&sem_trava).is_err());

    let mut referencia_solta = valid.clone();
    if let Operacao::Subtrair { destino, .. } =
        &mut referencia_solta.funcoes[0].instrucoes[4].operacao
    {
        *destino = "fantasma".into();
    }
    assert!(validate(&referencia_solta).is_err());

    // Antes do passe de erros, a ausência de erro associado é legítima.
    let mut parcial = lower(&parse(COFRE).unwrap());
    parcial.passes_aplicados.push(passes::TIPAGEM.into());
    assert!(validate(&parcial).is_ok());
}

#[test]
fn cli_emits_ir_or_semantic_errors() {
    let bin = env!("CARGO_BIN_EXE_contrato-simples");
    let ok = std::process::Command::new(bin)
        .args(["--ir", "examples/cofre.contrato"])
        .output()
        .unwrap();
    assert!(ok.status.success());
    let json: serde_json::Value = serde_json::from_slice(&ok.stdout).unwrap();
    assert_eq!(json["versao_ir"], 1);

    let err = std::process::Command::new(bin)
        .args(["--ir", "examples/invalidos/reentrancia.contrato"])
        .output()
        .unwrap();
    assert!(!err.status.success());
    assert!(err.stdout.is_empty());
    assert!(String::from_utf8_lossy(&err.stderr).contains("checks_effects_interactions"));
}
