BEGIN TRANSACTION;

PRAGMA user_version = 1;

CREATE TABLE
    quests
    (
        id BLOB PRIMARY KEY NOT NULL,
        status TEXT NOT NULL,
        url TEXT NOT NULL,
        create_datetime INTEGER NOT NULL,
        update_datetime INTEGER NOT NULL,
        parent_id BLOB,
        depth INTEGER NOT NULL,

        FOREIGN KEY (parent_id) REFERENCES quests (id),
        CHECK (status IN ('new', 'done', 'miss', 'fail', 'skip'))
    )
    WITHOUT ROWID;

CREATE TABLE
    quest_protocol_parameters
    (
        quest_id BLOB PRIMARY KEY NOT NULL,
        parameters TEXT NOT NULL,

        FOREIGN KEY (quest_id) REFERENCES quests (id)
    )
    WITHOUT ROWID;

CREATE TABLE
    assignments
    (
        id BLOB PRIMARY KEY NOT NULL,
        quest_id INTEGER NOT NULL,
        status TEXT NOT NULL,
        create_datetime INTEGER NOT NULL,
        update_datetime INTEGER NOT NULL,
        fetcher_id BLOB NOT NULL,

        FOREIGN KEY (quest_id) REFERENCES quests (id),
        CHECK (status IN ('act', 'comp', 'expd', 'fail'))
    )
    WITHOUT ROWID;

CREATE TABLE
    assignment_reports
    (
        assignment_id BLOB PRIMARY KEY NOT NULL,
        message TEXT NOT NULL,

        FOREIGN KEY (assignment_id) REFERENCES assignments(id)
    )
    WITHOUT ROWID;

COMMIT;
