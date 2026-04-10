CREATE TABLE task_completions (
    profile_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    task_id    TEXT NOT NULL,
    day        DATE NOT NULL DEFAULT CURRENT_DATE,
    PRIMARY KEY (profile_id, task_id, day)
);
