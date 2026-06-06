use futures::StreamExt;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();

    let args: Vec<String> = std::env::args().collect();
    let code = run(args).await;
    std::process::exit(code);
}

async fn run(args: Vec<String>) -> i32 {
    let mut stdout = tokio::io::stdout();
    match objectiveai_cli::run(args, None).await {
        Ok(mut stream) => {
            let mut last_tool_exit: Option<i32> = None;
            while let Some(item) = stream.next().await {
                match item {
                    Ok(run_item) => {
                        write_line(&mut stdout, &run_item).await;
                    }
                    Err(e) => {
                        // `ToolExit(code)` carries the exit code the
                        // upstream tool exited with — propagate it
                        // even though it's surfaced as an Err item.
                        if let objectiveai_cli::error::Error::ToolExit(code) = &e {
                            last_tool_exit = Some(*code);
                        }
                        write_error_line(&mut stdout, &e, None).await;
                    }
                }
            }
            last_tool_exit.unwrap_or(0)
        }
        Err(e) => {
            // `--help` / `--version` / no-subcommand → render the
            // clap output as a single informational line. Exit 0 so
            // scripts pipelining `--help` aren't penalised.
            if let objectiveai_cli::error::Error::ClapParse(ref clap_err) = e {
                if objectiveai_cli::is_informational(clap_err) {
                    write_help_line(&mut stdout, &clap_err.to_string()).await;
                    return 0;
                }
            }
            write_error_line(&mut stdout, &e, Some(true)).await;
            match e {
                objectiveai_cli::error::Error::ToolExit(code) => code,
                _ => 1,
            }
        }
    }
}

async fn write_line<T: serde::Serialize>(
    stdout: &mut tokio::io::Stdout,
    value: &T,
) {
    let line = match serde_json::to_string(value) {
        Ok(s) => s,
        Err(e) => format!(
            r#"{{"type":"error","fatal":false,"message":"serialize error: {e}"}}"#
        ),
    };
    let _ = stdout.write_all(line.as_bytes()).await;
    let _ = stdout.write_all(b"\n").await;
    let _ = stdout.flush().await;
}

async fn write_error_line(
    stdout: &mut tokio::io::Stdout,
    e: &objectiveai_cli::error::Error,
    fatal: Option<bool>,
) {
    let payload = objectiveai_sdk::cli::Error {
        r#type: objectiveai_sdk::cli::ErrorType::Error,
        level: Some(objectiveai_sdk::cli::Level::Error),
        fatal,
        message: e.output_message(),
    };
    write_line(stdout, &payload).await;
}

async fn write_help_line(stdout: &mut tokio::io::Stdout, help: &str) {
    let payload = serde_json::json!({
        "type": "help",
        "help": help,
    });
    write_line(stdout, &payload).await;
}
