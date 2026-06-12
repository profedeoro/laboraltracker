use crate::domain::project::Project;
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
