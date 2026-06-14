use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize)]
pub struct TemplateFile {
    pub name: Option<String>,
    pub source_url: String,
    pub output_path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GitHubItem {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub download_url: Option<String>,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct ProjectConfig {
    pub project_name: String,
    pub project_path: String,
    pub id: String,
    pub build_command: String,
    pub flash_command: String,
    pub install_components: Vec<String>,
}

pub enum LogType {
    Info,
    Warning,
    Error,
    Complete,
}
#[derive(Serialize)]
pub struct ProgressEvent<'a> {
    pub stage: &'a str,
    pub message: &'a str,
    pub current: u8,
    pub total: u8,
}
#[derive(Debug, Serialize, Clone, Deserialize)]
 pub enum ProgressType {
     Error,
     Complete,
     Loading,
     Finished
 }
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct ProgressLogShape {
    pub stage :ProgressType,
    pub message:String,
    pub id:String
}

pub static BRANCH_NAME: &str = "v0"; // can be set to "main" or "dev" depending on which branch you want to pull template files from
