CREATE TABLE organization
(
    id         BIGINT       NOT NULL AUTO_INCREMENT PRIMARY KEY,
    name       VARCHAR(120) NOT NULL COMMENT 'Human-readable organization name.',
    created_at TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
) COMMENT = 'Organizations exposed as base entities.';

CREATE TABLE account
(
    id              BIGINT                              NOT NULL AUTO_INCREMENT PRIMARY KEY,
    organization_id BIGINT                              NULL,
    email           TEXT                                NOT NULL,
    secret_hash     TEXT                                NOT NULL,
    status          ENUM ('PENDING','APPROVED','REJECTED') NOT NULL COMMENT 'The persisted review state.',
    profile         JSON                                NULL,
    balance         DECIMAL(12, 2)                      NULL,
    sequence_no     DECIMAL(14, 0)                      NOT NULL,
    birthday        DATE                                NULL,
    local_time      TIME                                NULL,
    local_created   DATETIME                            NULL,
    remote_created  TIMESTAMP                           NULL,
    active          BOOLEAN                             NOT NULL DEFAULT TRUE,
    avatar          BLOB                                NULL,
    CONSTRAINT fk_account_organization
        FOREIGN KEY (organization_id) REFERENCES organization (id)
) COMMENT = 'Account rows.';

CREATE TABLE membership
(
    account_id      BIGINT                              NOT NULL,
    organization_id BIGINT                              NOT NULL,
    status          ENUM ('PENDING','APPROVED','REJECTED') NOT NULL,
    PRIMARY KEY (account_id, organization_id),
    CONSTRAINT fk_membership_account
        FOREIGN KEY (account_id) REFERENCES account (id),
    CONSTRAINT fk_membership_organization
        FOREIGN KEY (organization_id) REFERENCES organization (id)
);

CREATE TABLE project
(
    id        BIGINT       NOT NULL AUTO_INCREMENT PRIMARY KEY,
    owner_id  BIGINT       NOT NULL,
    title     VARCHAR(200) NOT NULL,
    budget    DECIMAL(14, 2) NULL,
    starts_at DATETIME     NULL,
    CONSTRAINT fk_project_owner FOREIGN KEY (owner_id) REFERENCES account (id)
);

CREATE TABLE unsupported_record
(
    id      BIGINT   NOT NULL AUTO_INCREMENT PRIMARY KEY,
    payload GEOMETRY NOT NULL
);

CREATE TABLE internal_audit
(
    id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY
);
