use crate::application::create_project::CreateProjectUseCase;
use crate::application::list_projects::ListProjectsUseCase;
use crate::domain::error::AppError;
use crate::infrastructure::clock::SystemClock;
use crate::infrastructure::db::Db;
use crate::infrastructure::sqlite_project_repository::SqliteProjectRepository;
use crate::presentation::dto::ProjectDto;

#[tauri::command]
pub fn create_project(
    name: String,
    color: Option<String>,
    db: tauri::State<Db>,
) -> Result<ProjectDto, AppError> {
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::Repository("db mutex poisoned".into()))?;
    let mut repo = SqliteProjectRepository::new(&conn);
    let project = CreateProjectUseCase::execute(&mut repo, &SystemClock, name, color)?;
    Ok(ProjectDto::from(project))
}

#[tauri::command]
pub fn list_projects(db: tauri::State<Db>) -> Result<Vec<ProjectDto>, AppError> {
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::Repository("db mutex poisoned".into()))?;
    let repo = SqliteProjectRepository::new(&conn);
    let projects = ListProjectsUseCase::execute(&repo)?;
    Ok(projects.into_iter().map(ProjectDto::from).collect())
}
