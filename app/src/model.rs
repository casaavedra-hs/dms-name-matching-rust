use serde::{Deserialize, Serialize};

pub const APP_NAME: &str = "DMS Name Matching";
pub const APP_VERSION: &str = "11.3.3-rust-web";
pub const NORMALIZATION_VERSION: &str = "NORM-RUST-2-DIACRITIC-FOLD";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProcessMode {
    DedupOnly,
    NameMatchOnly,
    DedupNameMatch,
    DedupConsolidateMatch,
}

impl ProcessMode {
    pub const ALL: [Self; 4] = [
        Self::DedupOnly,
        Self::NameMatchOnly,
        Self::DedupNameMatch,
        Self::DedupConsolidateMatch,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::DedupOnly => "Deduplication Only",
            Self::NameMatchOnly => "Name Matching Only",
            Self::DedupNameMatch => "Deduplication + Name Matching",
            Self::DedupConsolidateMatch => "Deduplicate → Consolidate → Name Match",
        }
    }

    pub fn short_description(self) -> &'static str {
        match self {
            Self::DedupOnly => "Find duplicate persons within one selected table.",
            Self::NameMatchOnly => "Match an input table against a prepared base table.",
            Self::DedupNameMatch => "Identify duplicates first, then match every original input row.",
            Self::DedupConsolidateMatch => "Retain one qualifying duplicate representative before name matching while preserving lineage.",
        }
    }

    pub fn needs_base(self) -> bool { !matches!(self, Self::DedupOnly) }
    pub fn needs_dedup(self) -> bool { !matches!(self, Self::NameMatchOnly) }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConsolidationMode {
    ExactOnly,
    ExactAndHighProbability,
}

impl ConsolidationMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::ExactOnly => "Confirmed duplicates only (L1–L3)",
            Self::ExactAndHighProbability => "Confirmed + high-probability duplicates (L1–L7)",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MatchLevel {
    L01,L02,L03,L04,L05,L06,L07,L08,L09,L10,L11,L12,L13,L14,L15,L16,L17,
}

impl MatchLevel {
    pub const CORE: [Self; 7] = [Self::L01,Self::L02,Self::L03,Self::L04,Self::L05,Self::L06,Self::L07];
    pub const ADVANCED: [Self; 10] = [Self::L08,Self::L09,Self::L10,Self::L11,Self::L12,Self::L13,Self::L14,Self::L15,Self::L16,Self::L17];

    pub fn number(self) -> u8 {
        match self {
            Self::L01=>1,Self::L02=>2,Self::L03=>3,Self::L04=>4,Self::L05=>5,Self::L06=>6,Self::L07=>7,
            Self::L08=>8,Self::L09=>9,Self::L10=>10,Self::L11=>11,Self::L12=>12,Self::L13=>13,Self::L14=>14,
            Self::L15=>15,Self::L16=>16,Self::L17=>17,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::L01=>"L1",Self::L02=>"L2",Self::L03=>"L3",Self::L04=>"L4",Self::L05=>"L5",Self::L06=>"L6",Self::L07=>"L7",
            Self::L08=>"L8",Self::L09=>"L9",Self::L10=>"L10",Self::L11=>"L11",Self::L12=>"L12",Self::L13=>"L13",Self::L14=>"L14",
            Self::L15=>"L15",Self::L16=>"L16",Self::L17=>"L17",
        }
    }

    pub fn exact_phase(self) -> bool { !matches!(self, Self::L05 | Self::L06 | Self::L07 | Self::L15) }
    pub fn fuzzy_phase(self) -> bool { !self.exact_phase() }

    pub fn label(self) -> &'static str {
        match self {
            Self::L01=>"Exact FN + MN + LN + DOB",
            Self::L02=>"Exact FN + MI + LN + DOB",
            Self::L03=>"Exact FN + LN + DOB — Middle Missing/Compatible",
            Self::L04=>"Exact FN + MN + LN + Near DOB",
            Self::L05=>"Fuzzy FN + MN + LN + Exact DOB",
            Self::L06=>"Fuzzy FN + LN + Exact DOB",
            Self::L07=>"Fuzzy FN + MN + LN + Near DOB",
            Self::L08=>"Exact swapped FN ↔ LN + MN + DOB",
            Self::L09=>"Exact FN + swapped MN ↔ LN + DOB",
            Self::L10=>"Exact LN + swapped FN ↔ MN + DOB",
            Self::L11=>"Exact common ID + FN + LN",
            Self::L12=>"Exact FN + MN + LN + Barangay",
            Self::L13=>"Exact FN + MI + LN + Barangay",
            Self::L14=>"Exact FN + LN + Barangay",
            Self::L15=>"Fuzzy FN + MN + LN + City/Municipality",
            Self::L16=>"Exact FN + MI + LN + City/Municipality",
            Self::L17=>"Exact FN + LN + City/Municipality",
        }
    }

    pub fn default_enabled(self) -> bool { self.number() <= 7 }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConfig {
    pub host: String, pub port: u16, pub user: String, pub password: String, pub database: String, pub table: String,
}
impl Default for DbConfig {
    fn default() -> Self { Self { host:"127.0.0.1".into(),port:3306,user:"root".into(),password:String::new(),database:String::new(),table:String::new() } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMap {
    pub row_key:String,pub person_id:String,pub first_name:String,pub middle_name:String,pub middle_initial:String,
    pub last_name:String,pub birthdate:String,pub barangay_code:String,pub city_code:String,
}
impl Default for FieldMap {
    fn default() -> Self {
        Self { row_key:"id".into(),person_id:"person_id".into(),first_name:"first_name".into(),middle_name:"middle_name".into(),
            middle_initial:"middle_initial".into(),last_name:"last_name".into(),birthdate:"birthdate".into(),
            barangay_code:"barangay_code".into(),city_code:"city_code".into() }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GpuMode { Auto, ForceGpu, CpuOnly }
impl GpuMode {
    pub fn label(self) -> &'static str {
        match self { Self::Auto=>"Auto — benchmark and use GPU when beneficial",Self::ForceGpu=>"Prefer GPU — fall back to CPU on error",Self::CpuOnly=>"CPU only" }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub cpu_workers:usize,pub cpu_reserve_cores:usize,pub memory_limit_percent:f32,pub min_free_ram_gb:f32,
    pub exact_batch_size:usize,pub fuzzy_source_chunk:usize,pub fuzzy_candidate_cap:usize,pub gpu_mode:GpuMode,pub gpu_min_field_pairs:usize,
}
impl Default for PerformanceConfig {
    fn default() -> Self {
        let cpus=num_cpus::get().max(2); let reserve=2.min(cpus.saturating_sub(1));
        Self { cpu_workers:cpus.saturating_sub(reserve).max(1),cpu_reserve_cores:reserve,memory_limit_percent:82.0,
            min_free_ram_gb:8.0,exact_batch_size:100_000,fuzzy_source_chunk:2_500,fuzzy_candidate_cap:100,
            gpu_mode:GpuMode::Auto,gpu_min_field_pairs:20_000 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    pub l5:f32,pub l6:f32,pub l7:f32,pub l15:f32,pub minimum_margin:f32,pub l5_component_min:f32,
    pub fuzzy_middle_min_l5:f32,pub fuzzy_middle_min_l7:f32,
}
impl Default for ThresholdConfig {
    fn default() -> Self {
        Self { l5:92.0,l6:94.0,l7:95.0,l15:96.0,minimum_margin:3.0,l5_component_min:90.0,fuzzy_middle_min_l5:90.0,fuzzy_middle_min_l7:92.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub mode:ProcessMode,pub consolidation:ConsolidationMode,pub source1:DbConfig,pub source2:DbConfig,pub destination:DbConfig,
    pub source1_fields:FieldMap,pub source2_fields:FieldMap,pub selected_levels:Vec<MatchLevel>,pub thresholds:ThresholdConfig,
    pub performance:PerformanceConfig,pub rebuild_base_index:bool,
}
impl Default for AppConfig {
    fn default() -> Self {
        Self { mode:ProcessMode::NameMatchOnly,consolidation:ConsolidationMode::ExactOnly,source1:DbConfig::default(),source2:DbConfig::default(),
            destination:DbConfig{database:"name_matching".into(),table:String::new(),..DbConfig::default()},
            source1_fields:FieldMap::default(),source2_fields:FieldMap::default(),selected_levels:MatchLevel::CORE.to_vec(),
            thresholds:ThresholdConfig::default(),performance:PerformanceConfig::default(),rebuild_base_index:false }
    }
}
impl AppConfig {
    pub fn selected(&self, level:MatchLevel)->bool{self.selected_levels.contains(&level)}
    pub fn set_level(&mut self,level:MatchLevel,enabled:bool){if enabled&&!self.selected(level){self.selected_levels.push(level)}else if !enabled{self.selected_levels.retain(|x|*x!=level)}self.selected_levels.sort_by_key(|x|x.number())}
    pub fn reset_core_levels(&mut self){self.selected_levels=MatchLevel::CORE.to_vec()}
    pub fn exact_plan(&self)->Vec<MatchLevel>{let mut out:Vec<_>=self.selected_levels.iter().copied().filter(|x|x.exact_phase()).collect();out.sort_by_key(|x|x.number());out}
    pub fn fuzzy_plan(&self)->Vec<MatchLevel>{let mut out:Vec<_>=self.selected_levels.iter().copied().filter(|x|x.fuzzy_phase()).collect();out.sort_by_key(|x|x.number());out}
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobSnapshot {
    pub job_id:String,pub status:String,pub phase:String,pub level:String,pub total_source_rows:u64,pub processed_rows:u64,
    pub matched_rows:u64,pub ambiguous_rows:u64,pub unmatched_rows:u64,pub invalid_rows:u64,pub duplicate_rows:u64,pub retained_rows:u64,
    pub candidates_scored:u64,pub rows_per_second:f64,pub cpu_percent:f32,pub memory_percent:f32,pub memory_used_gb:f32,
    pub memory_total_gb:f32,pub gpu_name:String,pub gpu_backend:String,pub gpu_active:bool,pub checkpoint:String,pub message:String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LevelStat { pub level:String,pub phase:String,pub eligible:u64,pub matched:u64,pub ambiguous:u64,pub candidates:u64,pub duration_seconds:f64 }

#[derive(Debug, Clone)]
pub enum EngineEvent { Snapshot(JobSnapshot),Log(String),Finished(JobSnapshot),Failed(String) }
