CREATE EXTENSION IF NOT EXISTS citext;

CREATE TYPE review_status AS ENUM ('PENDING', 'APPROVED', 'REJECTED');
COMMENT ON TYPE review_status IS 'Review workflow state.';

CREATE TYPE unused_state AS ENUM ('ALPHA', 'BETA');

CREATE DOMAIN unsupported_code AS text
    CHECK (VALUE ~ '^[A-Z]+$');

CREATE TABLE organization
(
    id          uuid        NOT NULL PRIMARY KEY,
    name        varchar(120) NOT NULL,
    legal_name  citext      NULL,
    network     inet        NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);
COMMENT ON TABLE organization IS 'Organizations exposed as base entities.';
COMMENT ON COLUMN organization.name IS 'Human-readable organization name.';

CREATE TABLE account
(
    id              uuid          NOT NULL PRIMARY KEY,
    organization_id uuid          NULL REFERENCES organization (id),
    email           text          NOT NULL,
    secret_hash     text          NOT NULL,
    status          review_status NOT NULL,
    tags            text[]        NULL,
    matrix          integer[][]   NULL,
    profile         jsonb         NULL,
    balance         numeric(12, 2) NULL,
    sequence_no     numeric(14, 0) NOT NULL,
    birthday        date          NULL,
    local_time      time          NULL,
    offset_time     timetz        NULL,
    local_created   timestamp     NULL,
    remote_created  timestamptz   NOT NULL DEFAULT now(),
    active          boolean       NOT NULL DEFAULT true
);
COMMENT ON TABLE account IS 'Account row with "quoted" text
and a second line.';
COMMENT ON COLUMN account.status IS 'The persisted review state.';

CREATE TABLE membership
(
    account_id      uuid          NOT NULL REFERENCES account (id),
    organization_id uuid          NOT NULL REFERENCES organization (id),
    status          review_status NOT NULL,
    PRIMARY KEY (account_id, organization_id)
);

CREATE TABLE project
(
    id         uuid         NOT NULL PRIMARY KEY,
    owner_id   uuid         NOT NULL REFERENCES account (id),
    title      varchar(200) NOT NULL,
    budget     numeric      NULL,
    starts_at  timestamptz  NULL
);

CREATE TABLE unsupported_record
(
    id      uuid             NOT NULL PRIMARY KEY,
    payload unsupported_code NOT NULL
);

CREATE TABLE internal_audit
(
    id uuid NOT NULL PRIMARY KEY
);
