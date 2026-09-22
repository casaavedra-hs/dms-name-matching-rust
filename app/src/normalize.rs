use chrono::{Datelike, NaiveDate};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiddleRelation {
    Exact,
    InitialExact,
    InitialCompatible,
    MissingOne,
    MissingBoth,
    InitialConflict,
    Fuzzy,
}

fn transliterate_special(c: char, out: &mut String) {
    match c {
        'Ø' | 'ø' => out.push('O'),
        'Ł' | 'ł' => out.push('L'),
        'Đ' | 'đ' | 'Ð' | 'ð' => out.push('D'),
        'Þ' | 'þ' => out.push_str("TH"),
        'Æ' | 'æ' => out.push_str("AE"),
        'Œ' | 'œ' => out.push_str("OE"),
        _ => out.extend(c.to_uppercase()),
    }
}

pub fn normalize(v: &str) -> String {
    let mut folded = String::with_capacity(v.len());
    for c in v.nfd().filter(|c| !is_combining_mark(*c)) {
        transliterate_special(c, &mut folded);
    }
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn normalize_mi(mi: &str, middle: &str) -> String {
    let explicit = normalize(mi).replace('.', "");
    if let Some(c) = explicit.chars().next() {
        return c.to_string();
    }
    normalize(middle)
        .replace('.', "")
        .chars()
        .next()
        .map(|c| c.to_string())
        .unwrap_or_default()
}

pub fn parse_date(v: &str) -> Option<NaiveDate> {
    let s = v.trim();
    if s.is_empty() { return None; }
    let ten = s.chars().take(10).collect::<String>();
    NaiveDate::parse_from_str(&ten, "%Y-%m-%d").ok()
}

pub fn date_parts(v: &str) -> (Option<i32>, Option<u32>, Option<u32>) {
    parse_date(v)
        .map(|d| (Some(d.year()), Some(d.month()), Some(d.day())))
        .unwrap_or((None, None, None))
}

pub fn near_dob(a: NaiveDate, b: NaiveDate) -> bool {
    (a.year() - b.year()).abs() <= 5
        && (a.month() as i32 - b.month() as i32).abs() <= 3
        && (a.day() as i32 - b.day() as i32).abs() <= 3
}

pub fn middle_relation(a: &str, b: &str) -> MiddleRelation {
    let a = normalize(a).replace('.', "");
    let b = normalize(b).replace('.', "");
    if a.is_empty() && b.is_empty() { return MiddleRelation::MissingBoth; }
    if a.is_empty() || b.is_empty() { return MiddleRelation::MissingOne; }
    if a == b {
        return if a.chars().count() == 1 { MiddleRelation::InitialExact } else { MiddleRelation::Exact };
    }
    let ai = a.chars().next();
    let bi = b.chars().next();
    if a.chars().count() == 1 || b.chars().count() == 1 {
        return if ai == bi { MiddleRelation::InitialCompatible } else { MiddleRelation::InitialConflict };
    }
    MiddleRelation::Fuzzy
}

pub fn md5_key(parts: &[&str]) -> [u8; 16] {
    let joined = parts.join("\x1f");
    md5::compute(joined.as_bytes()).0
}

pub fn prefix_chars(v: &str, n: usize) -> String {
    normalize(v).chars().take(n).collect()
}

pub fn first_char(v: &str) -> String {
    normalize(v).chars().next().map(|c| c.to_string()).unwrap_or_default()
}

pub fn mysql_norm_expr(column: &str) -> String {
    let mut expr = format!(
        "REGEXP_REPLACE(UPPER(TRIM(COALESCE(CAST({column} AS CHAR),''))), '[[:space:]]+', ' ')"
    );
    let replacements = [
        ("Á","A"),("À","A"),("Â","A"),("Ä","A"),("Ã","A"),("Å","A"),("Ā","A"),("Ă","A"),("Ą","A"),
        ("É","E"),("È","E"),("Ê","E"),("Ë","E"),("Ē","E"),("Ĕ","E"),("Ė","E"),("Ę","E"),("Ě","E"),
        ("Í","I"),("Ì","I"),("Î","I"),("Ï","I"),("Ī","I"),("Ĭ","I"),("Į","I"),
        ("Ó","O"),("Ò","O"),("Ô","O"),("Ö","O"),("Õ","O"),("Ø","O"),("Ō","O"),("Ŏ","O"),("Ő","O"),
        ("Ú","U"),("Ù","U"),("Û","U"),("Ü","U"),("Ū","U"),("Ŭ","U"),("Ů","U"),("Ű","U"),("Ų","U"),
        ("Ñ","N"),("Ń","N"),("Ņ","N"),("Ň","N"),
        ("Ç","C"),("Ć","C"),("Ĉ","C"),("Ċ","C"),("Č","C"),
        ("Ý","Y"),("Ÿ","Y"),("Š","S"),("Ś","S"),("Ş","S"),
        ("Ž","Z"),("Ź","Z"),("Ż","Z"),("Ł","L"),("Đ","D"),("Ð","D"),
        ("Þ","TH"),("Æ","AE"),("Œ","OE"),
    ];
    for (from,to) in replacements {
        expr = format!("REPLACE({expr},'{from}','{to}')");
    }
    expr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_initial_different_full_middle_is_fuzzy() {
        assert_eq!(middle_relation("MARIO","MIGUEL"), MiddleRelation::Fuzzy);
    }

    #[test]
    fn missing_middle_is_not_conflict() {
        assert_eq!(middle_relation("","MARIO"), MiddleRelation::MissingOne);
    }

    #[test]
    fn initial_to_full_is_compatible() {
        assert_eq!(middle_relation("M","MARIO"), MiddleRelation::InitialCompatible);
    }

    #[test]
    fn near_dob_component_windows() {
        let a = NaiveDate::from_ymd_opt(1990,5,10).unwrap();
        let b = NaiveDate::from_ymd_opt(1995,8,13).unwrap();
        assert!(near_dob(a,b));
        let c = NaiveDate::from_ymd_opt(1996,8,13).unwrap();
        assert!(!near_dob(a,c));
    }

    #[test]
    fn folds_spanish_n_tilde_for_matching() {
        assert_eq!(normalize("Peñalosa"), "PENALOSA");
        assert_eq!(normalize("PENALOSA"), "PENALOSA");
    }

    #[test]
    fn collapses_spaces_and_accents() {
        assert_eq!(normalize("  María   José  "), "MARIA JOSE");
    }
}
