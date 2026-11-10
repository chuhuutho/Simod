//! Nhập hàng: tạo phiếu (UC13), xác nhận/từ chối (UC14), lịch sử (UC15).
use crate::inv;
use crate::util::*;
use crate::AppState;
use rusqlite::{params, Connection};
use simod_shared::*;
use tauri::State;

const ROW_SQL: &str = "SELECT p.id,p.code,s.name,u.full_name,p.created_at,cu.full_name,p.confirmed_at,p.total,p.status,p.note,
 p.supplier_id,p.created_by
 FROM phieu_nhap p
 JOIN nha_cung_cap s ON s.id=p.supplier_id
 JOIN nguoi_dung u ON u.id=p.created_by
 LEFT JOIN nguoi_dung cu ON cu.id=p.confirmed_by";

fn map_row(r: &rusqlite::Row) -> rusqlite::Result<(PurchaseRow, i64, i64)> {
    Ok((
        PurchaseRow {
            id: r.get(0)?,
            code: r.get(1)?,
            supplier_name: r.get(2)?,
            created_by_name: r.get(3)?,
            created_at: r.get(4)?,
            confirmed_by_name: r.get(5)?,
            confirmed_at: r.get(6)?,
            total: r.get(7)?,
            status: r.get(8)?,
            note: r.get(9)?,
        },
        r.get(10)?,
        r.get(11)?,
    ))
}

fn load(conn: &Connection, id: i64) -> Res<(PurchaseDetail, i64)> {
    let sql = format!("{ROW_SQL} WHERE p.id=?1");
    let (row, supplier_id, created_by) = conn
        .query_row(&sql, [id], map_row)
        .map_err(|_| "Không tìm thấy phiếu nhập.".to_string())?;
    let mut st = conn
        .prepare(
            "SELECT d.product_id,p.code,p.name,p.unit,p.has_expiry,d.qty,d.unit_cost,d.expiry
             FROM chi_tiet_phieu_nhap d JOIN san_pham p ON p.id=d.product_id WHERE d.receipt_id=?1 ORDER BY d.id",
        )
        .map_err(dberr)?;
    let lines = st
        .query_map([id], |r| {
            Ok(PurchaseLineOut {
                product_id: r.get(0)?,
                code: r.get(1)?,
                name: r.get(2)?,
                unit: r.get(3)?,
                has_expiry: r.get::<_, i64>(4)? != 0,
                quantity: r.get(5)?,
                unit_cost: r.get(6)?,
                expiry: r.get(7)?,
            })
        })
        .map_err(dberr)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(dberr)?;
    Ok((PurchaseDetail { row, supplier_id, lines }, created_by))
}

/// Kiểm tra dữ liệu phiếu (BR-15..BR-20); trả về danh sách dòng đã chuẩn hóa và tổng tiền.
fn validate(conn: &Connection, req: &SavePurchaseReq) -> Res<(Vec<PurchaseLineIn>, i64)> {
    let Some(sid) = req.supplier_id else { return Err(msg::MSG21.into()) };
    let ok: i64 = conn
        .query_row("SELECT COUNT(*) FROM nha_cung_cap WHERE id=?1 AND active=1", [sid], |r| r.get(0))
        .map_err(dberr)?;
    if ok == 0 {
        return Err(msg::MSG21.into());
    }
    if req.lines.is_empty() {
        return Err(msg::MSG24.into());
    }
    let today = today();
    let mut total = 0;
    let mut out = Vec::new();
    for l in &req.lines {
        let p = inv::get_product(conn, l.product_id)
            .map_err(dberr)?
            .ok_or_else(|| "Sản phẩm không tồn tại trong danh mục. Vui lòng báo Quản lý thêm sản phẩm.".to_string())?;
        if !p.active {
            return Err(format!("Sản phẩm {} đã ngừng kinh doanh, không thể nhập.", p.name));
        }
        if l.quantity <= 0 {
            return Err(msg::MSG15.into());
        }
        if l.unit_cost < 0 {
            return Err("Giá nhập phải là số nguyên không âm.".into());
        }
        let expiry = if p.has_expiry {
            let d = l
                .expiry
                .as_deref()
                .and_then(|d| parse_date(d).map(|_| d.trim().to_string()))
                .ok_or_else(|| format!("Vui lòng nhập hạn sử dụng cho sản phẩm {}.", p.name))?;
            if d.as_str() <= today.as_str() {
                return Err(msg::MSG23.into());
            }
            Some(d)
        } else {
            None
        };
        total += l.quantity * l.unit_cost;
        out.push(PurchaseLineIn { product_id: l.product_id, quantity: l.quantity, unit_cost: l.unit_cost, expiry });
    }
    Ok((out, total))
}

fn insert_lines(conn: &Connection, rid: i64, lines: &[PurchaseLineIn]) -> Res<()> {
    for l in lines {
        conn.execute(
            "INSERT INTO chi_tiet_phieu_nhap(receipt_id,product_id,qty,unit_cost,expiry) VALUES (?1,?2,?3,?4,?5)",
            params![rid, l.product_id, l.quantity, l.unit_cost, l.expiry],
        )
        .map_err(dberr)?;
    }
    Ok(())
}

/// Tạo phiếu mới, hoặc sửa lại phiếu "Cần chỉnh sửa" (quay về "Chờ xác nhận").
#[tauri::command]
pub fn save_purchase(state: State<AppState>, req: SavePurchaseReq) -> Res<Saved> {
    let user = state.user()?;
    let mut conn = state.db();
    let tx = conn.transaction().map_err(dberr)?;
    let (lines, total) = validate(&tx, &req)?;
    let supplier_id = req.supplier_id.unwrap();
    let saved = match req.id {
        None => {
            let code = gen_code(&tx, "phieu_nhap", "PN")?;
            tx.execute(
                "INSERT INTO phieu_nhap(code,supplier_id,created_by,created_at,total,status) VALUES (?1,?2,?3,?4,?5,?6)",
                params![code, supplier_id, user.id, now(), total, status::PN_PENDING],
            )
            .map_err(dberr)?;
            let id = tx.last_insert_rowid();
            insert_lines(&tx, id, &lines)?;
            log(&tx, &user, "Tạo phiếu nhập", &format!("{code}: {} VNĐ", fmt_money(total)));
            Saved { id, code }
        }
        Some(id) => {
            let (cur, created_by) = load(&tx, id)?;
            if cur.row.status == status::PN_CONFIRMED {
                return Err("Phiếu nhập đã xác nhận không được sửa hoặc xóa.".into());
            }
            if cur.row.status != status::PN_REJECTED {
                return Err("Chỉ sửa được phiếu ở trạng thái Cần chỉnh sửa.".into());
            }
            if !user.is_manager() && created_by != user.id {
                return Err(msg::MSG08.into());
            }
            tx.execute("DELETE FROM chi_tiet_phieu_nhap WHERE receipt_id=?1", [id]).map_err(dberr)?;
            insert_lines(&tx, id, &lines)?;
            tx.execute(
                "UPDATE phieu_nhap SET supplier_id=?1,total=?2,status=?3,note=NULL WHERE id=?4",
                params![supplier_id, total, status::PN_PENDING, id],
            )
            .map_err(dberr)?;
            log(&tx, &user, "Sửa phiếu nhập", &cur.row.code);
            Saved { id, code: cur.row.code }
        }
    };
    tx.commit().map_err(dberr)?;
    Ok(saved)
}

#[tauri::command]
pub fn list_purchases(state: State<AppState>, req: PurchaseQuery) -> Res<Vec<PurchaseRow>> {
    let user = state.user()?;
    let conn = state.db();
    let sql = format!(
        "{ROW_SQL} WHERE (?1 IS NULL OR p.created_by=?1) AND (?2 IS NULL OR p.status=?2)
         AND (?3 IS NULL OR p.supplier_id=?3)
         AND (?4 IS NULL OR substr(p.created_at,1,10) >= ?4) AND (?5 IS NULL OR substr(p.created_at,1,10) <= ?5)
         ORDER BY p.created_at DESC, p.id DESC LIMIT 1000"
    );
    let mut st = conn.prepare(&sql).map_err(dberr)?;
    let mine: Option<i64> = if user.is_manager() { None } else { Some(user.id) };
    let status = req.status.clone().filter(|s| !s.is_empty());
    let from = req.from.clone().filter(|s| !s.is_empty());
    let to = req.to.clone().filter(|s| !s.is_empty());
    let rows = st
        .query_map(params![mine, status, req.supplier_id, from, to], map_row)
        .map_err(dberr)?;
    rows.map(|r| r.map(|x| x.0)).collect::<Result<_, _>>().map_err(dberr)
}

#[tauri::command]
pub fn get_purchase(state: State<AppState>, req: IdReq) -> Res<PurchaseDetail> {
    let user = state.user()?;
    let conn = state.db();
    let (d, created_by) = load(&conn, req.id)?;
    if !user.is_manager() && created_by != user.id {
        return Err(msg::MSG08.into());
    }
    Ok(d)
}

/// Quản lý xác nhận: cộng tồn kho theo từng dòng (BR-19).
#[tauri::command]
pub fn confirm_purchase(state: State<AppState>, req: IdReq) -> Res<Saved> {
    let user = state.manager()?;
    let mut conn = state.db();
    let tx = conn.transaction().map_err(dberr)?;
    let (d, _) = load(&tx, req.id)?;
    if d.row.status == status::PN_CONFIRMED {
        return Err("Phiếu nhập đã được xác nhận trước đó.".into());
    }
    if d.row.status != status::PN_PENDING {
        return Err("Chỉ xác nhận được phiếu ở trạng thái Chờ xác nhận.".into());
    }
    for l in &d.lines {
        inv::add_batch(&tx, l.product_id, l.quantity, l.expiry.as_deref()).map_err(dberr)?;
    }
    tx.execute(
        "UPDATE phieu_nhap SET status=?1,confirmed_by=?2,confirmed_at=?3 WHERE id=?4",
        params![status::PN_CONFIRMED, user.id, now(), req.id],
    )
    .map_err(dberr)?;
    log(&tx, &user, "Xác nhận phiếu nhập", &format!("{} - cộng tồn {} dòng", d.row.code, d.lines.len()));
    tx.commit().map_err(dberr)?;
    Ok(Saved { id: req.id, code: d.row.code })
}

#[tauri::command]
pub fn reject_purchase(state: State<AppState>, req: ReasonReq) -> Res<Saved> {
    let user = state.manager()?;
    let reason = req_text(&req.reason, "Lý do")?;
    let conn = state.db();
    let (d, _) = load(&conn, req.id)?;
    if d.row.status != status::PN_PENDING {
        return Err("Chỉ từ chối được phiếu ở trạng thái Chờ xác nhận.".into());
    }
    conn.execute(
        "UPDATE phieu_nhap SET status=?1,note=?2 WHERE id=?3",
        params![status::PN_REJECTED, reason, req.id],
    )
    .map_err(dberr)?;
    log(&conn, &user, "Từ chối phiếu nhập", &format!("{}: {reason}", d.row.code));
    Ok(Saved { id: req.id, code: d.row.code })
}

#[tauri::command]
pub fn cancel_purchase(state: State<AppState>, req: IdReq) -> Res<Saved> {
    let user = state.user()?;
    let conn = state.db();
    let (d, created_by) = load(&conn, req.id)?;
    if d.row.status == status::PN_CONFIRMED {
        return Err("Phiếu nhập đã xác nhận không được sửa hoặc xóa.".into());
    }
    if d.row.status == status::PN_CANCELLED {
        return Ok(Saved { id: req.id, code: d.row.code });
    }
    if !user.is_manager() && created_by != user.id {
        return Err(msg::MSG08.into());
    }
    conn.execute("UPDATE phieu_nhap SET status=?1 WHERE id=?2", params![status::PN_CANCELLED, req.id])
        .map_err(dberr)?;
    log(&conn, &user, "Hủy phiếu nhập", &d.row.code);
    Ok(Saved { id: req.id, code: d.row.code })
}
