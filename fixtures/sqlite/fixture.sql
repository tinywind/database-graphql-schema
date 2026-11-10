PRAGMA foreign_keys = ON;

CREATE TABLE organization
(
    id         INTEGER  NOT NULL PRIMARY KEY,
    name       TEXT     NOT NULL,
    created_at timestamp_with_timezone_text NOT NULL
);

CREATE TABLE account
(
    id              INTEGER       NOT NULL PRIMARY KEY,
    organization_id INTEGER       NULL REFERENCES organization (id),
    email           TEXT          NOT NULL,
    secret_hash     TEXT          NOT NULL,
    profile         json_text     NULL,
    balance         decimal(12, 2) NULL,
    sequence_no     decimal(14, 0) NOT NULL,
    birthday        date_text     NULL,
    local_time      time_text     NULL,
    local_created   datetime_text NULL,
    remote_created  timestamp_with_timezone_text NULL,
    active          boolean       NOT NULL DEFAULT 1,
    avatar          BLOB          NULL
);

CREATE TABLE membership
(
    account_id      INTEGER NOT NULL REFERENCES account (id),
    organization_id INTEGER NOT NULL REFERENCES organization (id),
    PRIMARY KEY (account_id, organization_id)
);

CREATE TABLE project
(
    id        INTEGER  NOT NULL PRIMARY KEY,
    owner_id  INTEGER  NOT NULL REFERENCES account (id),
    title     TEXT     NOT NULL,
    budget    decimal(14, 2) NULL,
    starts_at datetime_text NULL
);

CREATE TABLE unsupported_record
(
    id      INTEGER    NOT NULL PRIMARY KEY,
    payload CUSTOMTYPE NOT NULL
);

CREATE TABLE internal_audit
(
    id INTEGER NOT NULL PRIMARY KEY
);
