use crate::domain::project::Project;
use crate::domain::task::Task;
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    // ts-rs maps i64 -> bigint, but Tauri's JSON IPC delivers epoch-millis as a JS
    // number (safe: < Number.MAX_SAFE_INTEGER). Force `number` so the TS type matches
    // the runtime value and timestamp arithmetic in later plans doesn't break.
    #[ts(type = "number")]
    pub created_at: i64,
    pub archived: bool,
}

impl From<Project> for ProjectDto {
    fn from(p: Project) -> Self {
        Self {
            id: p.id,
            name: p.name,
            color: p.color,
            created_at: p.created_at,
            archived: p.archived,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct TaskDto {
    pub id: String,
    pub project_id: String,
    pub name: String,
    // i64 -> number (epoch-millis, seguro < Number.MAX_SAFE_INTEGER); ver ProjectDto.
    #[ts(type = "number")]
    pub created_at: i64,
    pub completed: bool,
}

impl From<Task> for TaskDto {
    fn from(t: Task) -> Self {
        Self {
            id: t.id,
            project_id: t.project_id,
            name: t.name,
            created_at: t.created_at,
            completed: t.completed,
        }
    }
}
