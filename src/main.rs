use std::env;
use std::fs;
// use std::fs::File;
// use std::io::Write;
// use std::path::Path;
use std::process::Command;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Clone , Deserialize)]
struct ProjectConfig {
    project_name: String,
    project_path: String,
    id: String,
    build_command: String,
    flash_command: String,
}

enum LogType {
    Info,
    Warning,
    Error,
    Complete,
}

fn log(message: &str, milestone: &str, lt: LogType) {
    let text_for_log = format!(
        "[{}] - {}: {}",
        milestone,
        match lt {
            LogType::Info => "INFO",
            LogType::Warning => "WARNING",
            LogType::Error => "ERROR",
            LogType::Complete => "COMPLETE",
        },
        message
    );
    println!("{}", text_for_log);
}

fn flash_esp(project_name: &str) {
    // (3) run "cargo flash -p /dev/ttyUSB0" at  "target/xtensa-esp32-espidf/debug/{project_name}"
    Command::new("cargo")
        .arg("flash")
        .arg("--monitor")
        .arg("--port ")
        .arg("/dev/cu.usbserial-0001 ")
        .current_dir(format!("target/xtensa-esp32-espidf/debug/{}", project_name))
        .status()
        .expect("Failed to run cargo flash");
}

fn project_file_config_create(project_path: &std::path::PathBuf, project_name: &str) -> Option<ProjectConfig> {
    // Create a new file named "project_config.toml" in the project directory
    if project_path.join(".espConfig").exists() {
        log(
            &format!(
                "Project config folder already exists at: {}",
                project_path.join(".espConfig").display()
            ),
            "Project Config Creation",
            LogType::Warning,
        );
        return None;
    }
    let config_path_file = project_path.join(".espConfig/esp_config.json");
    fs::create_dir_all(project_path.join(".espConfig"))
        .expect("Failed to create project config folder");
    let _id = Uuid::new_v4();

    let project_config = ProjectConfig {
        project_name: project_name.to_string(),
        project_path: project_path.display().to_string(),
        id: Uuid::new_v4().to_string(),
        build_command: "source ~/export-esp.sh && cargo build".to_string(),
        flash_command: "cargo flash --monitor --port /dev/ttyUSB0".to_string(),
    };
    std::fs::write(
        config_path_file,
        format!(
            "
    {{
        \"project_name\": \"{}\",
        \"project_path\": \"{}\",
        \"id\": \"{}\",
        \"build_command\": \"source ~/export-esp.sh && cargo build\",
        \"flash_command\": \"cargo flash --monitor --port /dev/ttyUSB0\"
    }}
    ",
            project_config.project_name,
            project_config.project_path,
            project_config.id
        ),
    )
    .expect("Failed to create project config file");
    log(
        &format!(
            "Project config file created at: {}",
            project_path.join(".espConfig/esp_config.json").display()
        ),
        "Project Config Creation",
        LogType::Info,
    );
    Some(project_config)
}

fn load_project_database()->Option<Vec<ProjectConfig>>{
    let mut projects: Vec<ProjectConfig> = Vec::new();

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
    let db_content = std::fs::read_to_string(&db_path).expect("Failed to read project database file");
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
        serde_json::to_string_pretty(&projects).expect("Failed to serialize project database")
    )
        .expect("Failed to write project database file");
    log(
        "Project saved to database successfully!",
        "Project Database",
        LogType::Info,
    );
}


fn create_project(current_dir: &std::path::PathBuf, project_name: &str) -> Option<ProjectConfig> {
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

    // (2) run "cargo generate esp-rs/esp-idf-template {project_name}"
    let gen_status = Command::new("cargo")
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
        let project_identifier = project_file_config_create(&project_dir, &project_name);
        if project_identifier.is_some() {
            let config_data = project_identifier.as_ref().unwrap();
            log(
                "Project config file created successfully!",
                "Project Config Creation",
                LogType::Complete,
            );
            save_project_to_database(&config_data);
            return Some(config_data.clone());
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

fn main() {
    // get the current directory
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let mut project_name: String = current_dir
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let args: Vec<String> = env::args().collect();
    /*
     ["project", "create" , "project_name" , '--path' , 'path/to/project']
     ["project", "run" ,'--path' , 'path/to/project' ]
     ["project" , "build" ,'--path' , 'path/to/project' ]
     ["project", "help"]
     ["project", "install" , "{component_name}"]
     ["project", "uninstall" , "{component_name}"]
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
            project_name = args[2].clone();

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
                match create_project(&custom_path, &project_name) {
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
            } else {
                match create_project(&current_dir, &project_name) {
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
            }
        }
        "run" => {
            if args.len() >= 4 && args[2] == "--path" {
                let custom_path = std::path::PathBuf::from(&args[3]);
                if !custom_path.exists() || !custom_path.is_dir() {
                    log(
                        "Provided path does not exist or is not a directory.",
                        "Path Validation",
                        LogType::Error,
                    );
                    return;
                }
                let project_path = custom_path.join(&project_name);
                if !project_path.exists() || !project_path.is_dir() {
                    log(
                        "Project does not exist at the provided path.",
                        "Project Validation",
                        LogType::Error,
                    );
                    return;
                }
                flash_esp(&project_name);
            } else {
                if !current_dir.join(".espConfig/esp_config.json").exists() {
                    log(
                        "Project config file does not exist in the current directory. Please provide a valid project path or create a project first.",
                        "Project Validation",
                        LogType::Error,
                    );
                    return;
                }
                flash_esp(&project_name);
            }
        }

        "build" => {
            let build_script = "source ~/export-esp.sh && cargo build";
            let mut project_path =
                std::env::current_dir().expect("Failed to get current directory");
            if args.len() >= 4 && args[2] == "--path" {
                let custom_path = std::path::PathBuf::from(&args[3]);
                if !custom_path.exists() || !custom_path.is_dir() {
                    log(
                        "Provided path does not exist or is not a directory.",
                        "Path Validation",
                        LogType::Error,
                    );
                    return;
                }
                project_path = custom_path.join(&project_name);
                if !project_path.exists() || !project_path.is_dir() {
                    log(
                        "Project does not exist at the provided path.",
                        "Project Validation",
                        LogType::Error,
                    );
                    return;
                }
            } else {
                if !project_path.join(".espConfig/esp_config.json").exists() {
                    log(
                        "Project config file does not exist in the current directory. Please provide a valid project path or create a project first.",
                        "Project Validation",
                        LogType::Error,
                    );
                    return;
                }
            }

            let project_exists = project_file_valid(&project_path);

            if project_exists {
                // Implement build functionality here

                let build_status = Command::new("sh")
                    .arg("-c")
                    .arg(build_script)
                    .current_dir(&project_path)
                    .status()
                    .expect("Failed to run build script");
                if build_status.success() {
                    log(
                        "Project built successfully!",
                        "Project Build",
                        LogType::Info,
                    );
                } else {
                    log("Failed to build project.", "Project Build", LogType::Error);
                }
            } else {
                log(
                    "Project does not exist. Please create a project first.",
                    "Project Build",
                    LogType::Error,
                );
            }
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
