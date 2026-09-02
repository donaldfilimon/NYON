#![forbid(unsafe_code)]

#[tokio::main(flavor = "multi_thread")]
async fn main() -> std::process::ExitCode {
    match nyon::platform::native::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("NYON startup failed: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
