use std::env;
use std::fs;
// use std::fs::File;
// use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use anyhow::{Context, Result};
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tokio;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct TemplateFile {
    name: Option<String>,
    source_url: String,
    output_path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GitHubItem {
    name: String,
    path: String,
    #[serde(rename = "type")]
    item_type: String,
    download_url: Option<String>,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
struct ProjectConfig {
    project_name: String,
    project_path: String,
    id: String,
    build_command: String,
    flash_command: String,
    install_components: Vec<String>,
}

enum LogType {
    Info,
    Warning,
    Error,
    Complete,
}
#[derive(Serialize)]
struct ProgressEvent<'a> {
    stage: &'a str,
    message: &'a str,
    current: u8,
    total: u8,
}

async fn download_file(git_url: &str, output_path: &Path) -> Result<()> {
    let content = reqwest::get(git_url).await?.text().await?;
    if let Some(parent) = output_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(output_path, content).await?;
    Ok(())
}

async fn load_files_json(path: &Path) -> Result<Vec<TemplateFile>> {
    let json_text = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read JSON file: {}", path.display()))?;

    let files: Vec<TemplateFile> =
        serde_json::from_str(&json_text).with_context(|| "Failed to parse files.json")?;

    Ok(files)
}

fn emit_progress(stage: &str, message: &str, current: u8, total: u8) {
    let event = ProgressEvent {
        stage,
        message,
        current,
        total,
    };

    let json = serde_json::to_string(&event).expect("Failed to serialize progress event");
    println!("\n");
    println!("__ESP_PROGRESS__:{}", json);
}

fn log(message: &str, milestone: &str, lt: LogType) {
    println!("\n");
    let text_for_log = format!(
        "[{}] - {}: {}",
        match lt {
            LogType::Info => "INFO",
            LogType::Warning => "WARNING",
            LogType::Error => "ERROR",
            LogType::Complete => "COMPLETE",
        },
        milestone,
        message
    );
    println!("{}", text_for_log);
}

// fn project_file_config_create(
//     project_path: &std::path::PathBuf,
//     project_name: &str,
// ) -> Option<ProjectConfig> {
//     // Create a new file named "project_config.toml" in the project directory
//     if project_path.join(".espConfig").exists() {
//         log(
//             &format!(
//                 "Project config folder already exists at: {}",
//                 project_path.join(".espConfig").display()
//             ),
//             "Project Config Creation",
//             LogType::Warning,
//         );
//         return None;
//     }
//     let config_path_file = project_path.join(".espConfig/esp_config.json");
//     fs::create_dir_all(project_path.join(".espConfig"))
//         .expect("Failed to create project config folder");
//     let _id = Uuid::new_v4();

//     let project_config = ProjectConfig {
//         project_name: project_name.to_string(),
//         project_path: project_path.display().to_string(),
//         id: Uuid::new_v4().to_string(),
//         build_command: "source ~/export-esp.sh && cargo build".to_string(),
//         flash_command: "cargo flash --monitor --port /dev/ttyUSB0".to_string(),
//         install_components: Vec::new(),
//     };
//     std::fs::write(
//         config_path_file,
//         format!(
//             "
//     {{
//         \"project_name\": \"{}\",
//         \"project_path\": \"{}\",
//         \"id\": \"{}\",
//         \"build_command\": \"source ~/export-esp.sh && cargo build\",
//         \"flash_command\": \"cargo flash --monitor --port /dev/ttyUSB0\",
//         \"install_components\": []
//     }}
//     ",
//             project_config.project_name, project_config.project_path, project_config.id
//         ),
//     )
//     .expect("Failed to create project config file");
//     log(
//         &format!(
//             "Project config file created at: {}",
//             project_path.join(".espConfig/esp_config.json").display()
//         ),
//         "Project Config Creation",
//         LogType::Info,
//     );
//     Some(project_config)
// }

fn load_project_database() -> Option<Vec<ProjectConfig>> {
    let projects: Vec<ProjectConfig> = Vec::new();

    let home_dir = dirs::home_dir().expect("Failed to get home directory");

    let db_path = home_dir.join("esp_rust_projects.json");

    if !db_path.exists() {
        log(
            "Project database file does not exist. Creating a new one.",
            "Project Database",
            LogType::Warning,
        );
        std::fs::write(&db_path, "[]").expect("Failed to create project database file");
        return Some(projects);
    }
    let db_content =
        std::fs::read_to_string(&db_path).expect("Failed to read project database file");
    let parsed_projects: Vec<ProjectConfig> =
        serde_json::from_str(&db_content).expect("Failed to parse project database file");
    Some(parsed_projects)
}

fn save_project_to_database(project_config: &ProjectConfig) {
    let mut projects = load_project_database().expect("Failed to load project database");
    projects.push(project_config.clone());
    let home_dir = dirs::home_dir().expect("Failed to get home directory");
    let db_path = home_dir.join("esp_rust_projects.json");
    std::fs::write(
        &db_path,
        serde_json::to_string_pretty(&projects).expect("Failed to serialize project database"),
    )
    .expect("Failed to write project database file");
    log(
        "Project saved to database successfully!",
        "Project Database",
        LogType::Info,
    );
}

async fn update_project_struct(project_path: &std::path::PathBuf, max_steps: u8) -> bool {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let files_json_path = manifest_path.join("src").join("files.json");

    let files: Vec<TemplateFile> = load_files_json(&files_json_path).await.unwrap();
    log(
        &format!("Updating project structure with {} files...", files.len()),
        "Project Structure Update",
        LogType::Info,
    );

    for (i, file) in files.iter().enumerate() {
        emit_progress(
            "Project Structure Update",
            &format!(
                "Processing file: {}",
                file.name.as_ref().unwrap_or(&file.output_path)
            ),
            (i + 1) as u8,
            max_steps,
        );
        let output_path = project_path.join(&file.output_path);

        log(
            &format!(
                "
            Processing file: {}
            file source: {}
            file output path: {}
            name: {}
            ",
                output_path.display(),
                file.source_url,
                output_path.display(),
                file.name.as_ref().unwrap_or(&file.output_path)
            ),
            "Project Structure Update",
            LogType::Info,
        );
        match download_file(&file.source_url, &output_path).await {
            Ok(_) => log(
                &format!("File created: {}", output_path.display()),
                "Project Structure Update",
                LogType::Info,
            ),
            Err(e) => log(
                &format!("Failed to create file {}: {}", output_path.display(), e),
                "Project Structure Update",
                LogType::Error,
            ),
        }
    }

    let dependency_to_add_list = vec![
        ("uuid", None),
        ("serde", Some("derive")),
        ("serde_json", None),
    ];

    for (dep, feature) in dependency_to_add_list {
        let mut cmd = Command::new("cargo");
        cmd.arg("add").arg(dep).current_dir(&project_path);
        if let Some(feature_name) = feature {
            cmd.arg("--features").arg(feature_name);
        }
        match cmd.status() {
            Ok(status) if status.success() => log(
                &format!("Dependency '{}' added successfully.", dep),
                "Project Structure Update",
                LogType::Info,
            ),
            Ok(status) => log(
                &format!(
                    "Failed to add dependency '{}'. Exit status: {}",
                    dep, status
                ),
                "Project Structure Update",
                LogType::Error,
            ),
            Err(e) => log(
                &format!("Failed to run cargo add for '{}': {}", dep, e),
                "Project Structure Update",
                LogType::Error,
            ),
        };
    }

    emit_progress(
        "Project Structure Update",
        "Updating Cargo.toml with dependencies...",
        (79) as u8,
        max_steps,
    );
    Command::new("cargo")
        .arg("add")
        .arg("serde_json")
        .current_dir(&project_path)
        .status()
        .expect("Failed to run cargo add");

    let cargo_toml_path = project_path.join("Cargo.toml");
    if cargo_toml_path.exists() {
        let mut cargo_toml_content =
            std::fs::read_to_string(&cargo_toml_path).expect("Failed to read Cargo.toml");
        // remove old dependencies if they exist {""}
        let marker = "[dependencies]";
        let dependency_to_add = r#"uuid = { version = "1.23.2", features = ["v4"] }"#;
        if let Some(index) = cargo_toml_content.find(marker) {
            // remove old dependencies if they exist {"uuid = "1.20.0"
            let old_dependency_line = cargo_toml_content
                .lines()
                .find(|line| line.contains(r#"uuid = "1.20.0"#));
            if let Some(old_line) = old_dependency_line {
                cargo_toml_content = cargo_toml_content.replace(old_line, "");
            }

            let insert_position = index + marker.len();
            cargo_toml_content.insert_str(insert_position, &format!("\n{}", dependency_to_add));
        }

        std::fs::write(&cargo_toml_path, cargo_toml_content).expect("Failed to update Cargo.toml");
        log(
            "Cargo.toml updated with ESP dependencies.",
            "Project Structure Update",
            LogType::Info,
        );
    }
    return true;
}

fn project_name_is_valid(project_name: &str) -> bool {
    if project_name.is_empty() || project_name.trim().is_empty() || project_name.contains(' ') {
        log(
            "Project name cannot be empty or contain spaces.",
            "Project Name Validation",
            LogType::Error,
        );
        return false;
    }
    if project_name.contains('/') || project_name.contains('\\') {
        log(
            "Project name cannot contain path separators.",
            "Project Name Validation",
            LogType::Error,
        );
        return false;
    }
    if project_name.contains('.') {
        log(
            "Project name cannot contain dots.",
            "Project Name Validation",
            LogType::Error,
        );
        return false;
    }
    if project_name.contains('-') {
        log(
            "Project name cannot contain hyphens.",
            "Project Name Validation",
            LogType::Error,
        );
        return false;
    }
    if project_name.len() > 100 {
        log(
            "Project name cannot be longer than 100 characters.",
            "Project Name Validation",
            LogType::Error,
        );
        return false;
    }
    if project_name.len() < 3 {
        log(
            "Project name cannot be shorter than 3 characters.",
            "Project Name Validation",
            LogType::Error,
        );
        return false;
    }
    true
}

fn project_file_valid(current_dir: &std::path::Path) -> bool {
    if !current_dir.join(".espConfig/esp_config.json").exists() {
        log(
            "Project config file does not exist.",
            "Project Validation",
            LogType::Error,
        );
        return false;
    }
    true
}
fn load_config(path: &std::path::PathBuf) -> Option<ProjectConfig> {
    let config_path_file = path.join(".espConfig/esp_config.json");
    if !config_path_file.exists() {
        return None;
    }
    let config_content =
        std::fs::read_to_string(&config_path_file).expect("Failed to read project config file");
    let config: ProjectConfig =
        serde_json::from_str(&config_content).expect("Failed to parse project config file");
    Some(config)
}
fn save_config(path: &std::path::PathBuf, config: &ProjectConfig) -> Option<ProjectConfig> {
    let config_path_file = path.join(".espConfig/esp_config.json");

    if let Some(parent) = config_path_file.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create config directory");
    }
    std::fs::write(
        &config_path_file,
        serde_json::to_string_pretty(config).expect("Failed to serialize project config"),
    )
    .expect("Failed to write project config file");
    log(
        "Project config file saved successfully!",
        "Project Config Save",
        LogType::Info,
    );
    Some(config.clone())
}
fn update_config_file_with_component(
    project_path: &std::path::PathBuf,
    component_name: &str,
) -> bool {
    let config_path = project_path.join(".espConfig/esp_config.json");
    if !config_path.exists() {
        log(
            "Project config file does not exist.",
            "Project Config Update",
            LogType::Error,
        );
        return false;
    }
    let mut config = load_config(project_path).expect("Failed to load project config");
    let install_components = &mut config.install_components;
    install_components.push(component_name.to_string());
    if save_config(project_path, &config).is_some() {
        log(
            &format!("Component '{}' added to project config.", component_name),
            "Project Config Update",
            LogType::Info,
        );
        return true;
    } else {
        log(
            "Failed to find 'install_components' in project config.",
            "Project Config Update",
            LogType::Error,
        );
        return false;
    }
}

fn create_config(path: &std::path::PathBuf, config: &ProjectConfig) -> Option<ProjectConfig> {
    let get_config = load_config(path);

    if get_config.is_none() {
        log(
            "Project config file does not exist. Creating a new one.",
            "--Project Config Creation--",
            LogType::Info,
        );
        let saved_config = save_config(path, config);
        if saved_config.is_some() {
            return saved_config;
        } else {
            log(
                "Failed to create project config file.",
                "--Project Config Creation--",
                LogType::Error,
            );
            return None;
        }
    }
    return None;
}

async fn load_all_modules() -> Result<Vec<GitHubItem>, String> {
    let url = "https://api.github.com/repos/Adeun-Ilemobola/rust_esp32_based/contents/src/module?ref=main";
    let client = reqwest::Client::new();
    let response: Vec<GitHubItem> = client
        .get(url)
        .header(USER_AGENT, "Mozilla/5.0 (compatible; MyRustApp/1.0)")
        .header(ACCEPT, "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|err| err.to_string())?
        .json::<Vec<GitHubItem>>()
        .await
        .map_err(|err| err.to_string())?;

    let modules: Vec<GitHubItem> = response
        .into_iter()
        .filter(|item| {
            item.item_type == "file"
                && item.name.ends_with(".rs")
                && item.name != "mod.rs"
                && item.name != "ModuleCore.rs"
        })
        .collect();

    Ok(modules)
}

async fn install_component(component_name: &str) {
    // (1) check if the component exists in the module registry (github repo)
    let modules = load_all_modules().await;
    let get_config =
        load_config(&std::env::current_dir().expect("Failed to get current directory"));

    if get_config.is_none() {
        log(
            "Project config file does not exist in the current directory. Please provide a valid project path or create a project first.",
            "Component Installation",
            LogType::Error,
        );
        return;
    }

    match modules {
        Ok(modules) => {
            println!(
                "Loaded modules from registry: {:?}",
                modules.iter().map(|m| &m.name).collect::<Vec<&String>>()
            );
            if let Some(component) = modules
                .into_iter()
                .find(|m| m.name.to_lowercase() == component_name.to_lowercase())
            {
                log(
                    &format!("Component '{}' found in registry.", component_name),
                    "Component Installation",
                    LogType::Info,
                );
                let is_valid_project = project_file_valid(
                    &std::env::current_dir().expect("Failed to get current directory"),
                );
                if !is_valid_project {
                    log(
                        "Current directory is not a valid project. Please navigate to a valid project directory and try again.",
                        "Component Installation",
                        LogType::Error,
                    );
                    return;
                }
                let soc = &format!(
                    "https://raw.githubusercontent.com/Adeun-Ilemobola/rust_esp32_based/refs/heads/main/src/module/{}",
                    component.name
                );
                let module_folder_path = std::env::current_dir()
                    .expect("Failed to get current directory")
                    .join("src")
                    .join("module");
                let output_path = module_folder_path.join(&component.name);
                match download_file(soc, &output_path).await {
                    Ok(_) => {
                        log(
                            &format!(
                                "Component '{}' installed successfully at {}.",
                                component_name,
                                output_path.display()
                            ),
                            "Component Installation",
                            LogType::Info,
                        );
                        let mod_file = module_folder_path.join("mod.rs");
                        let installed_components = &get_config.unwrap().install_components;
                        
                        let mod_contents = installed_components
                            .iter()
                            .chain(std::iter::once(&component_name.replace(".rs", "")))
                            .map(|c| format!("pub mod {};", c.to_lowercase()))
                            .collect::<Vec<String>>()
                            .join("\n");

                        fs::write(&mod_file, mod_contents).expect("Failed to write to mod.rs");
                    }
                    Err(e) => log(
                        &format!("Failed to install component '{}': {}", component_name, e),
                        "Component Installation",
                        LogType::Error,
                    ),
                }
                let update_success = update_config_file_with_component(
                    &std::env::current_dir().expect("Failed to get current directory"),
                    component_name,
                );
                if !update_success {
                    log(
                        &format!(
                            "Failed to update project config with component '{}'.",
                            component_name
                        ),
                        "Component Installation",
                        LogType::Error,
                    );
                }
                // Proceed with installation
            } else {
                log(
                    &format!("Component '{}' not found in registry.", component_name),
                    "Component Installation",
                    LogType::Error,
                );
                return;
            }
        }
        Err(e) => {
            log(
                &format!("Failed to load module registry: {}", e),
                "Component Installation",
                LogType::Error,
            );
            return;
        }
    }
}

async fn create_project(
    current_dir: &std::path::PathBuf,
    project_name: &str,
) -> Option<ProjectConfig> {
    let total_steps = 99;
    emit_progress(
        "Project Creation",
        "Validating project name...",
        1,
        total_steps,
    );
    let project_dir = current_dir.join(&project_name);
    // let build_script = "source ~/export-esp.sh && cargo build";

    if project_dir.exists() {
        log(
            &format!(
                "Project directory already exists at: {}",
                project_dir.display()
            ),
            "Project Creation",
            LogType::Warning,
        );
        return None;
    }
    emit_progress(
        "Project Creation",
        "Generating project files...",
        2,
        total_steps,
    );
    // (2) run "cargo generate esp-rs/esp-idf-template {project_name}"
    let gen_status = Command::new("cargo")
        .current_dir(&current_dir)
        .arg("generate")
        .arg("esp-rs/esp-idf-template")
        .arg("cargo")
        .arg("--name")
        .arg(&project_name)
        .arg("-d")
        .arg("mcu=esp32")
        .arg("-d")
        .arg("advanced=false")
        .status()
        .expect("Failed to run cargo generate");

    if gen_status.success() {
        emit_progress(
            "Project Creation",
            "Creating project config file...",
            3,
            total_steps,
        );
        let project_identifier = create_config(
            &project_dir,
            &ProjectConfig {
                project_name: project_name.to_string(),
                project_path: project_dir.display().to_string(),
                id: Uuid::new_v4().to_string(),
                build_command: "source ~/export-esp.sh && cargo build".to_string(),
                flash_command: "cargo flash --monitor --port /dev/ttyUSB0".to_string(),
                install_components: Vec::new(),
            },
        );

        emit_progress(
            "Project Creation",
            "Creating project config file...",
            4,
            total_steps,
        );
        if project_identifier.is_some() {
            let config_data = project_identifier.as_ref().unwrap();
            log(
                "Project config file created successfully!",
                "Project Config Creation",
                LogType::Complete,
            );

            if update_project_struct(&project_dir, total_steps).await {
                save_project_to_database(&config_data);
                emit_progress(
                    "Project Creation",
                    "Project creation completed.",
                    total_steps,
                    total_steps,
                );
                return Some(config_data.clone());
            } else {
                log(
                    "Failed to update project structure.",
                    "Project Structure Update",
                    LogType::Error,
                );
                return None;
            }
        } else {
            log(
                "Failed to create project config file.",
                "Project Config Creation",
                LogType::Error,
            );
            return None;
        }
    } else {
        log(
            "Failed to generate project.",
            "Project Generation",
            LogType::Error,
        );
        return None;
    }
}

fn build_project() {
    let get_config =
        load_config(&std::env::current_dir().expect("Failed to get current directory"));
    if get_config.is_none() {
        log(
            "Project config file does not exist in the current directory. Please provide a valid project path or create a project first.",
            "Project Validation",
            LogType::Error,
        );
        return;
    }
    let config = get_config.unwrap();
    let build_script = &config.build_command;
    Command::new("sh")
        .arg("-c")
        .arg(build_script)
        .current_dir(&config.project_path)
        .status()
        .expect("Failed to run build script");
}

fn run_project_flash() {
    let get_config =
        load_config(&std::env::current_dir().expect("Failed to get current directory"));
    if get_config.is_none() {
        log(
            "Project config file does not exist in the current directory. Please provide a valid project path or create a project first.",
            "Project Validation",
            LogType::Error,
        );
        return;
    }
    let config = get_config.unwrap();
    Command::new("cargo")
        .arg("flash")
        .arg("--monitor")
        .arg("--port ")
        .arg("/dev/cu.usbserial-0001 ")
        .current_dir(format!(
            "target/xtensa-esp32-espidf/debug/{}",
            config.project_name
        ))
        .status()
        .expect("Failed to run cargo flash");
}

#[tokio::main]
async fn main() {
    // get the current directory
    let current_dir = std::env::current_dir().expect("Failed to get current directory");

    let args: Vec<String> = env::args().collect();
    /*
     ["project", "create" , "project_name" , '--path' , 'path/to/project']
     ["project", "run"  ]
     ["project" , "build" ]
     ["project", "help"]
     ["project", "add" , "{component_name}"]
     ["project", "remove" , "{component_name}"]
     ["project", "update" , "{component_name}"]
     ["project", "listcomponents"]

    */
    if args.len() < 2 {
        log(
            "Please provide a command.",
            "Command Validation",
            LogType::Error,
        );
        log("Example: project run", "Command Validation", LogType::Info);
        return;
    }

    let command = &args[1];

    match command.as_str() {
        "create" => {
            if args.len() < 3 {
                log(
                    "Please provide a project name.",
                    "Command Validation",
                    LogType::Error,
                );
                return;
            }
            let project_name = args[2].clone();

            let all_projects = load_project_database().expect("Failed to load project database");
            if all_projects.iter().any(|p| p.project_name == project_name) {
                log(
                    "A project with this name already exists in the database. Please choose a different name.",
                    "Project Name Validation",
                    LogType::Error,
                );
                return;
            }

            if !project_name_is_valid(&project_name) {
                return;
            }
            if args.len() >= 5 && args[3] == "--path" {
                let custom_path = std::path::PathBuf::from(&args[4]);
                log(
                    &format!("the path: {}", custom_path.display()),
                    "Path Validation",
                    LogType::Info,
                );
                if !custom_path.exists() || !custom_path.is_dir() {
                    log(
                        "Provided path does not exist or is not a directory.",
                        "Path Validation",
                        LogType::Error,
                    );
                    return;
                }
                let project_path = custom_path.join(&project_name);
                if project_path.exists() {
                    log(
                        "Project already exists at the provided path.",
                        "Project Creation",
                        LogType::Error,
                    );
                    return;
                }
                match create_project(&custom_path, &project_name).await {
                    Some(config_data) => {
                        log(
                            &format!(
                                "Project '{}' created successfully at {}!",
                                config_data.project_name,
                                project_path.display()
                            ),
                            "Project Creation",
                            LogType::Info,
                        );
                    }
                    None => {
                        log(
                            "Failed to create project.",
                            "Project Creation",
                            LogType::Error,
                        );
                    }
                }
            } else if args.len() >= 3 {
                log(
                    &format!("the path: {}", current_dir.display()),
                    "Path Validation",
                    LogType::Info,
                );
                match create_project(&current_dir, &project_name).await {
                    Some(config_data) => {
                        log(
                            &format!(
                                "Project '{}' created successfully at {}!",
                                config_data.project_name,
                                current_dir.join(&config_data.project_name).display()
                            ),
                            "Project Creation",
                            LogType::Info,
                        );
                    }
                    None => {
                        log(
                            "Failed to create project.",
                            "Project Creation",
                            LogType::Error,
                        );
                    }
                }
            } else {
                log(
                    "Invalid command format. Please provide a project name and optionally a path.",
                    "Command Validation",
                    LogType::Error,
                );
                log(
                    "Example: project create my_project --path /path/to/projects",
                    "Command Validation",
                    LogType::Info,
                );
                return;
            }
        }
        "run" => {
            build_project();
            run_project_flash();
        }

        "add" => {
            if args.len() < 3 {
                log(
                    "Please provide a component name to install.",
                    "Command Validation",
                    LogType::Error,
                );
                return;
            }
            let mut component_name = args[2].clone();
            if !component_name.ends_with(".rs") {
                component_name = format!("{}.rs", &component_name);
            }
            install_component(&component_name).await;
        }

        "build" => {
            build_project();
        }
        "help" => {
            println!("Available commands:");
            println!("create <project_name> - Create a new project with the specified name.");
            println!("run - Flash the project to the ESP device.");
            println!("build - Build the project.");
            println!("help - Show this help message.");
            println!("install <component_name> - Install a component.");
            println!("uninstall <component_name> - Uninstall a component.");
            println!("update <component_name> - Update a component.");
            println!("listcomponents - List all available components.");
        }

        _ => {
            println!("Unknown command: {}", command);
            println!("Available commands: create, run, build, help");
        }
    }
}
