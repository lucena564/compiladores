use std::{env, fs, process::ExitCode};

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 1 {
        return Err("Uso: contrato-simples <arquivo.contrato>\nA saída é a AST em JSON.".into());
    }
    let source = fs::read_to_string(&args[0]).map_err(|e| format!("{}: {e}", args[0]))?;
    let ast = contrato_simples::parse(&source).map_err(|e| format!("{}: {e}", args[0]))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&ast).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
