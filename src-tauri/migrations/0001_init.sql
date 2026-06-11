CREATE TABLE project (
    id          TEXT    PRIMARY KEY,
    name        TEXT    NOT NULL CHECK (length(trim(name)) > 0),
    color       TEXT,
    created_at  INTEGER NOT NULL,
    archived    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE task (
    id          TEXT    PRIMARY KEY,
    project_id  TEXT    NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    name        TEXT    NOT NULL CHECK (length(trim(name)) > 0),
    created_at  INTEGER NOT NULL,
    completed   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX ix_task_project ON task(project_id);

CREATE TABLE time_session (
    id                TEXT    PRIMARY KEY,
    task_id           TEXT    NOT NULL REFERENCES task(id) ON DELETE CASCADE,
    started_at        INTEGER NOT NULL,
    ended_at          INTEGER,
    last_heartbeat_at INTEGER,
    is_suspect        INTEGER NOT NULL DEFAULT 0,
    CHECK (ended_at IS NULL OR ended_at >= started_at)
);
CREATE INDEX ix_session_task    ON time_session(task_id);
CREATE INDEX ix_session_started ON time_session(started_at);

CREATE UNIQUE INDEX ux_one_running_session
    ON time_session( (ended_at IS NULL) )
    WHERE ended_at IS NULL;
