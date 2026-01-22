//! User input handling for the unified command

use crate::console::CliConsole;
use sage_core::input::{InputChannelHandle, InputRequestKind, InputResponse};
use std::io::Write;

/// Handle user input requests from the execution loop
pub async fn handle_user_input(mut handle: InputChannelHandle, verbose: bool) {
    let console = CliConsole::new(verbose);
    while let Some(request) = handle.request_rx.recv().await {
        // Display the question based on request kind
        console.print_header("User Input Required");

        match &request.kind {
            InputRequestKind::Questions { questions } => {
                for question in questions {
                    println!("{}", question.question);
                    for (idx, opt) in question.options.iter().enumerate() {
                        println!("  {}. {}: {}", idx + 1, opt.label, opt.description);
                    }
                }
            }
            InputRequestKind::Permission {
                tool_name,
                description,
                ..
            } => {
                println!("Permission required for tool: {}", tool_name);
                println!("{}", description);
                println!("Enter 'yes' or 'y' to allow, 'no' or 'n' to deny:");
            }
            InputRequestKind::FreeText { prompt, .. } => {
                println!("{}", prompt);
            }
            InputRequestKind::Simple {
                question, options, ..
            } => {
                println!("{}", question);
                if let Some(opts) = options {
                    for (idx, opt) in opts.iter().enumerate() {
                        println!("  {}. {}: {}", idx + 1, opt.label, opt.description);
                    }
                }
            }
        }

        // Read user input using async stdin to avoid blocking the async runtime
        print!("> ");
        let _ = std::io::stdout().flush();

        // Use tokio's blocking task for stdin since std::io::stdin is blocking
        let input_result = tokio::task::spawn_blocking(|| {
            let mut input = String::new();
            match std::io::stdin().read_line(&mut input) {
                Ok(_) => Some(input),
                Err(_) => None,
            }
        })
        .await;

        match input_result {
            Ok(Some(input)) => {
                let content = input.trim().to_string();

                // Check for cancel keywords
                let cancelled = content.to_lowercase() == "cancel"
                    || content.to_lowercase() == "quit"
                    || content.to_lowercase() == "exit";

                let response = if cancelled {
                    InputResponse::cancelled(request.id)
                } else {
                    // Handle permission responses specially
                    if matches!(&request.kind, InputRequestKind::Permission { .. }) {
                        let lower = content.to_lowercase();
                        if lower == "yes" || lower == "y" {
                            InputResponse::permission_granted(request.id)
                        } else if lower == "no" || lower == "n" {
                            InputResponse::permission_denied(
                                request.id,
                                Some("User denied".to_string()),
                            )
                        } else {
                            InputResponse::text(request.id, content)
                        }
                    } else {
                        InputResponse::text(request.id, content)
                    }
                };

                if let Err(e) = handle.respond(response).await {
                    eprintln!("Failed to send response: {}", e);
                    break;
                }
            }
            _ => {
                // EOF or error - send cancelled
                let _ = handle.respond(InputResponse::cancelled(request.id)).await;
                break;
            }
        }
    }
}
