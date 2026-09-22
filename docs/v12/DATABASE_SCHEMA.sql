-- DMS Name Matching Enterprise v12
-- Initial database design draft

CREATE TABLE IF NOT EXISTS nm_users (
    user_id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    username VARCHAR(100) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    role ENUM('ADMIN','OPERATOR','VIEWER') NOT NULL DEFAULT 'OPERATOR',
    is_active TINYINT(1) NOT NULL DEFAULT 1,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS nm_sessions (
    session_id CHAR(64) NOT NULL PRIMARY KEY,
    user_id BIGINT UNSIGNED NOT NULL,
    machine_name VARCHAR(255) NULL,
    started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME NOT NULL,
    is_revoked TINYINT(1) NOT NULL DEFAULT 0,
    KEY ix_nm_sessions_user(user_id),
    CONSTRAINT fk_nm_sessions_user FOREIGN KEY(user_id) REFERENCES nm_users(user_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS nm_connection_profiles (
    profile_id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    owner_user_id BIGINT UNSIGNED NULL,
    profile_name VARCHAR(255) NOT NULL,
    host VARCHAR(255) NOT NULL DEFAULT '127.0.0.1',
    port INT NOT NULL DEFAULT 3306,
    username VARCHAR(255) NOT NULL,
    encrypted_password TEXT NULL,
    default_database_name VARCHAR(255) NULL,
    is_shared TINYINT(1) NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    KEY ix_nm_connection_profiles_owner(owner_user_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS nm_job_registry (
    job_id CHAR(36) NOT NULL PRIMARY KEY,
    job_name VARCHAR(255) NULL,
    created_by_user_id BIGINT UNSIGNED NOT NULL,
    created_by_username VARCHAR(100) NOT NULL,
    process_mode VARCHAR(64) NOT NULL,
    status VARCHAR(64) NOT NULL,
    current_stage VARCHAR(64) NULL,
    current_level VARCHAR(32) NULL,
    source_database VARCHAR(255) NOT NULL,
    source_table VARCHAR(255) NOT NULL,
    base_database VARCHAR(255) NULL,
    base_table VARCHAR(255) NULL,
    destination_database VARCHAR(255) NOT NULL,
    destination_table VARCHAR(255) NOT NULL,
    selected_levels_json JSON NOT NULL,
    field_mapping_json JSON NOT NULL,
    extra_fields_json JSON NULL,
    config_hash CHAR(64) NOT NULL,
    total_rows BIGINT UNSIGNED NOT NULL DEFAULT 0,
    processed_rows BIGINT UNSIGNED NOT NULL DEFAULT 0,
    matched_rows BIGINT UNSIGNED NOT NULL DEFAULT 0,
    ambiguous_rows BIGINT UNSIGNED NOT NULL DEFAULT 0,
    unmatched_rows BIGINT UNSIGNED NOT NULL DEFAULT 0,
    invalid_rows BIGINT UNSIGNED NOT NULL DEFAULT 0,
    duplicate_rows BIGINT UNSIGNED NOT NULL DEFAULT 0,
    retained_rows BIGINT UNSIGNED NOT NULL DEFAULT 0,
    rows_per_second DOUBLE NULL,
    elapsed_seconds DOUBLE NULL,
    started_at DATETIME NULL,
    finished_at DATETIME NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    KEY ix_nm_job_registry_user(created_by_user_id),
    KEY ix_nm_job_registry_status(status),
    KEY ix_nm_job_registry_created(created_at),
    CONSTRAINT fk_nm_job_registry_user FOREIGN KEY(created_by_user_id) REFERENCES nm_users(user_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS nm_job_checkpoints (
    checkpoint_id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    job_id CHAR(36) NOT NULL,
    stage VARCHAR(64) NOT NULL,
    level_name VARCHAR(32) NULL,
    batch_no BIGINT UNSIGNED NOT NULL DEFAULT 0,
    source_cursor VARCHAR(255) NULL,
    processed_rows BIGINT UNSIGNED NOT NULL DEFAULT 0,
    checkpoint_json JSON NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    KEY ix_nm_job_checkpoints_job(job_id, stage, level_name, batch_no),
    CONSTRAINT fk_nm_job_checkpoints_job FOREIGN KEY(job_id) REFERENCES nm_job_registry(job_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS nm_live_match_preview (
    preview_id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    job_id CHAR(36) NOT NULL,
    event_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    batch_no BIGINT UNSIGNED NULL,
    matched_level VARCHAR(32) NULL,
    match_type ENUM('EXACT','FUZZY','DEDUP','AMBIGUOUS','UNMATCHED','INVALID') NOT NULL,
    source_unique_id VARCHAR(255) NULL,
    source_name VARCHAR(768) NULL,
    matched_unique_id VARCHAR(255) NULL,
    matched_name VARCHAR(768) NULL,
    final_score DECIMAL(7,3) NULL,
    first_name_score DECIMAL(7,3) NULL,
    middle_name_score DECIMAL(7,3) NULL,
    last_name_score DECIMAL(7,3) NULL,
    candidate_count INT NULL,
    decision VARCHAR(64) NULL,
    ambiguity_reason VARCHAR(128) NULL,
    KEY ix_nm_live_match_preview_job(preview_id, job_id),
    CONSTRAINT fk_nm_live_match_preview_job FOREIGN KEY(job_id) REFERENCES nm_job_registry(job_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS nm_audit_log (
    audit_id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    user_id BIGINT UNSIGNED NULL,
    username VARCHAR(100) NULL,
    action_type VARCHAR(100) NOT NULL,
    entity_type VARCHAR(100) NULL,
    entity_id VARCHAR(255) NULL,
    details_json JSON NULL,
    machine_name VARCHAR(255) NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    KEY ix_nm_audit_log_user(user_id),
    KEY ix_nm_audit_log_action(action_type),
    KEY ix_nm_audit_log_created(created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
