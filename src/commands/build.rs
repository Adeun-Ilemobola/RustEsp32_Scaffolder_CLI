
use crate::sharedTypes::{LogType, ProgressType};
use crate::utility::{log, progress_log};
use crate::projectConfig::load_config;
use std::process::Command;

const BUILD_ID: &str = "build-esp";

pub fn build_esp() -> bool {
    progress_log(
        ProgressType::Loading,
        "Loading project config".to_string(),
        BUILD_ID.to_string(),
    );

    let get_config =
        load_config(&std::env::current_dir().expect("Failed to get current directory"));
    if get_config.is_none() {
        progress_log(
            ProgressType::Error,
            "Project config file does not exist. Create a project first or navigate to a valid project directory.".to_string(),
            BUILD_ID.to_string(),
        );
        log(
            "Project config file does not exist in the current directory. Please provide a valid project path or create a project first.",
            "Project Validation",
            LogType::Error,
        );
        return false;
    }
    let config = get_config.unwrap();

    progress_log(
        ProgressType::Loading,
        format!("Running build command for '{}'", config.project_name),
        BUILD_ID.to_string(),
    );

    let build_script = &config.build_command;
    let status = Command::new("bash")
        .arg("-lc")
        .arg(build_script)
        .current_dir(&config.project_path)
        .status();

    match status {
        Ok(status) if status.success() => {
            progress_log(
                ProgressType::Finished,
                format!("Build succeeded for '{}'", config.project_name),
                BUILD_ID.to_string(),
            );
            true
        }
        Ok(status) => {
            progress_log(
                ProgressType::Error,
                format!("Build failed with exit status: {}", status),
                BUILD_ID.to_string(),
            );
            log(
                &format!("Build failed with exit status: {}", status),
                "Build",
                LogType::Error,
            );
            false
        }
        Err(e) => {
            progress_log(
                ProgressType::Error,
                format!("Failed to run build command: {}", e),
                BUILD_ID.to_string(),
            );
            log(
                &format!("Failed to run build command: {}", e),
                "Build",
                LogType::Error,
            );
            false
        }
    }
}
