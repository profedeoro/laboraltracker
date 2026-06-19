use crate::application::create_project::CreateProjectUseCase;
use crate::application::create_task::CreateTaskUseCase;
use crate::application::list_projects::ListProjectsUseCase;
use crate::application::list_tasks::ListTasksUseCase;
use crate::application::start_timer::StartTimerUseCase;
use crate::application::stop_timer::StopTimerUseCase;
use crate::domain::error::AppError;
use crate::domain::ports::TimeSessionRepository;
use crate::infrastructure::clock::SystemClock;
use crate::infrastructure::db::Db;
use crate::infrastructure::sqlite_project_repository::SqliteProjectRepository;
use crate::infrastructure::sqlite_task_repository::SqliteTaskRepository;
use crate::infrastructure::sqlite_time_session_repository::SqliteTimeSessionRepository;
use crate::presentation::dto::ProjectDto;
use crate::presentation::dto::TaskDto;
use crate::presentation::dto::TimeSessionDto;

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

#[tauri::command]
pub fn create_task(
    project_id: String,
    name: String,
    db: tauri::State<Db>,
) -> Result<TaskDto, AppError> {
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::Repository("db mutex poisoned".into()))?;
    let projects = SqliteProjectRepository::new(&conn);
    let mut tasks = SqliteTaskRepository::new(&conn);
    let task = CreateTaskUseCase::execute(&mut tasks, &projects, &SystemClock, project_id, name)?;
    Ok(TaskDto::from(task))
}

#[tauri::command]
pub fn list_tasks(
    project_id: String,
    db: tauri::State<Db>,
) -> Result<Vec<TaskDto>, AppError> {
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::Repository("db mutex poisoned".into()))?;
    let repo = SqliteTaskRepository::new(&conn);
    let tasks = ListTasksUseCase::execute(&repo, &project_id)?;
    Ok(tasks.into_iter().map(TaskDto::from).collect())
}

#[tauri::command]
pub fn start_timer(task_id: String, db: tauri::State<Db>) -> Result<TimeSessionDto, AppError> {
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::Repository("db mutex poisoned".into()))?;
    let tasks = SqliteTaskRepository::new(&conn);
    let mut sessions = SqliteTimeSessionRepository::new(&conn);
    let session = StartTimerUseCase::execute(&mut sessions, &tasks, &SystemClock, task_id)?;
    Ok(TimeSessionDto::from(session))
}

#[tauri::command]
pub fn stop_timer(db: tauri::State<Db>) -> Result<TimeSessionDto, AppError> {
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::Repository("db mutex poisoned".into()))?;
    let mut sessions = SqliteTimeSessionRepository::new(&conn);
    let session = StopTimerUseCase::execute(&mut sessions, &SystemClock)?;
    Ok(TimeSessionDto::from(session))
}

#[tauri::command]
pub fn running_timer(db: tauri::State<Db>) -> Result<Option<TimeSessionDto>, AppError> {
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::Repository("db mutex poisoned".into()))?;
    let sessions = SqliteTimeSessionRepository::new(&conn);
    Ok(sessions.running()?.map(TimeSessionDto::from))
}
