use std::env;
// use std::fs;
// use std::path::Path;
// use std::path::PathBuf;
use std::process::Command;
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

fn create_project(current_dir: &std::path::PathBuf, project_name: &str) -> Option<String> {
    let project_dir = current_dir.join(&project_name);
    let build_script = "source ~/export-esp.sh && cargo build";

    if project_dir.exists() {
        println!(
            "Project directory already exists at: {}",
            project_dir.display()
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
        println!("Project generated successfully!");

        let build_status = Command::new("sh")
            .arg("-c")
            .arg(build_script)
            .current_dir(&project_dir)
            .status()
            .expect("Failed to run build script");
        if build_status.success() {
            println!("Project built successfully!");
        } else {
            println!("Failed to build project.");
        }
    } else {
        println!("Failed to generate project.");
        return None;
    }

    println!("Project directory created at: {}", project_dir.display());

    Some(String::from(project_name))
}

fn project_name_is_valid(project_name: &str) -> bool {
    if project_name.is_empty() || project_name.trim().is_empty() || project_name.contains(' ') {
        println!("Project name cannot be empty or contain spaces.");
        return false;
    }
    if project_name.contains('/') || project_name.contains('\\') {
        println!("Project name cannot contain path separators.");
        return false;
    }
    if project_name.contains('.') {
        println!("Project name cannot contain dots.");
        return false;
    }
    if project_name.contains('-') {
        println!("Project name cannot contain hyphens.");
        return false;
    }
    if project_name.len() > 100 {
        println!("Project name cannot be longer than 100 characters.");
        return false;
    }
    if project_name.len() < 3 {
        println!("Project name cannot be shorter than 3 characters.");
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
        println!("Project does not exist at: {}", project_path.display());
        return false;
    }
    if !cargo_toml_path.exists() {
        println!(
            "Cargo.toml does not exist at: {}",
            cargo_toml_path.display()
        );
        return false;
    }
    if !main_rs_path.exists() {
        println!("main.rs does not exist at: {}", main_rs_path.display());
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
        println!("Please provide a command.");
        println!("Example: project run");
        return;
    }

    println!("Current directory: {}", current_dir.display());

    let command = &args[1];

    match command.as_str() {
        "create" => {
            if args.len() < 3 {
                println!("Please provide a project name.");
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
                    println!("Project {} created successfully!", project);
                } else {
                    println!("Failed to create project.");
                }
            } else {
                println!("Project already exists. Skipping creation.");
            }
        }
        "run" => {
            if project_exists {
                flash_esp(&project_name);
            } else {
                println!("Project does not exist. Please create a project first.");
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
                    println!("Project built successfully!");
                } else {
                    println!("Failed to build project.");
                }
            } else {
                println!("Project does not exist. Please create a project first.");
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
