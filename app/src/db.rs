use anyhow::{bail, Context, Result};
use mysql::{prelude::Queryable, OptsBuilder, Pool, PooledConn, Row};
use serde::{Deserialize, Serialize};

use crate::model::DbConfig;

pub const BASE_TABLE: &str = "nm_base_index";
pub const BASE_REGISTRY: &str = "nm_base_registry";
pub const STAGE_TABLE: &str = "nm_stage";
pub const JOBS_TABLE: &str = "nm_jobs";
pub const LEVEL_STATS_TABLE: &str = "nm_level_stats";
pub const RESULTS_TABLE: &str = "nm_results";
pub const DEDUP_TABLE: &str = "nm_dedup";
pub const METRICS_TABLE: &str = "nm_metrics";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub key: String,
}

pub fn qident(v: &str) -> String {
    format!("{}`{}", "`", v.replace('`', "``")) + "`"
}

pub fn qtable(db: &str, table: &str) -> String {
    format!("{}.{}", qident(db), qident(table))
}

pub fn validate_identifier(v: &str, what: &str) -> Result<()> {
    if v.trim().is_empty() { bail!("{what} is required"); }
    if v.len() > 128 { bail!("{what} is too long"); }
    if v.contains('\0') { bail!("{what} contains an invalid null character"); }
    Ok(())
}

fn builder(cfg: &DbConfig, with_db: bool) -> OptsBuilder {
    let mut b = OptsBuilder::new()
        .ip_or_hostname(Some(cfg.host.clone()))
        .tcp_port(cfg.port)
        .user(Some(cfg.user.clone()))
        .pass(Some(cfg.password.clone()));
    if with_db && !cfg.database.trim().is_empty() {
        b = b.db_name(Some(cfg.database.clone()));
    }
    b
}

pub fn pool(cfg: &DbConfig) -> Result<Pool> {
    Pool::new(builder(cfg, true)).context("Unable to create MySQL connection pool")
}

pub fn pool_no_db(cfg: &DbConfig) -> Result<Pool> {
    Pool::new(builder(cfg, false)).context("Unable to create MySQL server connection pool")
}

pub fn connect(cfg: &DbConfig) -> Result<PooledConn> {
    pool(cfg)?.get_conn().context("Unable to connect to MySQL")
}

pub fn test_connection(cfg: &DbConfig) -> Result<String> {
    let mut c = connect(cfg)?;
    let version: Option<String> = c.query_first("SELECT VERSION()")?;
    Ok(version.unwrap_or_else(|| "MySQL".into()))
}

pub fn list_databases(cfg: &DbConfig) -> Result<Vec<String>> {
    let mut c = pool_no_db(cfg)?.get_conn()?;
    c.query_map("SHOW DATABASES", |name: String| name).context("Unable to list databases")
}

pub fn list_tables(cfg: &DbConfig) -> Result<Vec<String>> {
    validate_identifier(&cfg.database, "Database")?;
    let mut c = connect(cfg)?;
    let sql = "SELECT TABLE_NAME FROM information_schema.tables WHERE table_schema=? ORDER BY TABLE_NAME";
    c.exec_map(sql, (&cfg.database,), |name: String| name).context("Unable to list tables")
}

pub fn list_columns(cfg: &DbConfig) -> Result<Vec<ColumnInfo>> {
    validate_identifier(&cfg.database, "Database")?;
    validate_identifier(&cfg.table, "Table")?;
    let mut c = connect(cfg)?;
    let sql = r#"SELECT COLUMN_NAME, DATA_TYPE, COLUMN_KEY
                 FROM information_schema.columns
                 WHERE table_schema=? AND table_name=? ORDER BY ORDINAL_POSITION"#;
    let rows: Vec<Row> = c.exec(sql, (&cfg.database, &cfg.table))?;
    Ok(rows.into_iter().map(|r| ColumnInfo {
        name: r.get::<String, _>(0).unwrap_or_default(),
        data_type: r.get::<String, _>(1).unwrap_or_default(),
        key: r.get::<String, _>(2).unwrap_or_default(),
    }).collect())
}

pub fn ensure_output_database(cfg: &DbConfig) -> Result<()> {
    validate_identifier(&cfg.database, "Destination database")?;
    let mut c = pool_no_db(cfg)?.get_conn()?;
    c.query_drop(format!("CREATE DATABASE IF NOT EXISTS {} CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci", qident(&cfg.database)))?;
    Ok(())
}

pub fn ensure_schema(cfg: &DbConfig) -> Result<()> {
    ensure_output_database(cfg)?;
    let mut c = connect(cfg)?;
    for sql in schema_statements() {
        c.query_drop(sql).with_context(|| format!("Schema statement failed: {sql}"))?;
    }
    Ok(())
}

fn schema_statements() -> Vec<&'static str> {
    vec![
        r#"CREATE TABLE IF NOT EXISTS nm_base_registry (
            id TINYINT PRIMARY KEY DEFAULT 1,
            base_signature CHAR(64) NOT NULL,
            source_description VARCHAR(512) NOT NULL,
            normalization_version VARCHAR(64) NOT NULL,
            row_count BIGINT NOT NULL DEFAULT 0,
            build_cursor VARCHAR(255) NULL,
            status VARCHAR(32) NOT NULL,
            built_at DATETIME NULL,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
        ) ENGINE=InnoDB"#,
        r#"CREATE TABLE IF NOT EXISTS nm_base_index (
            base_row_id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
            source_row_key VARCHAR(255) NOT NULL,
            person_id VARCHAR(255) NULL,
            first_name VARCHAR(255) NOT NULL DEFAULT '',
            middle_name VARCHAR(255) NOT NULL DEFAULT '',
            middle_initial VARCHAR(8) NOT NULL DEFAULT '',
            last_name VARCHAR(255) NOT NULL DEFAULT '',
            birthdate DATE NULL,
            birth_year SMALLINT NULL,
            birth_month TINYINT NULL,
            birth_day TINYINT NULL,
            barangay_code VARCHAR(64) NOT NULL DEFAULT '',
            city_code VARCHAR(64) NOT NULL DEFAULT '',
            first_initial VARCHAR(8) NOT NULL DEFAULT '',
            last_initial VARCHAR(8) NOT NULL DEFAULT '',
            first_prefix3 VARCHAR(24) NOT NULL DEFAULT '',
            last_prefix3 VARCHAR(24) NOT NULL DEFAULT '',
            key_l1 BINARY(16) NULL,
            key_l2 BINARY(16) NULL,
            key_l3 BINARY(16) NULL,
            key_full_name BINARY(16) NULL,
            UNIQUE KEY uq_base_source_key(source_row_key),
            KEY ix_l1(key_l1), KEY ix_l2(key_l2), KEY ix_l3(key_l3), KEY ix_full_name(key_full_name),
            KEY ix_dob_block(birthdate,last_prefix3,first_initial),
            KEY ix_dob_block2(birthdate,first_prefix3,last_initial),
            KEY ix_near_block(last_prefix3,first_initial,birth_year),
            KEY ix_barangay(barangay_code,last_prefix3,first_initial),
            KEY ix_city(city_code,last_prefix3,first_initial)
        ) ENGINE=InnoDB"#,
        r#"CREATE TABLE IF NOT EXISTS nm_jobs (
            job_id CHAR(36) PRIMARY KEY,
            process_mode VARCHAR(64) NOT NULL,
            consolidation_mode VARCHAR(64) NOT NULL,
            selected_levels VARCHAR(255) NOT NULL,
            config_hash CHAR(64) NOT NULL,
            status VARCHAR(32) NOT NULL,
            current_phase VARCHAR(64) NULL,
            current_level VARCHAR(16) NULL,
            stage_cursor VARCHAR(255) NULL,
            total_source_rows BIGINT NOT NULL DEFAULT 0,
            processed_rows BIGINT NOT NULL DEFAULT 0,
            matched_rows BIGINT NOT NULL DEFAULT 0,
            ambiguous_rows BIGINT NOT NULL DEFAULT 0,
            unmatched_rows BIGINT NOT NULL DEFAULT 0,
            invalid_rows BIGINT NOT NULL DEFAULT 0,
            duplicate_rows BIGINT NOT NULL DEFAULT 0,
            retained_rows BIGINT NOT NULL DEFAULT 0,
            candidates_scored BIGINT NOT NULL DEFAULT 0,
            config_json LONGTEXT NOT NULL,
            started_at DATETIME NULL,
            finished_at DATETIME NULL,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
        ) ENGINE=InnoDB"#,
        r#"CREATE TABLE IF NOT EXISTS nm_stage (
            source_row_id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
            job_id CHAR(36) NOT NULL,
            source_row_key VARCHAR(255) NOT NULL,
            source_record_id VARCHAR(255) NULL,
            first_name VARCHAR(255) NOT NULL DEFAULT '',
            middle_name VARCHAR(255) NOT NULL DEFAULT '',
            middle_initial VARCHAR(8) NOT NULL DEFAULT '',
            last_name VARCHAR(255) NOT NULL DEFAULT '',
            birthdate DATE NULL,
            birth_year SMALLINT NULL,
            birth_month TINYINT NULL,
            birth_day TINYINT NULL,
            barangay_code VARCHAR(64) NOT NULL DEFAULT '',
            city_code VARCHAR(64) NOT NULL DEFAULT '',
            first_initial VARCHAR(8) NOT NULL DEFAULT '',
            last_initial VARCHAR(8) NOT NULL DEFAULT '',
            first_prefix3 VARCHAR(24) NOT NULL DEFAULT '',
            last_prefix3 VARCHAR(24) NOT NULL DEFAULT '',
            key_l1 BINARY(16) NULL,
            key_l2 BINARY(16) NULL,
            key_l3 BINARY(16) NULL,
            key_full_name BINARY(16) NULL,
            dedup_root_id BIGINT UNSIGNED NULL,
            dedup_level VARCHAR(16) NULL,
            dedup_confidence VARCHAR(32) NULL,
            dedup_ambiguous_reason VARCHAR(64) NULL,
            match_eligible TINYINT(1) NOT NULL DEFAULT 1,
            match_state VARCHAR(32) NOT NULL DEFAULT 'PENDING',
            match_ambiguous_level VARCHAR(16) NULL,
            match_ambiguous_reason VARCHAR(64) NULL,
            UNIQUE KEY uq_stage_job_source(job_id,source_row_key),
            KEY ix_stage_job_state(job_id,match_eligible,match_state,source_row_id),
            KEY ix_stage_l1(job_id,key_l1), KEY ix_stage_l2(job_id,key_l2), KEY ix_stage_l3(job_id,key_l3),
            KEY ix_stage_full(job_id,key_full_name),
            KEY ix_stage_dob_block(job_id,birthdate,last_prefix3,first_initial),
            KEY ix_stage_dob_block2(job_id,birthdate,first_prefix3,last_initial),
            KEY ix_stage_near(job_id,last_prefix3,first_initial,birth_year),
            KEY ix_stage_dedup(job_id,dedup_root_id,source_row_id)
        ) ENGINE=InnoDB"#,
        r#"CREATE TABLE IF NOT EXISTS nm_results (
            result_id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
            job_id CHAR(36) NOT NULL,
            source_row_id BIGINT UNSIGNED NOT NULL,
            source_record_id VARCHAR(255) NULL,
            base_row_id BIGINT UNSIGNED NULL,
            base_record_id VARCHAR(255) NULL,
            match_status VARCHAR(32) NOT NULL,
            matched_category VARCHAR(64) NULL,
            matched_level VARCHAR(16) NULL,
            final_score DECIMAL(7,3) NULL,
            first_name_score DECIMAL(7,3) NULL,
            middle_name_score DECIMAL(7,3) NULL,
            last_name_score DECIMAL(7,3) NULL,
            candidate_count INT NOT NULL DEFAULT 0,
            second_best_score DECIMAL(7,3) NULL,
            score_margin DECIMAL(7,3) NULL,
            middle_match_type VARCHAR(64) NULL,
            ambiguity_reason VARCHAR(64) NULL,
            result_origin VARCHAR(64) NOT NULL DEFAULT 'DIRECT',
            dedup_group_source_row_id BIGINT UNSIGNED NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE KEY uq_result_job_source(job_id,source_row_id),
            KEY ix_result_job_status(job_id,match_status,matched_level),
            KEY ix_result_base(job_id,base_row_id)
        ) ENGINE=InnoDB"#,
        r#"CREATE TABLE IF NOT EXISTS nm_dedup (
            dedup_id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
            job_id CHAR(36) NOT NULL,
            source_row_id BIGINT UNSIGNED NOT NULL,
            duplicate_of_source_row_id BIGINT UNSIGNED NOT NULL,
            dedup_level VARCHAR(16) NOT NULL,
            dedup_score DECIMAL(7,3) NULL,
            confidence VARCHAR(32) NOT NULL,
            middle_match_type VARCHAR(64) NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE KEY uq_dedup_job_source(job_id,source_row_id),
            KEY ix_dedup_root(job_id,duplicate_of_source_row_id)
        ) ENGINE=InnoDB"#,
        r#"CREATE TABLE IF NOT EXISTS nm_level_stats (
            job_id CHAR(36) NOT NULL,
            phase VARCHAR(64) NOT NULL,
            level_name VARCHAR(16) NOT NULL,
            status VARCHAR(32) NOT NULL DEFAULT 'RUNNING',
            last_source_row_id BIGINT UNSIGNED NOT NULL DEFAULT 0,
            eligible BIGINT NOT NULL DEFAULT 0,
            matched BIGINT NOT NULL DEFAULT 0,
            ambiguous BIGINT NOT NULL DEFAULT 0,
            candidates BIGINT NOT NULL DEFAULT 0,
            duration_seconds DOUBLE NOT NULL DEFAULT 0,
            started_at DATETIME NULL,
            finished_at DATETIME NULL,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
            PRIMARY KEY(job_id,phase,level_name)
        ) ENGINE=InnoDB"#,
        r#"CREATE TABLE IF NOT EXISTS nm_metrics (
            metric_id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
            job_id CHAR(36) NOT NULL,
            captured_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            phase VARCHAR(64) NULL,
            level_name VARCHAR(16) NULL,
            cpu_percent FLOAT NULL,
            memory_percent FLOAT NULL,
            memory_used_gb FLOAT NULL,
            gpu_active TINYINT(1) NOT NULL DEFAULT 0,
            rows_per_second DOUBLE NULL,
            candidates_per_second DOUBLE NULL,
            KEY ix_metrics_job_time(job_id,captured_at)
        ) ENGINE=InnoDB"#,
    ]
}

pub fn same_server(a: &DbConfig, b: &DbConfig) -> bool {
    a.host.eq_ignore_ascii_case(&b.host) && a.port == b.port
}
