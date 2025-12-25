//! Docker command execution logic

use anyhow::{Result, Context};
use tokio::process::Command;
use tracing::debug;

use super::types::DockerOperation;

/// Execute a docker command
pub async fn execute_docker_command(args: &[&str], working_dir: Option<&str>) -> Result<String> {
    let mut cmd = Command::new("docker");
    cmd.args(args);

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    debug!("Executing docker command: docker {}", args.join(" "));

    let output = cmd.output().await
        .with_context(|| format!("Failed to execute docker command: docker {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Docker command failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.to_string())
}

/// Execute a docker-compose command
pub async fn execute_compose_command(args: &[&str], working_dir: Option<&str>) -> Result<String> {
    let mut cmd = Command::new("docker-compose");
    cmd.args(args);

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    debug!("Executing docker-compose command: docker-compose {}", args.join(" "));

    let output = cmd.output().await
        .with_context(|| format!("Failed to execute docker-compose command: docker-compose {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Docker Compose command failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.to_string())
}

/// Handle container operations
pub async fn handle_container_operation(operation: &DockerOperation, working_dir: Option<&str>) -> Result<String> {
    match operation {
        DockerOperation::ListContainers { all } => {
            let mut args = vec!["ps"];
            if *all {
                args.push("-a");
            }
            execute_docker_command(&args, working_dir).await
        }
        DockerOperation::RunContainer { image, name, ports, volumes, environment, detach, remove, command } => {
            let mut args = vec!["run"];

            if *detach {
                args.push("-d");
            }
            if *remove {
                args.push("--rm");
            }

            if let Some(name) = name {
                args.extend(vec!["--name", name]);
            }

            if let Some(ports) = ports {
                for port in ports {
                    args.extend(vec!["-p", port]);
                }
            }

            if let Some(volumes) = volumes {
                for volume in volumes {
                    args.extend(vec!["-v", volume]);
                }
            }

            if let Some(env) = environment {
                for (key, value) in env {
                    args.extend(vec!["-e", &format!("{}={}", key, value)]);
                }
            }

            args.push(image);

            if let Some(cmd) = command {
                args.extend(cmd.split_whitespace());
            }

            execute_docker_command(&args, working_dir).await
        }
        DockerOperation::StopContainer { container } => {
            execute_docker_command(&["stop", container], working_dir).await
        }
        DockerOperation::StartContainer { container } => {
            execute_docker_command(&["start", container], working_dir).await
        }
        DockerOperation::RemoveContainer { container, force } => {
            let mut args = vec!["rm"];
            if *force {
                args.push("-f");
            }
            args.push(container);
            execute_docker_command(&args, working_dir).await
        }
        DockerOperation::ContainerLogs { container, follow, tail } => {
            let mut args = vec!["logs"];
            if *follow {
                args.push("-f");
            }
            if let Some(tail_lines) = tail {
                args.extend(vec!["--tail", &tail_lines.to_string()]);
            }
            args.push(container);
            execute_docker_command(&args, working_dir).await
        }
        DockerOperation::Exec { container, command, interactive } => {
            let mut args = vec!["exec"];
            if *interactive {
                args.push("-it");
            }
            args.push(container);
            args.extend(command.split_whitespace());
            execute_docker_command(&args, working_dir).await
        }
        DockerOperation::InspectContainer { container } => {
            execute_docker_command(&["inspect", container], working_dir).await
        }
        _ => Err(anyhow::anyhow!("Invalid container operation")),
    }
}

/// Handle image operations
pub async fn handle_image_operation(operation: &DockerOperation, working_dir: Option<&str>) -> Result<String> {
    match operation {
        DockerOperation::ListImages { all } => {
            let mut args = vec!["images"];
            if *all {
                args.push("-a");
            }
            execute_docker_command(&args, working_dir).await
        }
        DockerOperation::BuildImage { dockerfile_path, tag, context, build_args } => {
            let mut args = vec!["build"];
            args.extend(vec!["-t", tag]);

            if let Some(build_args) = build_args {
                for (key, value) in build_args {
                    args.extend(vec!["--build-arg", &format!("{}={}", key, value)]);
                }
            }

            args.extend(vec!["-f", dockerfile_path]);

            let build_context = context.as_deref().unwrap_or(".");
            args.push(build_context);

            execute_docker_command(&args, working_dir).await
        }
        DockerOperation::PullImage { image } => {
            execute_docker_command(&["pull", image], working_dir).await
        }
        DockerOperation::PushImage { image } => {
            execute_docker_command(&["push", image], working_dir).await
        }
        DockerOperation::RemoveImage { image, force } => {
            let mut args = vec!["rmi"];
            if *force {
                args.push("-f");
            }
            args.push(image);
            execute_docker_command(&args, working_dir).await
        }
        DockerOperation::TagImage { source, target } => {
            execute_docker_command(&["tag", source, target], working_dir).await
        }
        _ => Err(anyhow::anyhow!("Invalid image operation")),
    }
}

/// Handle volume operations
pub async fn handle_volume_operation(operation: &DockerOperation, working_dir: Option<&str>) -> Result<String> {
    match operation {
        DockerOperation::ListVolumes => {
            execute_docker_command(&["volume", "ls"], working_dir).await
        }
        DockerOperation::CreateVolume { name } => {
            execute_docker_command(&["volume", "create", name], working_dir).await
        }
        DockerOperation::RemoveVolume { name } => {
            execute_docker_command(&["volume", "rm", name], working_dir).await
        }
        _ => Err(anyhow::anyhow!("Invalid volume operation")),
    }
}

/// Handle network operations
pub async fn handle_network_operation(operation: &DockerOperation, working_dir: Option<&str>) -> Result<String> {
    match operation {
        DockerOperation::ListNetworks => {
            execute_docker_command(&["network", "ls"], working_dir).await
        }
        DockerOperation::CreateNetwork { name, driver } => {
            let mut args = vec!["network", "create"];
            if let Some(driver) = driver {
                args.extend(vec!["--driver", driver]);
            }
            args.push(name);
            execute_docker_command(&args, working_dir).await
        }
        DockerOperation::RemoveNetwork { name } => {
            execute_docker_command(&["network", "rm", name], working_dir).await
        }
        _ => Err(anyhow::anyhow!("Invalid network operation")),
    }
}

/// Handle system operations
pub async fn handle_system_operation(operation: &DockerOperation, working_dir: Option<&str>) -> Result<String> {
    match operation {
        DockerOperation::SystemInfo => {
            execute_docker_command(&["system", "info"], working_dir).await
        }
        DockerOperation::SystemPrune { volumes } => {
            let mut args = vec!["system", "prune", "-f"];
            if *volumes {
                args.push("--volumes");
            }
            execute_docker_command(&args, working_dir).await
        }
        _ => Err(anyhow::anyhow!("Invalid system operation")),
    }
}

/// Handle Docker Compose operations
pub async fn handle_compose_operation(operation: &DockerOperation, working_dir: Option<&str>) -> Result<String> {
    match operation {
        DockerOperation::ComposeUp { file, detach } => {
            let mut args = vec![];
            if let Some(file) = file {
                args.extend(vec!["-f", file]);
            }
            args.push("up");
            if *detach {
                args.push("-d");
            }
            execute_compose_command(&args, working_dir).await
        }
        DockerOperation::ComposeDown { file, volumes } => {
            let mut args = vec![];
            if let Some(file) = file {
                args.extend(vec!["-f", file]);
            }
            args.push("down");
            if *volumes {
                args.push("--volumes");
            }
            execute_compose_command(&args, working_dir).await
        }
        DockerOperation::ComposeLogs { file, follow } => {
            let mut args = vec![];
            if let Some(file) = file {
                args.extend(vec!["-f", file]);
            }
            args.push("logs");
            if *follow {
                args.push("-f");
            }
            execute_compose_command(&args, working_dir).await
        }
        _ => Err(anyhow::anyhow!("Invalid compose operation")),
    }
}
