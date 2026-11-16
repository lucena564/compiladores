use std::{env, fs, process::ExitCode};

const USO: &str = "Uso: contrato-simples [--ir] <arquivo.contrato>\n\
Sem opções, a saída é a AST em JSON. Com --ir, executa o middle-end e emite a IR validada.";

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let (middle, path) = match args.as_slice() {
        [path] if !path.starts_with("--") => (false, path),
        [flag, path] if flag == "--ir" => (true, path),
        _ => return Err(USO.into()),
    };
    let source = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let ast = contrato_simples::parse(&source).map_err(|e| format!("{path}: {e}"))?;
    let json = if middle {
        let analise = contrato_simples::middle::analyze(&ast).map_err(|diagnosticos| {
            diagnosticos
                .iter()
                .map(|d| format!("{path}: {d}"))
                .collect::<Vec<_>>()
                .join("\n")
        })?;
        for aviso in &analise.avisos {
            eprintln!("{path}: {aviso}");
        }
        serde_json::to_string_pretty(&analise.ir)
    } else {
        serde_json::to_string_pretty(&ast)
    };
    println!("{}", json.map_err(|e| e.to_string())?);
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
