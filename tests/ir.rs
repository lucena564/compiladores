use contrato_simples::{
    ast::{Type},
    parse,
    symbol_table::SymbolTable,
    verify::verify_ir,
};

const COUNTER: &str = include_str!("../examples/contador.contrato");

#[test]
fn symbol_table_stores_variables() {
    let ast = parse(COUNTER).unwrap();

    let table = SymbolTable::from_contract(&ast).unwrap();

    assert_eq!(table.len(), 1);
    assert!(table.contains("total"));
    assert_eq!(table.lookup("total"), Some(&Type::Numero));
}

#[test]
fn symbol_table_rejects_duplicate_variables() {
    let source = r#"contrato: Teste
variaveis:
  total: numero = 0
  total: numero = 1
funcoes:
  consultar:
    passos:
      - retornar total
"#;

    let ast = parse(source).unwrap();

    let result = SymbolTable::from_contract(&ast);

    assert!(result.is_err());
}

#[test]
fn valid_counter_passes_ir_verification() {
    let ast = parse(COUNTER).unwrap();

    assert!(verify_ir(&ast).is_ok());
}

#[test]
fn rejects_unknown_variable() {
    let source =
        COUNTER.replace("somar 1 em total", "somar 1 em inexistente");

    let ast = parse(&source).unwrap();

    let result = verify_ir(&ast);

    assert!(result.is_err());

    let errors = result.unwrap_err();

    assert!(
        errors
            .iter()
            .any(|error| error.mensagem.contains("inexistente"))
    );
}

#[test]
fn rejects_wrong_value_type() {
    let source =
        COUNTER.replace("somar 1 em total", "somar \"texto\" em total");

    let ast = parse(&source).unwrap();

    let result = verify_ir(&ast);

    assert!(result.is_err());
}

#[test]
fn rejects_wrong_return_type() {
    let source =
        COUNTER.replace("retornar total", "retornar \"texto\"");

    let ast = parse(&source).unwrap();

    let result = verify_ir(&ast);

    assert!(result.is_err());
}

#[test]
fn rejects_return_in_function_without_return_type() {
    let source =
        COUNTER.replace(
            "somar 1 em total",
            "retornar total",
        );

    let ast = parse(&source).unwrap();

    let result = verify_ir(&ast);

    assert!(result.is_err());
}

#[test]
fn accepts_address_initialized_with_text() {
    let source = r#"contrato: Cadastro
variaveis:
  dono: endereco = "0x0000000000000000000000000000000000000000"
funcoes:
  consultar:
    retorna: endereco
    passos:
      - retornar dono
"#;

    let ast = parse(source).unwrap();

    assert!(verify_ir(&ast).is_ok());
}

#[test]
fn rejects_function_with_missing_return() {
    let source = r#"contrato: Teste
variaveis:
  total: numero = 0
funcoes:
  consultar:
    retorna: numero
    passos:
      - somar 1 em total
"#;

    let ast = parse(source).unwrap();

    let result = verify_ir(&ast);

    assert!(result.is_err());
}

#[test]
fn rejects_duplicate_functions() {
    let source =
        COUNTER.replace("consultar:", "incrementar:");

    let ast = parse(&source).unwrap();

    let result = verify_ir(&ast);

    assert!(result.is_err());
}