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
