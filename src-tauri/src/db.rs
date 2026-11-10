use crate::{auth, inv, util};
use rusqlite::{params, Connection};
use std::path::Path;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS nguoi_dung(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  username TEXT NOT NULL UNIQUE COLLATE NOCASE,
  full_name TEXT NOT NULL,
  password_hash TEXT NOT NULL,
  role TEXT NOT NULL,
  active INTEGER NOT NULL DEFAULT 1,
  failed_count INTEGER NOT NULL DEFAULT 0,
  locked_until TEXT
);
CREATE TABLE IF NOT EXISTS danh_muc(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE
);
CREATE TABLE IF NOT EXISTS san_pham(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  code TEXT NOT NULL UNIQUE COLLATE NOCASE,
  name TEXT NOT NULL,
  category_id INTEGER NOT NULL REFERENCES danh_muc(id),
  unit TEXT NOT NULL,
  price INTEGER NOT NULL CHECK(price > 0),
  stock INTEGER NOT NULL DEFAULT 0,
  min_stock INTEGER NOT NULL DEFAULT 0,
  has_expiry INTEGER NOT NULL DEFAULT 0,
  active INTEGER NOT NULL DEFAULT 1
);
-- Lô hàng: theo dõi hạn sử dụng; tổng qty của các lô = san_pham.stock
CREATE TABLE IF NOT EXISTS lo_hang(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  product_id INTEGER NOT NULL REFERENCES san_pham(id),
  qty INTEGER NOT NULL,
  expiry TEXT,
  received_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_lo_hang_product ON lo_hang(product_id);
CREATE TABLE IF NOT EXISTS nha_cung_cap(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  phone TEXT NOT NULL DEFAULT '',
  address TEXT NOT NULL DEFAULT '',
  active INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE IF NOT EXISTS phieu_nhap(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  code TEXT NOT NULL UNIQUE,
  supplier_id INTEGER NOT NULL REFERENCES nha_cung_cap(id),
  created_by INTEGER NOT NULL REFERENCES nguoi_dung(id),
  created_at TEXT NOT NULL,
  confirmed_by INTEGER REFERENCES nguoi_dung(id),
  confirmed_at TEXT,
  total INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL,
  note TEXT
);
CREATE TABLE IF NOT EXISTS chi_tiet_phieu_nhap(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  receipt_id INTEGER NOT NULL REFERENCES phieu_nhap(id) ON DELETE CASCADE,
  product_id INTEGER NOT NULL REFERENCES san_pham(id),
  qty INTEGER NOT NULL,
  unit_cost INTEGER NOT NULL,
  expiry TEXT
);
CREATE TABLE IF NOT EXISTS hoa_don(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  code TEXT NOT NULL UNIQUE,
  created_by INTEGER NOT NULL REFERENCES nguoi_dung(id),
  created_at TEXT NOT NULL,
  method TEXT NOT NULL,
  total INTEGER NOT NULL,
  paid INTEGER NOT NULL,
  change INTEGER NOT NULL,
  status TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_hoa_don_created ON hoa_don(created_at);
CREATE TABLE IF NOT EXISTS chi_tiet_hoa_don(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  invoice_id INTEGER NOT NULL REFERENCES hoa_don(id) ON DELETE CASCADE,
  product_id INTEGER NOT NULL REFERENCES san_pham(id),
  qty INTEGER NOT NULL,
  unit_price INTEGER NOT NULL,
  amount INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS phieu_kiem_ke(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  code TEXT NOT NULL UNIQUE,
  created_by INTEGER NOT NULL REFERENCES nguoi_dung(id),
  created_at TEXT NOT NULL,
  approved_by INTEGER REFERENCES nguoi_dung(id),
  approved_at TEXT,
  status TEXT NOT NULL,
  note TEXT
);
-- Đồng thời là lịch sử điều chỉnh tồn kho (BR-26)
CREATE TABLE IF NOT EXISTS chi_tiet_kiem_ke(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  stocktake_id INTEGER NOT NULL REFERENCES phieu_kiem_ke(id) ON DELETE CASCADE,
  product_id INTEGER NOT NULL REFERENCES san_pham(id),
  system_qty INTEGER NOT NULL,
  actual_qty INTEGER,
  reason TEXT NOT NULL DEFAULT '',
  old_qty INTEGER,
  new_qty INTEGER,
  UNIQUE(stocktake_id, product_id)
);
CREATE TABLE IF NOT EXISTS nhat_ky(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  at TEXT NOT NULL,
  user_id INTEGER,
  username TEXT NOT NULL,
  action TEXT NOT NULL,
  detail TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS cau_hinh(
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
";

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    conn.execute_batch(SCHEMA)?;
    conn.execute(
        "INSERT OR IGNORE INTO cau_hinh(key,value) VALUES ('expiry_days','30')",
        [],
    )?;
    seed_if_empty(&conn)?;
    Ok(conn)
}

/// Giữ tối đa 30 bản sao lưu gần nhất.
fn prune_backups(bdir: &Path) {
    if let Ok(rd) = std::fs::read_dir(bdir) {
        let mut files: Vec<_> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        files.sort();
        while files.len() > 30 {
            let _ = std::fs::remove_file(files.remove(0));
        }
    }
}

fn backup_into(conn: &Connection, file: &Path) -> rusqlite::Result<()> {
    conn.execute("VACUUM INTO ?1", [file.to_string_lossy().to_string()]).map(|_| ())
}

/// Sao lưu hằng ngày (mục 5.3.3): mỗi ngày một bản, tự chạy khi mở ứng dụng.
pub fn backup_daily(conn: &Connection, dir: &Path) {
    let bdir = dir.join("backups");
    if std::fs::create_dir_all(&bdir).is_err() {
        return;
    }
    let file = bdir.join(format!("simod-{}.db", util::today().replace('-', "")));
    if !file.exists() {
        let _ = backup_into(conn, &file);
    }
    prune_backups(&bdir);
}

/// Sao lưu thủ công: trả về đường dẫn file vừa tạo.
pub fn backup_now(conn: &Connection, dir: &Path) -> Result<String, String> {
    let bdir = dir.join("backups");
    std::fs::create_dir_all(&bdir).map_err(|e| e.to_string())?;
    let stamp = util::now().replace(['-', ':'], "").replace(' ', "-");
    let file = bdir.join(format!("simod-{stamp}.db"));
    backup_into(conn, &file).map_err(util::dberr)?;
    prune_backups(&bdir);
    Ok(file.to_string_lossy().to_string())
}

/// Dữ liệu mẫu cho lần chạy đầu tiên.
/// Tài khoản: admin / admin123 (Quản lý), nhanvien / nhanvien123 (Nhân viên).
fn seed_if_empty(conn: &Connection) -> rusqlite::Result<()> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM nguoi_dung", [], |r| r.get(0))?;
    if n > 0 {
        return Ok(());
    }
    conn.execute(
        "INSERT INTO nguoi_dung(username,full_name,password_hash,role) VALUES (?1,?2,?3,'manager')",
        params!["admin", "Chủ cửa hàng", auth::hash_password("admin123")],
    )?;
    conn.execute(
        "INSERT INTO nguoi_dung(username,full_name,password_hash,role) VALUES (?1,?2,?3,'staff')",
        params!["nhanvien", "Nhân viên bán hàng", auth::hash_password("nhanvien123")],
    )?;
    for c in ["Đồ uống", "Bánh kẹo", "Gia vị", "Đồ gia dụng", "Mì - Thực phẩm khô"] {
        conn.execute("INSERT INTO danh_muc(name) VALUES (?1)", [c])?;
    }
    for (name, phone, addr) in [
        ("Công ty Phân phối Hà Nội", "0243 111 2222", "12 Láng Hạ, Đống Đa, Hà Nội"),
        ("Đại lý Bánh kẹo Minh Anh", "0912 345 678", "45 Nguyễn Trãi, Thanh Xuân, Hà Nội"),
    ] {
        conn.execute(
            "INSERT INTO nha_cung_cap(name,phone,address) VALUES (?1,?2,?3)",
            params![name, phone, addr],
        )?;
    }
    // (mã, tên, danh mục, đơn vị, giá, tồn, tối thiểu, có HSD?, số ngày HSD)
    let products: [(&str, &str, i64, &str, i64, i64, i64, bool, i64); 8] = [
        ("NUOC001", "Nước suối Aquafina 500ml", 1, "chai", 6000, 48, 12, true, 300),
        ("NUOC002", "Coca-Cola lon 330ml", 1, "lon", 10000, 36, 12, true, 20),
        ("BANH001", "Bánh quy Oreo", 2, "gói", 15000, 8, 10, true, 150),
        ("KEO001", "Kẹo Alpenliebe", 2, "gói", 12000, 25, 8, true, 400),
        ("GV001", "Nước mắm Nam Ngư 500ml", 3, "chai", 32000, 15, 5, true, 500),
        ("GV002", "Muối i-ốt 500g", 3, "gói", 7000, 30, 8, false, 0),
        ("GD001", "Giấy vệ sinh Pulppy (4 cuộn)", 4, "lốc", 28000, 20, 6, false, 0),
        ("MI001", "Mì Hảo Hảo tôm chua cay", 5, "gói", 4500, 100, 30, true, 180),
    ];
    for (code, name, cat, unit, price, stock, min, has_exp, days) in products {
        conn.execute(
            "INSERT INTO san_pham(code,name,category_id,unit,price,min_stock,has_expiry) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![code, name, cat, unit, price, min, has_exp],
        )?;
        let pid = conn.last_insert_rowid();
        let exp = if has_exp { Some(util::today_plus_days(days)) } else { None };
        inv::add_batch(conn, pid, stock, exp.as_deref())?;
    }
    Ok(())
}
