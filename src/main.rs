#![forbid(unsafe_code)]

#[tokio::main(flavor = "multi_thread")]
async fn main() -> std::process::ExitCode {
    match intergalactic_warfare::platform::native::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Intergalactic Warfare startup failed: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
