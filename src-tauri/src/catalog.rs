//! Tài khoản, danh mục, sản phẩm, nhà cung cấp.
use crate::auth::hash_password;
use crate::inv;
use crate::util::*;
use crate::AppState;
use rusqlite::{params, OptionalExtension};
use simod_shared::*;
use tauri::State;

// ───────────────────────── Tài khoản (UC04) ─────────────────────────

#[tauri::command]
pub fn list_users(state: State<AppState>, req: SearchReq) -> Res<Vec<UserRow>> {
    state.manager()?;
    let conn = state.db();
    let mut st = conn
        .prepare("SELECT id,username,full_name,role,active,locked_until FROM nguoi_dung ORDER BY role, full_name")
        .map_err(dberr)?;
    let now = now();
    let rows = st
        .query_map([], |r| {
            let locked: Option<String> = r.get(5)?;
            Ok(UserRow {
                id: r.get(0)?,
                username: r.get(1)?,
                full_name: r.get(2)?,
                role: Role::from_db(&r.get::<_, String>(3)?),
                active: r.get::<_, i64>(4)? != 0,
                locked_until: locked.filter(|l| l.as_str() > now.as_str()),
            })
        })
        .map_err(dberr)?;
    let mut out = Vec::new();
    for row in rows {
        let u = row.map_err(dberr)?;
        if contains(&req.search, &[&u.username, &u.full_name]) {
            out.push(u);
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn save_user(state: State<AppState>, req: SaveUserReq) -> Res<i64> {
    let me = state.manager()?;
    let username = req_text(&req.username, "Tên đăng nhập")?;
    let full_name = req_text(&req.full_name, "Họ và tên")?;
    let conn = state.db();
    let dup: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM nguoi_dung WHERE username=?1 AND id<>?2",
            params![username, req.id.unwrap_or(-1)],
            |r| r.get(0),
        )
        .map_err(dberr)?;
    if dup > 0 {
        return Err(msg::msg06("Tên đăng nhập"));
    }
    let pw = req.password.as_deref().filter(|p| !p.is_empty());
    if let Some(p) = pw {
        if p.chars().count() < 6 {
            return Err("Mật khẩu phải có ít nhất 6 ký tự.".into());
        }
    }
    match req.id {
        None => {
            let Some(p) = pw else { return Err(msg::msg02("Mật khẩu")) };
            conn.execute(
                "INSERT INTO nguoi_dung(username,full_name,password_hash,role) VALUES (?1,?2,?3,?4)",
                params![username, full_name, hash_password(p), req.role.as_str()],
            )
            .map_err(dberr)?;
            let id = conn.last_insert_rowid();
            log(&conn, &me, "Tạo tài khoản", &format!("{username} ({})", req.role.label()));
            Ok(id)
        }
        Some(id) => {
            if id == me.id && req.role != Role::Manager {
                return Err("Không thể tự hạ quyền của chính mình.".into());
            }
            conn.execute(
                "UPDATE nguoi_dung SET username=?1, full_name=?2, role=?3 WHERE id=?4",
                params![username, full_name, req.role.as_str(), id],
            )
            .map_err(dberr)?;
            if let Some(p) = pw {
                conn.execute(
                    "UPDATE nguoi_dung SET password_hash=?1 WHERE id=?2",
                    params![hash_password(p), id],
                )
                .map_err(dberr)?;
            }
            log(&conn, &me, "Cập nhật tài khoản", &username);
            Ok(id)
        }
    }
}

/// Khóa / mở khóa tài khoản (mở khóa cũng xóa bộ đếm đăng nhập sai).
#[tauri::command]
pub fn set_user_active(state: State<AppState>, req: SetActiveReq) -> Res<()> {
    let me = state.manager()?;
    if req.id == me.id && !req.active {
        return Err("Không thể khóa tài khoản của chính mình.".into());
    }
    let conn = state.db();
    conn.execute(
        "UPDATE nguoi_dung SET active=?1, failed_count=0, locked_until=NULL WHERE id=?2",
        params![req.active, req.id],
    )
    .map_err(dberr)?;
    log(&conn, &me, if req.active { "Mở khóa tài khoản" } else { "Khóa tài khoản" }, &format!("id={}", req.id));
    Ok(())
}

// ───────────────────────── Danh mục ─────────────────────────

#[tauri::command]
pub fn list_categories(state: State<AppState>, _req: Empty) -> Res<Vec<Category>> {
    state.user()?;
    let conn = state.db();
    let mut st = conn.prepare("SELECT id,name FROM danh_muc ORDER BY name").map_err(dberr)?;
    let rows = st
        .query_map([], |r| Ok(Category { id: r.get(0)?, name: r.get(1)? }))
        .map_err(dberr)?;
    rows.collect::<Result<_, _>>().map_err(dberr)
}

#[tauri::command]
pub fn save_category(state: State<AppState>, req: SaveCategoryReq) -> Res<i64> {
    let me = state.manager()?;
    let name = req_text(&req.name, "Tên danh mục")?;
    let conn = state.db();
    let dup: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM danh_muc WHERE name=?1 AND id<>?2",
            params![name, req.id.unwrap_or(-1)],
            |r| r.get(0),
        )
        .map_err(dberr)?;
    if dup > 0 {
        return Err(msg::msg06("Tên danh mục"));
    }
    let id = match req.id {
        Some(id) => {
            conn.execute("UPDATE danh_muc SET name=?1 WHERE id=?2", params![name, id]).map_err(dberr)?;
            id
        }
        None => {
            conn.execute("INSERT INTO danh_muc(name) VALUES (?1)", [&name]).map_err(dberr)?;
            conn.last_insert_rowid()
        }
    };
    log(&conn, &me, "Lưu danh mục", &name);
    Ok(id)
}

// ───────────────────────── Sản phẩm (UC05–UC08) ─────────────────────────

#[tauri::command]
pub fn list_products(state: State<AppState>, req: ProductQuery) -> Res<Vec<Product>> {
    state.user()?;
    let conn = state.db();
    let all = inv::all_products(&conn).map_err(dberr)?;
    Ok(all
        .into_iter()
        .filter(|p| !req.only_active || p.active)
        .filter(|p| req.category_id.map_or(true, |c| c == p.category_id))
        .filter(|p| contains(&req.search, &[&p.name, &p.code]))
        .collect())
}

fn valid_date(s: &Option<String>) -> Option<String> {
    s.as_deref().filter(|d| parse_date(d).is_some()).map(|d| d.trim().to_string())
}

#[tauri::command]
pub fn save_product(state: State<AppState>, req: SaveProductReq) -> Res<i64> {
    let me = state.manager()?;
    let code = req_text(&req.code, "Mã sản phẩm")?;
    let name = req_text(&req.name, "Tên sản phẩm")?;
    let unit = req_text(&req.unit, "Đơn vị tính")?;
    if req.price <= 0 {
        return Err("Giá bán niêm yết phải lớn hơn 0.".into());
    }
    if req.min_stock < 0 {
        return Err("Ngưỡng tồn tối thiểu phải là số nguyên không âm.".into());
    }
    let conn = state.db();
    let cat_ok: i64 = conn
        .query_row("SELECT COUNT(*) FROM danh_muc WHERE id=?1", [req.category_id], |r| r.get(0))
        .map_err(dberr)?;
    if cat_ok == 0 {
        return Err(msg::msg02("Danh mục"));
    }
    let dup: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM san_pham WHERE code=?1 AND id<>?2",
            params![code, req.id.unwrap_or(-1)],
            |r| r.get(0),
        )
        .map_err(dberr)?;
    if dup > 0 {
        return Err(msg::msg06("Mã sản phẩm"));
    }
    match req.id {
        Some(id) => {
            conn.execute(
                "UPDATE san_pham SET code=?1,name=?2,category_id=?3,unit=?4,price=?5,min_stock=?6,has_expiry=?7 WHERE id=?8",
                params![code, name, req.category_id, unit, req.price, req.min_stock, req.has_expiry, id],
            )
            .map_err(dberr)?;
            log(&conn, &me, "Cập nhật sản phẩm", &format!("{code} - {name}, giá {}", fmt_money(req.price)));
            Ok(id)
        }
        None => {
            if req.initial_stock < 0 {
                return Err("Tồn đầu kỳ phải là số nguyên không âm.".into());
            }
            let expiry = if req.has_expiry && req.initial_stock > 0 {
                let d = valid_date(&req.initial_expiry).ok_or_else(|| "Vui lòng nhập hạn sử dụng cho tồn đầu kỳ.".to_string())?;
                if d.as_str() <= today().as_str() {
                    return Err(msg::MSG23.into());
                }
                Some(d)
            } else {
                None
            };
            conn.execute(
                "INSERT INTO san_pham(code,name,category_id,unit,price,min_stock,has_expiry) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![code, name, req.category_id, unit, req.price, req.min_stock, req.has_expiry],
            )
            .map_err(dberr)?;
            let id = conn.last_insert_rowid();
            inv::add_batch(&conn, id, req.initial_stock, expiry.as_deref()).map_err(dberr)?;
            log(&conn, &me, "Thêm sản phẩm", &format!("{code} - {name}"));
            Ok(id)
        }
    }
}

/// Ngừng / kinh doanh lại (BR-05: không xóa sản phẩm).
#[tauri::command]
pub fn set_product_active(state: State<AppState>, req: SetActiveReq) -> Res<()> {
    let me = state.manager()?;
    let conn = state.db();
    conn.execute("UPDATE san_pham SET active=?1 WHERE id=?2", params![req.active, req.id]).map_err(dberr)?;
    log(
        &conn,
        &me,
        if req.active { "Kinh doanh lại sản phẩm" } else { "Ngừng kinh doanh sản phẩm" },
        &format!("id={}", req.id),
    );
    Ok(())
}

// ───────────────────────── Nhà cung cấp (UC12) ─────────────────────────

#[tauri::command]
pub fn list_suppliers(state: State<AppState>, req: SupplierQuery) -> Res<Vec<Supplier>> {
    state.user()?;
    let conn = state.db();
    let mut st = conn
        .prepare("SELECT id,name,phone,address,active FROM nha_cung_cap ORDER BY name")
        .map_err(dberr)?;
    let rows = st
        .query_map([], |r| {
            Ok(Supplier {
                id: r.get(0)?,
                name: r.get(1)?,
                phone: r.get(2)?,
                address: r.get(3)?,
                active: r.get::<_, i64>(4)? != 0,
            })
        })
        .map_err(dberr)?;
    let mut out = Vec::new();
    for row in rows {
        let s = row.map_err(dberr)?;
        if (!req.only_active || s.active) && contains(&req.search, &[&s.name, &s.phone, &s.address]) {
            out.push(s);
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn save_supplier(state: State<AppState>, req: SaveSupplierReq) -> Res<i64> {
    let me = state.manager()?;
    let name = req_text(&req.name, "Tên nhà cung cấp")?;
    let phone = req.phone.trim().to_string();
    if !phone.is_empty() && !phone.chars().all(|c| c.is_ascii_digit() || " .-+()".contains(c)) {
        return Err("Số điện thoại không hợp lệ.".into());
    }
    let conn = state.db();
    let dup: Option<i64> = conn
        .query_row(
            "SELECT id FROM nha_cung_cap WHERE name=?1 AND id<>?2",
            params![name, req.id.unwrap_or(-1)],
            |r| r.get(0),
        )
        .optional()
        .map_err(dberr)?;
    if dup.is_some() {
        return Err(msg::msg06("Tên nhà cung cấp"));
    }
    let id = match req.id {
        Some(id) => {
            conn.execute(
                "UPDATE nha_cung_cap SET name=?1,phone=?2,address=?3 WHERE id=?4",
                params![name, phone, req.address.trim(), id],
            )
            .map_err(dberr)?;
            id
        }
        None => {
            conn.execute(
                "INSERT INTO nha_cung_cap(name,phone,address) VALUES (?1,?2,?3)",
                params![name, phone, req.address.trim()],
            )
            .map_err(dberr)?;
            conn.last_insert_rowid()
        }
    };
    log(&conn, &me, "Lưu nhà cung cấp", &name);
    Ok(id)
}

#[tauri::command]
pub fn set_supplier_active(state: State<AppState>, req: SetActiveReq) -> Res<()> {
    let me = state.manager()?;
    let conn = state.db();
    conn.execute("UPDATE nha_cung_cap SET active=?1 WHERE id=?2", params![req.active, req.id]).map_err(dberr)?;
    log(
        &conn,
        &me,
        if req.active { "Kích hoạt nhà cung cấp" } else { "Ngừng hoạt động nhà cung cấp" },
        &format!("id={}", req.id),
    );
    Ok(())
}
