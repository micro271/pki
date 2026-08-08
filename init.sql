CREATE TYPE event_status AS ENUM (
    'resolved',
    'ongoing'
);

CREATE TYPE severity_level AS ENUM (
    'warning',
    'information',
    'average',
    'high',
    'disaster',
    'notclassifier'
);

CREATE TABLE IF NOT EXISTS zbx_hosts (
    host TEXT PRIMARY KEY,
    host_id BIGINT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS events (
    eventid BIGINT PRIMARY KEY,
    host TEXT NOT NULL REFERENCES zbx_hosts(host),
    severity severity_level NOT NULL,
    trigger_name TEXT NOT NULL,
    start_time BIGINT NOT NULL,
    opdata TEXT NOT NULL DEFAULT '',
    end_time BIGINT,
    status event_status NOT NULL
);

CREATE TABLE IF NOT EXISTS zbx_groups (
    group_name TEXT PRIMARY KEY,
    group_id BIGINT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS zbx_group_host (
    group_name TEXT NOT NULL REFERENCES zbx_groups(group_name) ON DELETE CASCADE,
    host TEXT NOT NULL REFERENCES zbx_hosts(host) ON DELETE CASCADE,
    PRIMARY KEY (group_name, host)
);

CREATE INDEX IF NOT EXISTS idx_events_host ON events (host);
CREATE INDEX IF NOT EXISTS idx_events_status ON events (status);
CREATE INDEX IF NOT EXISTS idx_events_start_time ON events (start_time);