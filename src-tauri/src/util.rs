use chrono::{DateTime, Duration, FixedOffset, NaiveDate, Utc};
use rusqlite::Connection;
use simod_shared::{msg, UserInfo};

pub type Res<T> = Result<T, String>;

fn vn_now() -> DateTime<FixedOffset> {
    // Múi giờ Việt Nam (GMT+7)
    Utc::now().with_timezone(&FixedOffset::east_opt(7 * 3600).unwrap())
}

pub fn now() -> String {
    vn_now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn today() -> String {
    vn_now().format("%Y-%m-%d").to_string()
}

pub fn now_plus_minutes(m: i64) -> String {
    (vn_now() + Duration::minutes(m)).format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn today_plus_days(d: i64) -> String {
    (vn_now() + Duration::days(d)).format("%Y-%m-%d").to_string()
}

pub fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok()
}

pub fn days_between(from: &str, to: &str) -> i64 {
    match (parse_date(from), parse_date(to)) {
        (Some(a), Some(b)) => (b - a).num_days(),
        _ => 0,
    }
}

/// Lỗi cơ sở dữ liệu -> MSG34 (chi tiết ghi ra stderr).
pub fn dberr(e: rusqlite::Error) -> String {
    eprintln!("[DB] {e}");
    msg::MSG34.to_string()
}

/// Tìm kiếm không phân biệt hoa/thường và dấu trên nhiều trường.
pub fn contains(q: &Option<String>, fields: &[&str]) -> bool {
    simod_shared::matches_query(q.as_deref().unwrap_or(""), fields)
}

pub fn req_text(v: &str, field: &str) -> Res<String> {
    let t = v.trim();
    if t.is_empty() {
        Err(msg::msg02(field))
    } else {
        Ok(t.to_string())
    }
}

/// Ghi nhật ký hoạt động (BR-03).
pub fn log(conn: &Connection, user: &UserInfo, action: &str, detail: &str) {
    let _ = conn.execute(
        "INSERT INTO nhat_ky(at,user_id,username,action,detail) VALUES (?1,?2,?3,?4,?5)",
        rusqlite::params![now(), user.id, user.username, action, detail],
    );
}

/// Sinh mã tự động dạng PREFIX-yyyyMMdd-NNNN (mục 5.3.1).
pub fn gen_code(conn: &Connection, table: &str, prefix: &str) -> Res<String> {
    let day = today().replace('-', "");
    let like = format!("{prefix}-{day}-%");
    let n: i64 = conn
        .query_row(&format!("SELECT COUNT(*) FROM {table} WHERE code LIKE ?1"), [&like], |r| r.get(0))
        .map_err(dberr)?;
    Ok(format!("{prefix}-{day}-{:04}", n + 1))
}
