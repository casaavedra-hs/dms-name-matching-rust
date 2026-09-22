use std::{fs, path::{Path, PathBuf}};

use anyhow::Result;
use mysql::prelude::Queryable;
use serde::{Deserialize, Serialize};

use crate::{db, model::{DbConfig, LevelStat}};

#[derive(Debug, Clone, Serialize)]
struct ResultExportRow {
    result_id: u64,
    source_row_id: u64,
    source_record_id: Option<String>,
    base_row_id: Option<u64>,
    base_record_id: Option<String>,
    status: String,
    category: Option<String>,
    level: Option<String>,
    final_score: Option<f64>,
    fn_score: Option<f64>,
    mn_score: Option<f64>,
    ln_score: Option<f64>,
    candidate_count: u64,
    second_best_score: Option<f64>,
    score_margin: Option<f64>,
    middle_match_type: Option<String>,
    ambiguity_reason: Option<String>,
    origin: String,
    dedup_group_source_row_id: Option<u64>,
}

impl ResultExportRow {
    fn from_mysql_row(row: &mysql::Row) -> Self {
        Self {
            result_id: row.get::<u64, _>(0).unwrap_or(0),
            source_row_id: row.get::<u64, _>(1).unwrap_or(0),
            source_record_id: row.get::<Option<String>, _>(2).flatten(),
            base_row_id: row.get::<Option<u64>, _>(3).flatten(),
            base_record_id: row.get::<Option<String>, _>(4).flatten(),
            status: row.get::<String, _>(5).unwrap_or_default(),
            category: row.get::<Option<String>, _>(6).flatten(),
            level: row.get::<Option<String>, _>(7).flatten(),
            final_score: row.get::<Option<f64>, _>(8).flatten(),
            fn_score: row.get::<Option<f64>, _>(9).flatten(),
            mn_score: row.get::<Option<f64>, _>(10).flatten(),
            ln_score: row.get::<Option<f64>, _>(11).flatten(),
            candidate_count: row.get::<u64, _>(12).unwrap_or(0),
            second_best_score: row.get::<Option<f64>, _>(13).flatten(),
            score_margin: row.get::<Option<f64>, _>(14).flatten(),
            middle_match_type: row.get::<Option<String>, _>(15).flatten(),
            ambiguity_reason: row.get::<Option<String>, _>(16).flatten(),
            origin: row.get::<String, _>(17).unwrap_or_default(),
            dedup_group_source_row_id: row.get::<Option<u64>, _>(18).flatten(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VitalReport {
    pub total_rows:u64,pub matched:u64,pub ambiguous:u64,pub unmatched:u64,pub invalid:u64,pub duplicates:u64,pub retained:u64,
    pub direct_matches:u64,pub inherited_matches:u64,pub middle_fuzzy:u64,pub middle_missing:u64,
    pub candidate_cap_ambiguities:u64,pub score_margin_ambiguities:u64,pub multiple_exact_ambiguities:u64,pub level_stats:Vec<LevelStat>,
}

pub fn load_vital_report(dest:&DbConfig,job_id:&str)->Result<VitalReport>{
    let pool=db::pool(dest)?;let mut c=pool.get_conn()?;
    let main:Option<(u64,u64,u64,u64,u64,u64,u64)>=c.exec_first(
        format!("SELECT total_source_rows,matched_rows,ambiguous_rows,unmatched_rows,invalid_rows,duplicate_rows,retained_rows FROM {} WHERE job_id=?",db::qident(db::JOBS_TABLE)),
        (job_id,)
    )?;
    let (total_rows,matched,ambiguous,unmatched,invalid,duplicates,retained)=main.unwrap_or_default();
    let direct_matches:u64=c.exec_first(format!("SELECT COUNT(*) FROM {} WHERE job_id=? AND match_status='MATCHED' AND result_origin='DIRECT'",db::qident(db::RESULTS_TABLE)),(job_id,))?.unwrap_or(0);
    let inherited_matches:u64=c.exec_first(format!("SELECT COUNT(*) FROM {} WHERE job_id=? AND match_status='MATCHED' AND result_origin='INHERITED_DEDUP'",db::qident(db::RESULTS_TABLE)),(job_id,))?.unwrap_or(0);
    let middle_fuzzy:u64=c.exec_first(format!("SELECT COUNT(*) FROM {} WHERE job_id=? AND middle_match_type='Fuzzy'",db::qident(db::RESULTS_TABLE)),(job_id,))?.unwrap_or(0);
    let middle_missing:u64=c.exec_first(format!("SELECT COUNT(*) FROM {} WHERE job_id=? AND middle_match_type IN ('MissingOne','MissingBoth')",db::qident(db::RESULTS_TABLE)),(job_id,))?.unwrap_or(0);
    let candidate_cap_ambiguities:u64=c.exec_first(format!("SELECT COUNT(*) FROM {} WHERE job_id=? AND ambiguity_reason='CANDIDATE_CAP'",db::qident(db::RESULTS_TABLE)),(job_id,))?.unwrap_or(0);
    let score_margin_ambiguities:u64=c.exec_first(format!("SELECT COUNT(*) FROM {} WHERE job_id=? AND ambiguity_reason='SCORE_MARGIN'",db::qident(db::RESULTS_TABLE)),(job_id,))?.unwrap_or(0);
    let multiple_exact_ambiguities:u64=c.exec_first(format!("SELECT COUNT(*) FROM {} WHERE job_id=? AND ambiguity_reason='MULTIPLE_EXACT'",db::qident(db::RESULTS_TABLE)),(job_id,))?.unwrap_or(0);
    let raw:Vec<(String,String,u64,u64,u64,u64,f64)>=c.exec(
        format!("SELECT level_name,phase,eligible,matched,ambiguous,candidates,duration_seconds FROM {} WHERE job_id=? ORDER BY phase,level_name",db::qident(db::LEVEL_STATS_TABLE)),(job_id,)
    )?;
    let level_stats=raw.into_iter().map(|r|LevelStat{level:r.0,phase:r.1,eligible:r.2,matched:r.3,ambiguous:r.4,candidates:r.5,duration_seconds:r.6}).collect();
    Ok(VitalReport{total_rows,matched,ambiguous,unmatched,invalid,duplicates,retained,direct_matches,inherited_matches,middle_fuzzy,middle_missing,candidate_cap_ambiguities,score_margin_ambiguities,multiple_exact_ambiguities,level_stats})
}

pub fn export_job(dest:&DbConfig,job_id:&str,root:&Path)->Result<PathBuf>{
    let dir=root.join(job_id);fs::create_dir_all(&dir)?;
    let pool=db::pool(dest)?;let mut c=pool.get_conn()?;
    let mut summary=csv::Writer::from_path(dir.join("summary.csv"))?;
    summary.write_record(["metric","value"])?;
    let r=load_vital_report(dest,job_id)?;
    for (k,v) in [
        ("total_rows",r.total_rows),("matched",r.matched),("ambiguous",r.ambiguous),("unmatched",r.unmatched),("invalid",r.invalid),
        ("duplicate_rows",r.duplicates),("retained_rows",r.retained),("direct_matches",r.direct_matches),("inherited_matches",r.inherited_matches),
        ("middle_fuzzy",r.middle_fuzzy),("middle_missing",r.middle_missing),("candidate_cap_ambiguities",r.candidate_cap_ambiguities),
        ("score_margin_ambiguities",r.score_margin_ambiguities),("multiple_exact_ambiguities",r.multiple_exact_ambiguities),
    ] {
        let value=v.to_string();summary.write_record([k,value.as_str()])?;
    }
    summary.flush()?;
    let mut levels=csv::Writer::from_path(dir.join("level_stats.csv"))?;
    levels.write_record(["phase","level","eligible","matched","ambiguous","candidates","duration_seconds"])?;
    for s in &r.level_stats{levels.serialize((&s.phase,&s.level,s.eligible,s.matched,s.ambiguous,s.candidates,s.duration_seconds))?;}
    levels.flush()?;
    export_results_stream(&mut c,job_id,&dir.join("results.csv"))?;
    export_dedup_stream(&mut c,job_id,&dir.join("dedup.csv"))?;
    Ok(dir)
}

fn export_results_stream(c:&mut mysql::PooledConn,job_id:&str,path:&Path)->Result<()>{
    let mut w=csv::WriterBuilder::new().has_headers(false).from_path(path)?;
    w.write_record(["result_id","source_row_id","source_record_id","base_row_id","base_record_id","status","category","level","final_score","fn_score","mn_score","ln_score","candidate_count","second_best_score","score_margin","middle_match_type","ambiguity_reason","origin","dedup_group_source_row_id"])?;
    let mut last=0u64;
    loop{
        let sql=format!("SELECT result_id,source_row_id,source_record_id,base_row_id,base_record_id,match_status,matched_category,matched_level,final_score,first_name_score,middle_name_score,last_name_score,candidate_count,second_best_score,score_margin,middle_match_type,ambiguity_reason,result_origin,dedup_group_source_row_id FROM {} WHERE job_id=? AND result_id>? ORDER BY result_id LIMIT 50000",db::qident(db::RESULTS_TABLE));
        let rows:Vec<mysql::Row>=c.exec(sql,(job_id,last))?;if rows.is_empty(){break;}
        for row in &rows{w.serialize(ResultExportRow::from_mysql_row(row))?;}
        last=rows.last().and_then(|row|row.get::<u64,_>(0)).unwrap_or(last);w.flush()?;
    }
    Ok(())
}

fn export_dedup_stream(c:&mut mysql::PooledConn,job_id:&str,path:&Path)->Result<()>{
    let mut w=csv::WriterBuilder::new().has_headers(true).from_path(path)?;
    w.write_record(["dedup_id","source_row_id","duplicate_of_source_row_id","level","score","confidence","middle_match_type"])?;
    let mut last=0u64;
    loop{
        type R=(u64,u64,u64,String,Option<f64>,String,Option<String>);
        let sql=format!("SELECT dedup_id,source_row_id,duplicate_of_source_row_id,dedup_level,dedup_score,confidence,middle_match_type FROM {} WHERE job_id=? AND dedup_id>? ORDER BY dedup_id LIMIT 50000",db::qident(db::DEDUP_TABLE));
        let rows:Vec<R>=c.exec(sql,(job_id,last))?;if rows.is_empty(){break;}
        for r in &rows{w.serialize(r)?;}last=rows.last().unwrap().0;w.flush()?;
    }
    Ok(())
}
