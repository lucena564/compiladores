use contrato_simples::{
    ast::{Command, Value},
    parse,
};

const COUNTER: &str = include_str!("../examples/contador.contrato");

#[test]
fn counter_preserves_order_operands_and_source_location() {
    let ast = parse(COUNTER).unwrap();
    assert_eq!(ast.nome, "Contador");
    assert_eq!(ast.variaveis[0].inicial, Value::Numero("0".into()));
    assert_eq!(ast.funcoes.len(), 2);
    assert_eq!(
        ast.funcoes[0].passos[0].comando,
        Command::Somar {
            destino: "total".into(),
            valor: Value::Numero("1".into())
        }
    );
    let origin = &ast.funcoes[0].passos[0].origem;
    assert_eq!(origin.linha, 8);
    assert!(COUNTER[origin.inicio..origin.fim].contains("somar 1 em total"));
}

#[test]
fn accepts_all_types_and_decodes_text() {
    let ast = parse(include_str!("../examples/cadastro.contrato")).unwrap();
    assert_eq!(ast.variaveis[1].inicial, Value::Logico(false));
    let ast =
        parse(&COUNTER.replace("numero = 0", "texto = \"Olá # mundo\\n\\\"sim\\\"\"")).unwrap();
    assert_eq!(
        ast.variaveis[0].inicial,
        Value::Texto("Olá # mundo\n\"sim\"".into())
    );
}

#[test]
fn accepts_comments_crlf_and_missing_final_newline() {
    for source in [
        COUNTER.replace('\n', "\r\n"),
        COUNTER.trim_end().into(),
        COUNTER.replace("contrato: Contador", "contrato: Contador # exemplo\n\n"),
    ] {
        let ast = parse(&source).unwrap();
        assert_eq!(ast.nome, "Contador");
        assert!(ast.origem.fim <= source.len());
    }
}

#[test]
fn rejects_malformed_documents_with_location() {
    for source in [
        "".to_owned(),
        COUNTER.replace("contrato:", "contrato"),
        COUNTER.replace("  total:", "   total:"),
        COUNTER.replace("  total:", "\ttotal:"),
        COUNTER.replace("numero =", "inteiro ="),
        COUNTER.replace("somar 1 em total", "executar total"),
        COUNTER.replace("somar 1 em total", "somar 01 em total"),
        COUNTER.replace("somar 1 em total", "somar -1 em total"),
        COUNTER.replace("somar 1 em total", "somar 1.5 em total"),
        COUNTER.replace("Contador", "1Contrato"),
        COUNTER.replace("Contador", "contrato"),
        format!("{COUNTER}lixo\n"),
        "contrato: Vazio\nfuncoes:\n".into(),
        "contrato: Vazio\nfuncoes:\n  fazer:\n    passos:\n".into(),
        COUNTER.replace("numero = 0", "texto = \"sem fim"),
        COUNTER.replace("numero = 0", "texto = \"tab\tcru\""),
        COUNTER.replace("numero = 0", "texto = \"escape\\q\""),
    ] {
        let error = parse(&source).expect_err(&source);
        assert!(error.contains("Erro de sintaxe:"));
        assert!(error.contains("-->"));
    }
}

#[test]
fn leaves_semantics_for_next_stage() {
    let ast =
        parse(&COUNTER.replace("somar 1 em total", "somar \"texto\" em inexistente")).unwrap();
    assert_eq!(
        ast.funcoes[0].passos[0].comando,
        Command::Somar {
            destino: "inexistente".into(),
            valor: Value::Texto("texto".into())
        }
    );
    // Duplicidade de nomes também pertence à análise semântica.
    assert!(parse(&COUNTER.replace("consultar:", "incrementar:")).is_ok());
}

#[test]
fn no_variables_and_large_integer_are_syntactically_valid() {
    let source = "contrato: Simples\nfuncoes:\n  consultar:\n    retorna: numero\n    passos:\n      - retornar 9999999999999999999999999999999999999999999";
    let ast = parse(source).unwrap();
    assert!(ast.variaveis.is_empty());
    assert_eq!(ast.funcoes.len(), 1);
}

#[test]
fn cli_reports_invalid_file_without_json_output() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_contrato-simples"))
        .arg("arquivo-inexistente.contrato")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}
