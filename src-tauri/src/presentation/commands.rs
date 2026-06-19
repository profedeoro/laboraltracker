use crate::application::create_project::CreateProjectUseCase;
use crate::application::create_task::CreateTaskUseCase;
use crate::application::heartbeat::HeartbeatUseCase;
use crate::application::list_projects::ListProjectsUseCase;
use crate::application::list_tasks::ListTasksUseCase;
use crate::application::start_timer::StartTimerUseCase;
use crate::application::stop_timer::StopTimerUseCase;
use crate::domain::error::AppError;
use crate::infrastructure::clock::SystemClock;
use crate::infrastructure::db::Db;
use crate::domain::ports::TimeSessionRepository;
use crate::infrastructure::sqlite_project_repository::SqliteProjectRepository;
use crate::infrastructure::sqlite_task_repository::SqliteTaskRepository;
use crate::infrastructure::sqlite_time_session_repository::SqliteTimeSessionRepository;
use crate::presentation::dto::ProjectDto;
use crate::presentation::dto::TaskDto;
use crate::presentation::dto::TimeSessionDto;
use rusqlite::Connection;
use std::sync::MutexGuard;

fn lock_db<'a>(db: &'a tauri::State<'a, Db>) -> Result<MutexGuard<'a, Connection>, AppError> {
    db.0.lock()
        .map_err(|e| AppError::Repository(format!("db lock poisoned: {e}")))
}

#[tauri::command]
pub fn create_project(
    name: String,
    color: Option<String>,
    db: tauri::State<Db>,
) -> Result<ProjectDto, AppError> {
    let conn = lock_db(&db)?;
    let mut project_repo = SqliteProjectRepository::new(&conn);
    let project = CreateProjectUseCase::execute(&mut project_repo, &SystemClock, name, color)?;
    Ok(ProjectDto::from(project))
}

#[tauri::command]
pub fn list_projects(db: tauri::State<Db>) -> Result<Vec<ProjectDto>, AppError> {
    let conn = lock_db(&db)?;
    let project_repo = SqliteProjectRepository::new(&conn);
    let projects = ListProjectsUseCase::execute(&project_repo)?;
    Ok(projects.into_iter().map(ProjectDto::from).collect())
}

#[tauri::command]
pub fn create_task(
    project_id: String,
    name: String,
    db: tauri::State<Db>,
) -> Result<TaskDto, AppError> {
    let conn = lock_db(&db)?;
    let project_repo = SqliteProjectRepository::new(&conn);
    let mut task_repo = SqliteTaskRepository::new(&conn);
    let task = CreateTaskUseCase::execute(&mut task_repo, &project_repo, &SystemClock, project_id, name)?;
    Ok(TaskDto::from(task))
}

#[tauri::command]
pub fn list_tasks(
    project_id: String,
    db: tauri::State<Db>,
) -> Result<Vec<TaskDto>, AppError> {
    let conn = lock_db(&db)?;
    let task_repo = SqliteTaskRepository::new(&conn);
    let tasks = ListTasksUseCase::execute(&task_repo, &project_id)?;
    Ok(tasks.into_iter().map(TaskDto::from).collect())
}

#[tauri::command]
pub fn start_timer(task_id: String, db: tauri::State<Db>) -> Result<TimeSessionDto, AppError> {
    let conn = lock_db(&db)?;
    let task_repo = SqliteTaskRepository::new(&conn);
    let mut session_repo = SqliteTimeSessionRepository::new(&conn);
    let session = StartTimerUseCase::execute(&mut session_repo, &task_repo, &SystemClock, task_id)?;
    Ok(TimeSessionDto::from(session))
}

#[tauri::command]
pub fn stop_timer(db: tauri::State<Db>) -> Result<TimeSessionDto, AppError> {
    let conn = lock_db(&db)?;
    let mut session_repo = SqliteTimeSessionRepository::new(&conn);
    let session = StopTimerUseCase::execute(&mut session_repo, &SystemClock)?;
    Ok(TimeSessionDto::from(session))
}

#[tauri::command]
pub fn running_timer(db: tauri::State<Db>) -> Result<Option<TimeSessionDto>, AppError> {
    let conn = lock_db(&db)?;
    let session_repo = SqliteTimeSessionRepository::new(&conn);
    Ok(session_repo.running()?.map(TimeSessionDto::from))
}

#[tauri::command]
pub fn heartbeat(db: tauri::State<Db>) -> Result<Option<TimeSessionDto>, AppError> {
    let conn = lock_db(&db)?;
    let mut session_repo = SqliteTimeSessionRepository::new(&conn);
    let session = HeartbeatUseCase::execute(&mut session_repo, &SystemClock)?;
    Ok(session.map(TimeSessionDto::from))
}
