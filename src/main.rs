use std::env;
use std::process;

fn main() {
    let (code, stdout, stderr) = runtime_zero::run(env::args().skip(1));

    if !stdout.is_empty() {
        print!("{stdout}");
    }

    if !stderr.is_empty() {
        eprint!("{stderr}");
    }

    process::exit(code.as_i32());
}
