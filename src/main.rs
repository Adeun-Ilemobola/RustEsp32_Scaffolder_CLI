use std::env;
use std::fs;
// use std::fs::File;
// use std::io::Write;
// use std::path::Path;
use uuid::Uuid;

use std::process::Command;

enum log_Type {
    Info,
    Warning,
    Error,  
}

fn milestone_log(message: &str , milestone: &str , log_Type: log_Type) {
    println!("=============={}================", 
    match log_Type {
        log_Type::Info => "INFO",
        log_Type::Warning => "WARNING",
        log_Type::Error => "ERROR",
    });
    println!("Milestone: {}", milestone);
    println!("{}", message);
    println!("==============================");
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

fn project_file_config_create(project_path: &std::path::PathBuf, project_name: &str) -> Option<()> {
    // Create a new file named "project_config.toml" in the project directory
    if project_path.join(".espConfig").exists() {
        milestone_log(
            &format!("Project config folder already exists at: {}", project_path.join(".espConfig").display()),
            "Project Config Creation",
            log_Type::Warning
        );  
        return None;
    }
    let config_path_file = project_path.join(".espConfig/esp_config.json");
    fs::create_dir_all(project_path.join(".espConfig")).expect("Failed to create project config folder");
    let _id = Uuid::new_v4();
    std::fs::write(
        config_path_file,
        format!(
            "
    {{
        \"project_name\": \"{}\",
        \"project_path\": \"{}\",
        \"project_id\": \"{}\",
        \"build_command\": \"source ~/export-esp.sh && cargo build\",
        \"flash_command\": \"cargo flash --monitor --port /dev/ttyUSB0\"
    }}
    ",
            project_name,
            project_path.display(),
            _id
        ),
    )
    .expect("Failed to create project config file" );
    milestone_log(
        &format!("Project config file created at: {}", project_path.join(".espConfig/esp_config.json").display()),
        "Project Config Creation",
        log_Type::Info
    );
    Some(())
}

fn create_project(current_dir: &std::path::PathBuf, project_name: &str) -> Option<String> {
    let project_dir = current_dir.join(&project_name);
    // let build_script = "source ~/export-esp.sh && cargo build";

    if project_dir.exists() {
       milestone_log(
           &format!("Project directory already exists at: {}", project_dir.display()),
           "Project Creation",
           log_Type::Warning
       );
        return None;
    }
    // // (1) run backround process "source ~/export-esp.sh"
    // Command::new("sh")
    //     .arg("-c")
    //     .arg("source ~/export-esp.sh")
    //     .status()
    //     .expect("Failed to run background process");

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
            milestone_log(
                &format!(
                    "Project config file created successfully at: {}. cd into the project directory and run 'cargo flash' to flash the project to your ESP device.", 
                    project_dir.join(".espConfig/esp_config.json").display()
                ),
                "Project Config Creation",
                log_Type::Info
            );
        } else {
            milestone_log("Failed to create project config file.", "Project Config Creation", log_Type::Error);
            return None;
        }

        // let build_status = Command::new("sh")
        //     .arg("-c")
        //     .arg(build_script)
        //     .current_dir(&project_dir)
        //     .status()
        //     .expect("Failed to run build script");
        // if build_status.success() {
        //     milestone_log("Project built successfully!", "Project Build", log_Type::Info);
        //     let fal =project_file_config_create(&project_dir, &project_name);
        //     if fal.is_some() {
        //         milestone_log(
        //             &format!(
        //                 "
        //                 Project config file created successfully at: {}
        //                 cd into the project directory and run 'cargo flash' to flash the project to your ESP device.
        //                 ", 
        //                 project_dir.join("._config/esp_config.json").display()
        //             ),
        //             "Project Config Creation",
        //             log_Type::Info
        //         );
        //     } else {
        //         milestone_log("Failed to create project config file.", "Project Config Creation", log_Type::Error);
        //     }
        // } else {
        //     milestone_log("Failed to build project.", "Project Build", log_Type::Error);
        // }
    } else {
        milestone_log("Failed to generate project.", "Project Generation", log_Type::Error);
        return None;
    }

    milestone_log("Project generated successfully!", "Project Generation", log_Type::Info);

    Some(String::from(project_name))
}

fn project_name_is_valid(project_name: &str) -> bool {
    if project_name.is_empty() || project_name.trim().is_empty() || project_name.contains(' ') {
        milestone_log("Project name cannot be empty or contain spaces.", "Project Name Validation", log_Type::Error);
        return false;
    }
    if project_name.contains('/') || project_name.contains('\\') {
        milestone_log("Project name cannot contain path separators.", "Project Name Validation", log_Type::Error);
        return false;
    }
    if project_name.contains('.') {
        milestone_log("Project name cannot contain dots.", "Project Name Validation", log_Type::Error);
        return false;
    }
    if project_name.contains('-') {
        milestone_log("Project name cannot contain hyphens.", "Project Name Validation", log_Type::Error);
        return false;
    }
    if project_name.len() > 100 {
        milestone_log("Project name cannot be longer than 100 characters.", "Project Name Validation", log_Type::Error);
        return false;
    }
    if project_name.len() < 3 {
        milestone_log("Project name cannot be shorter than 3 characters.", "Project Name Validation", log_Type::Error);
        return false;
    }
    true
}

fn project_file_valid(project_name: &str) -> bool {
    let project_path = std::env::current_dir()
        .expect("Failed to get current directory")
        .join(project_name);
    let cargo_toml_path = project_path.join("Cargo.toml");
    let main_rs_path = project_path.join("src").join("main.rs");
    if !project_path.exists() {
        milestone_log(&format!("Project does not exist at: {}", project_path.display()), "Project File Validation", log_Type::Error);
        return false;
    }
    if !cargo_toml_path.exists() {
        milestone_log(&format!("Cargo.toml does not exist at: {}", cargo_toml_path.display()), "Project File Validation", log_Type::Error);
        return false;
    }
    if !main_rs_path.exists() {
        milestone_log(&format!("main.rs does not exist at: {}", main_rs_path.display()), "Project File Validation", log_Type::Error);
        return false;
    }
    true
}

fn main() {
    // get the current directory
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let mut project_exists = false;
    let mut project_name: String = current_dir
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let args: Vec<String> = env::args().collect();
    /*
     ["project", "create" , "project_name"]
     ["project", "run"]
     ["project" , "build"]
     ["project", "help"]
     ["project", "install" , "{component_name}"]
     ["project", "uninstall" , "{component_name}"]
     ["project", "update" , "{component_name}"]
     ["project", "listcomponents"]

    */
    if args.len() < 2 {
        milestone_log("Please provide a command.", "Command Validation", log_Type::Error);
        milestone_log("Example: project run", "Command Validation", log_Type::Info);
        return;
    }

    

    let command = &args[1];

    match command.as_str() {
        "create" => {
            if args.len() < 3 {
                milestone_log("Please provide a project name.", "Command Validation", log_Type::Error);
                return;
            }
            project_name = args[2].clone();

            if !project_name_is_valid(&project_name) {
                return;
            }

            // Check if the project already exists
            let project_path = current_dir.join(&project_name);
            project_exists = project_path.exists();

            if !project_exists {
                let project = create_project(&current_dir, &project_name);
                if let Some(project) = project {
                    milestone_log(&format!("Project {} created successfully!", project), "Project Creation", log_Type::Info);
                } else {
                    milestone_log("Failed to create project.", "Project Creation", log_Type::Error);
                }
            } else {
                milestone_log("Project already exists. Skipping creation.", "Project Creation", log_Type::Info);
            }
        }
        "run" => {
            if project_exists {
                flash_esp(&project_name);
            } else {
                milestone_log("Project does not exist. Please create a project first.", "Project Run", log_Type::Error);
            }
        }

        "build" => {
            let project_exists = project_file_valid(&project_name);

            if project_exists {
                // Implement build functionality here
                let build_script = "source ~/export-esp.sh && cargo build";
                let build_status = Command::new("sh")
                    .arg("-c")
                    .arg(build_script)
                    .current_dir(&current_dir.join(&project_name))
                    .status()
                    .expect("Failed to run build script");
                if build_status.success() {
                    milestone_log("Project built successfully!", "Project Build", log_Type::Info);
                } else {
                    milestone_log("Failed to build project.", "Project Build", log_Type::Error);
                }
            } else {
                milestone_log("Project does not exist. Please create a project first.", "Project Build", log_Type::Error);
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
