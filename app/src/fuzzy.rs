use std::collections::HashMap;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use strsim::{jaro_winkler, levenshtein};

use crate::model::{GpuMode, MatchLevel, ThresholdConfig};
use crate::normalize::{middle_relation, normalize, MiddleRelation};

#[cfg(feature = "gpu")]
use crate::gpu::GpuLevenshtein;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub source_row_id: u64,
    pub source_record_id: String,
    pub base_row_id: u64,
    pub base_record_id: String,
    pub source_first_name: String,
    pub source_middle_name: String,
    pub source_last_name: String,
    pub base_first_name: String,
    pub base_middle_name: String,
    pub base_last_name: String,
    pub candidate_pool_count: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoredCandidate {
    pub source_row_id:u64,pub source_record_id:String,pub base_row_id:u64,pub base_record_id:String,
    pub first_name_score:f32,pub middle_name_score:Option<f32>,pub last_name_score:f32,pub final_score:f32,
    pub middle_match_type:String,pub candidate_count:u32,pub second_best_score:Option<f32>,pub score_margin:Option<f32>,
    pub candidate_truncated:bool,pub ambiguous:bool,pub ambiguity_reason:Option<String>,
}

fn normalized_levenshtein(a:&str,b:&str)->f32{
    let a=normalize(a);let b=normalize(b);
    if a.is_empty()&&b.is_empty(){return 100.0}
    if a.is_empty()||b.is_empty(){return 0.0}
    let max_len=a.chars().count().max(b.chars().count()).max(1) as f32;
    let d=levenshtein(&a,&b) as f32;
    ((1.0-d/max_len).max(0.0)*100.0).min(100.0)
}

fn jw(a:&str,b:&str)->f32{
    let a=normalize(a);let b=normalize(b);
    if a.is_empty()&&b.is_empty(){100.0}
    else if a.is_empty()||b.is_empty(){0.0}
    else{(jaro_winkler(&a,&b)*100.0) as f32}
}

pub fn blend_cpu(a:&str,b:&str)->f32{
    (jw(a,b)*0.70+normalized_levenshtein(a,b)*0.30).clamp(0.0,100.0)
}

fn weighted_name_score(fn_score:f32,mn_score:Option<f32>,ln_score:f32)->f32{
    match mn_score{Some(mn)=>fn_score*0.35+mn*0.25+ln_score*0.40,None=>fn_score*(35.0/75.0)+ln_score*(40.0/75.0)}
}

#[derive(Debug)]
pub struct SimilarityEngine{
    pub gpu_name:String,pub gpu_backend:String,pub gpu_active:bool,
    gpu_mode:GpuMode,gpu_min_field_pairs:usize,
    #[cfg(feature="gpu")]
    gpu:Option<GpuLevenshtein>,
}

impl SimilarityEngine{
    pub fn new(mode:GpuMode,gpu_min_field_pairs:usize)->Self{
        #[cfg(feature="gpu")]
        {
            if !matches!(mode,GpuMode::CpuOnly){
                if let Ok(gpu)=GpuLevenshtein::new(){
                    let name=gpu.adapter_name().to_owned();
                    let backend=gpu.backend_name().to_owned();
                    return Self{gpu_name:name,gpu_backend:backend,gpu_active:false,gpu_mode:mode,gpu_min_field_pairs,gpu:Some(gpu)};
                }
            }
            Self{gpu_name:"Not available".into(),gpu_backend:"CPU".into(),gpu_active:false,gpu_mode:mode,gpu_min_field_pairs,gpu:None}
        }
        #[cfg(not(feature="gpu"))]
        {
            Self{gpu_name:"GPU feature disabled".into(),gpu_backend:"CPU".into(),gpu_active:false,gpu_mode:mode,gpu_min_field_pairs}
        }
    }

    fn blend_batch(&mut self,pairs:&[(String,String)])->Vec<f32>{
        if pairs.is_empty(){return Vec::new();}
        let jws:Vec<f32>=pairs.par_iter().map(|(a,b)|jw(a,b)).collect();
        #[cfg(feature="gpu")]
        if !matches!(self.gpu_mode,GpuMode::CpuOnly)&&pairs.len()>=self.gpu_min_field_pairs{
            if let Some(gpu)=self.gpu.as_mut(){
                if let Ok(lev)=gpu.normalized_similarity_batch(pairs){
                    self.gpu_active=true;
                    return jws.into_par_iter().zip(lev.into_par_iter()).map(|(j,l)|(j*0.70+l*0.30).clamp(0.0,100.0)).collect();
                }
            }
        }
        self.gpu_active=false;
        pairs.par_iter().zip(jws.into_par_iter()).map(|((a,b),j)|(j*0.70+normalized_levenshtein(a,b)*0.30).clamp(0.0,100.0)).collect()
    }

    pub fn score_candidates(&mut self,level:MatchLevel,candidates:Vec<Candidate>,thresholds:&ThresholdConfig,candidate_cap:usize)->Vec<ScoredCandidate>{
        if candidates.is_empty(){return Vec::new();}
        let fn_pairs:Vec<_>=candidates.iter().map(|c|(c.base_first_name.clone(),c.source_first_name.clone())).collect();
        let ln_pairs:Vec<_>=candidates.iter().map(|c|(c.base_last_name.clone(),c.source_last_name.clone())).collect();
        let fn_scores=self.blend_batch(&fn_pairs);let ln_scores=self.blend_batch(&ln_pairs);

        let mut mn_pairs=Vec::new();let mut mn_index:Vec<Option<usize>>=Vec::with_capacity(candidates.len());let mut relations=Vec::with_capacity(candidates.len());
        for c in &candidates{
            let rel=middle_relation(&c.base_middle_name,&c.source_middle_name);relations.push(rel);
            if matches!(rel,MiddleRelation::Fuzzy){let idx=mn_pairs.len();mn_pairs.push((c.base_middle_name.clone(),c.source_middle_name.clone()));mn_index.push(Some(idx));}
            else{mn_index.push(None);}
        }
        let fuzzy_mn_scores=self.blend_batch(&mn_pairs);

        let scored_raw:Vec<Option<ScoredCandidate>>=(0..candidates.len()).into_par_iter().map(|i|{
            let c=&candidates[i];let fn_s=fn_scores[i];let ln_s=ln_scores[i];let rel=relations[i];
            let mn_s=match rel{
                MiddleRelation::Exact|MiddleRelation::InitialExact|MiddleRelation::InitialCompatible=>Some(100.0),
                MiddleRelation::MissingOne|MiddleRelation::MissingBoth=>None,
                MiddleRelation::InitialConflict=>Some(0.0),
                MiddleRelation::Fuzzy=>mn_index[i].map(|x|fuzzy_mn_scores[x]),
            };
            let accepted=match level{
                MatchLevel::L06=>{
                    let bm=normalize(&c.base_middle_name);let sm=normalize(&c.source_middle_name);
                    if !bm.is_empty()&&!sm.is_empty()&&bm!=sm{false}
                    else{let final_score=fn_s*0.45+ln_s*0.55;fn_s>=thresholds.l6&&ln_s>=thresholds.l6&&final_score>=thresholds.l6}
                }
                MatchLevel::L05=>{
                    if matches!(rel,MiddleRelation::InitialConflict){false}else{
                        let final_score=weighted_name_score(fn_s,mn_s,ln_s);
                        match rel{
                            MiddleRelation::MissingOne|MiddleRelation::MissingBoth=>fn_s>=thresholds.l5.max(95.0)&&ln_s>=thresholds.l5.max(95.0)&&final_score>=thresholds.l5.max(95.0),
                            MiddleRelation::Fuzzy=>fn_s>=thresholds.l5_component_min&&ln_s>=thresholds.l5_component_min&&final_score>=thresholds.l5&&mn_s.unwrap_or(0.0)>=thresholds.fuzzy_middle_min_l5,
                            _=>fn_s>=thresholds.l5_component_min&&ln_s>=thresholds.l5_component_min&&final_score>=thresholds.l5,
                        }
                    }
                }
                MatchLevel::L07=>{
                    if matches!(rel,MiddleRelation::InitialConflict){false}else{
                        let final_score=weighted_name_score(fn_s,mn_s,ln_s);
                        match rel{
                            MiddleRelation::MissingOne|MiddleRelation::MissingBoth=>fn_s>=97.0&&ln_s>=97.0&&final_score>=thresholds.l7,
                            MiddleRelation::Fuzzy=>fn_s>=thresholds.l7&&ln_s>=thresholds.l7&&final_score>=thresholds.l7&&mn_s.unwrap_or(0.0)>=thresholds.fuzzy_middle_min_l7,
                            _=>fn_s>=thresholds.l7&&ln_s>=thresholds.l7&&final_score>=thresholds.l7,
                        }
                    }
                }
                MatchLevel::L15=>{
                    if matches!(rel,MiddleRelation::InitialConflict){false}else{
                        let final_score=weighted_name_score(fn_s,mn_s,ln_s);
                        match rel{
                            MiddleRelation::MissingOne|MiddleRelation::MissingBoth=>fn_s>=97.0&&ln_s>=97.0&&final_score>=thresholds.l15,
                            MiddleRelation::Fuzzy=>fn_s>=thresholds.l15&&ln_s>=thresholds.l15&&final_score>=thresholds.l15&&mn_s.unwrap_or(0.0)>=93.0,
                            _=>fn_s>=thresholds.l15&&ln_s>=thresholds.l15&&final_score>=thresholds.l15,
                        }
                    }
                }
                _=>false,
            };
            if !accepted{return None;}
            let final_score=if matches!(level,MatchLevel::L06){fn_s*0.45+ln_s*0.55}else{weighted_name_score(fn_s,mn_s,ln_s)};
            Some(ScoredCandidate{
                source_row_id:c.source_row_id,source_record_id:c.source_record_id.clone(),base_row_id:c.base_row_id,base_record_id:c.base_record_id.clone(),
                first_name_score:fn_s,middle_name_score:mn_s,last_name_score:ln_s,final_score,middle_match_type:format!("{rel:?}"),
                candidate_count:c.candidate_pool_count,second_best_score:None,score_margin:None,candidate_truncated:c.candidate_pool_count as usize>candidate_cap,
                ambiguous:false,ambiguity_reason:None,
            })
        }).collect();

        let mut grouped:HashMap<u64,Vec<ScoredCandidate>>=HashMap::new();
        for s in scored_raw.into_iter().flatten(){grouped.entry(s.source_row_id).or_default().push(s);}
        let mut decisions=Vec::with_capacity(grouped.len());
        for (_source,mut rows) in grouped{
            rows.sort_by(|a,b|b.final_score.total_cmp(&a.final_score).then(a.base_row_id.cmp(&b.base_row_id)));
            let mut best=rows.remove(0);let second=rows.first().map(|x|x.final_score);let margin=second.map(|s|best.final_score-s);
            best.second_best_score=second;best.score_margin=margin;
            if best.candidate_truncated{best.ambiguous=true;best.ambiguity_reason=Some("CANDIDATE_CAP".into());}
            else if margin.is_some_and(|m|m<thresholds.minimum_margin){best.ambiguous=true;best.ambiguity_reason=Some("SCORE_MARGIN".into());}
            decisions.push(best);
        }
        decisions.sort_by_key(|x|x.source_row_id);decisions
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn c(m1:&str,m2:&str)->Candidate{Candidate{source_row_id:1,source_record_id:"S".into(),base_row_id:2,base_record_id:"B".into(),source_first_name:"JUAN".into(),source_middle_name:m2.into(),source_last_name:"SANTOS".into(),base_first_name:"JUAN".into(),base_middle_name:m1.into(),base_last_name:"SANTOS".into(),candidate_pool_count:1}}
    #[test] fn missing_middle_renormalizes(){let score=weighted_name_score(100.0,None,100.0);assert!((score-100.0).abs()<0.01);}
    #[test] fn l5_matches_mercedita_mercidita_and_penalosa_enye(){
        let candidate=Candidate{source_row_id:10,source_record_id:"S10".into(),base_row_id:20,base_record_id:"B20".into(),source_first_name:"Mercidita".into(),source_middle_name:"Peñalosa".into(),source_last_name:"Alagon".into(),base_first_name:"Mercedita".into(),base_middle_name:"Penalosa".into(),base_last_name:"Alagon".into(),candidate_pool_count:1};
        let mut e=SimilarityEngine::new(GpuMode::CpuOnly,usize::MAX);let out=e.score_candidates(MatchLevel::L05,vec![candidate],&ThresholdConfig::default(),100);
        assert_eq!(out.len(),1);assert!(!out[0].ambiguous);assert!(out[0].final_score>=92.0);assert!(out[0].first_name_score>=90.0);assert_eq!(out[0].middle_name_score,Some(100.0));
    }
    #[test] fn l6_does_not_override_middle_conflict(){let mut e=SimilarityEngine::new(GpuMode::CpuOnly,usize::MAX);let out=e.score_candidates(MatchLevel::L06,vec![c("MARIO","MIGUEL")],&ThresholdConfig::default(),100);assert!(out.is_empty());}
}
