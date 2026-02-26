#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    let code = hash::run();
    std::process::ExitCode::from(code)
}
