use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    let [path] = arguments.as_slice() else {
        eprintln!("usage: validate_evidence <performance.json>");
        return ExitCode::from(2);
    };
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("read performance evidence: {error}");
            return ExitCode::from(2);
        }
    };
    match rz0_performance_contract::decode_performance_evidence(&bytes) {
        Ok(evidence) => {
            println!(
                "valid {} {:?} authorized={}",
                evidence.evidence_id, evidence.decision, evidence.release_authorized
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("invalid performance evidence: {error}");
            ExitCode::from(2)
        }
    }
}
