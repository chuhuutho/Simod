//! Cảnh báo tồn kho, kiểm kê (UC16–UC19), báo cáo doanh thu (UC20), nhật ký.
use crate::inv;
use crate::util::*;
use crate::AppState;
use rusqlite::{params, Connection};
use simod_shared::*;
use tauri::State;

// ───────────────────────── Cảnh báo (BR-27, BR-28) ─────────────────────────

fn expiry_days(conn: &Connection) -> i64 {
    conn.query_row("SELECT value FROM cau_hinh WHERE key='expiry_days'", [], |r| r.get::<_, String>(0))
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30)
}

fn compute_alerts(conn: &Connection) -> Res<Alerts> {
    let days = expiry_days(conn);
    let mut st = conn
        .prepare(
            "SELECT code,name,unit,stock,min_stock FROM san_pham
             WHERE active=1 AND stock<=min_stock ORDER BY (stock*1.0/MAX(min_stock,1)), name",
        )
        .map_err(dberr)?;
    let low = st
        .query_map([], |r| {
            Ok(LowStockItem { code: r.get(0)?, name: r.get(1)?, unit: r.get(2)?, stock: r.get(3)?, min_stock: r.get(4)? })
        })
        .map_err(dberr)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(dberr)?;

    let today = today();
    let limit = today_plus_days(days);
    let mut st = conn
        .prepare(
            "SELECT p.code,p.name,p.unit,SUM(l.qty),l.expiry FROM lo_hang l JOIN san_pham p ON p.id=l.product_id
             WHERE p.active=1 AND l.qty>0 AND l.expiry IS NOT NULL AND l.expiry<=?1
             GROUP BY l.product_id,l.expiry ORDER BY l.expiry,p.name",
        )
        .map_err(dberr)?;
    let near_expiry = st
        .query_map([&limit], |r| {
            let expiry: String = r.get(4)?;
            Ok(ExpiryItem {
                code: r.get(0)?,
                name: r.get(1)?,
                unit: r.get(2)?,
                quantity: r.get(3)?,
                days_left: days_between(&today, &expiry),
                expiry,
            })
        })
        .map_err(dberr)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(dberr)?;
    Ok(Alerts { low, near_expiry, expiry_days: days })
}

#[tauri::command]
pub fn get_alerts(state: State<AppState>, _req: Empty) -> Res<Alerts> {
    state.user()?;
    compute_alerts(&state.db())
}

#[tauri::command]
pub fn set_expiry_days(state: State<AppState>, req: SetDaysReq) -> Res<()> {
    let me = state.manager()?;
    if req.days < 1 || req.days > 365 {
        return Err("Số ngày cảnh báo phải từ 1 đến 365.".into());
    }
    let conn = state.db();
    conn.execute(
        "INSERT INTO cau_hinh(key,value) VALUES ('expiry_days',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [req.days.to_string()],
    )
    .map_err(dberr)?;
    log(&conn, &me, "Đổi cấu hình cảnh báo cận date", &format!("{} ngày", req.days));
    Ok(())
}

#[tauri::command]
pub fn get_dashboard(state: State<AppState>, _req: Empty) -> Res<Dashboard> {
    let user = state.user()?;
    let conn = state.db();
    let alerts = compute_alerts(&conn)?;
    let today = today();
    let mine: Option<i64> = if user.is_manager() { None } else { Some(user.id) };
    let count = |sql: &str, p: &[&dyn rusqlite::ToSql]| -> Res<i64> {
        conn.query_row(sql, p, |r| r.get(0)).map_err(dberr)
    };
    let pending_purchases = count(
        "SELECT COUNT(*) FROM phieu_nhap WHERE status=?1 AND (?2 IS NULL OR created_by=?2)",
        &[&status::PN_PENDING, &mine],
    )?;
    let pending_stocktakes = if user.is_manager() {
        count("SELECT COUNT(*) FROM phieu_kiem_ke WHERE status=?1", &[&status::KK_PENDING])?
    } else {
        count(
            "SELECT COUNT(*) FROM phieu_kiem_ke WHERE status IN (?1,?2)",
            &[&status::KK_COUNTING, &status::KK_RECOUNT],
        )?
    };
    let (today_invoices, revenue) = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(total),0) FROM hoa_don
             WHERE status=?1 AND substr(created_at,1,10)=?2 AND (?3 IS NULL OR created_by=?3)",
            params![status::HD_PAID, today, mine],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        )
        .map_err(dberr)?;
    Ok(Dashboard {
        low_count: alerts.low.len() as i64,
        expiry_count: alerts.near_expiry.len() as i64,
        pending_purchases,
        pending_stocktakes,
        today_revenue: if user.is_manager() { Some(revenue) } else { None },
        today_invoices,
    })
}

// ───────────────────────── Kiểm kê ─────────────────────────

const KK_ROW_SQL: &str = "SELECT k.id,k.code,u.full_name,k.created_at,au.full_name,k.approved_at,k.status,k.note,
 (SELECT COUNT(*) FROM chi_tiet_kiem_ke d WHERE d.stocktake_id=k.id AND d.actual_qty IS NOT NULL AND d.actual_qty<>d.system_qty)
 FROM phieu_kiem_ke k JOIN nguoi_dung u ON u.id=k.created_by LEFT JOIN nguoi_dung au ON au.id=k.approved_by";

fn kk_row(r: &rusqlite::Row) -> rusqlite::Result<StocktakeRow> {
    Ok(StocktakeRow {
        id: r.get(0)?,
        code: r.get(1)?,
        created_by_name: r.get(2)?,
        created_at: r.get(3)?,
        approved_by_name: r.get(4)?,
        approved_at: r.get(5)?,
        status: r.get(6)?,
        note: r.get(7)?,
        diff_count: r.get(8)?,
    })
}

fn load_stocktake(conn: &Connection, id: i64) -> Res<StocktakeDetail> {
    let sql = format!("{KK_ROW_SQL} WHERE k.id=?1");
    let row = conn.query_row(&sql, [id], kk_row).map_err(|_| "Không tìm thấy phiếu kiểm kê.".to_string())?;
    let mut st = conn
        .prepare(
            "SELECT d.product_id,p.code,p.name,p.unit,d.system_qty,d.actual_qty,d.reason
             FROM chi_tiet_kiem_ke d JOIN san_pham p ON p.id=d.product_id WHERE d.stocktake_id=?1 ORDER BY p.name COLLATE NOCASE",
        )
        .map_err(dberr)?;
    let lines = st
        .query_map([id], |r| {
            Ok(StocktakeLine {
                product_id: r.get(0)?,
                code: r.get(1)?,
                name: r.get(2)?,
                unit: r.get(3)?,
                system_qty: r.get(4)?,
                actual_qty: r.get(5)?,
                reason: r.get(6)?,
            })
        })
        .map_err(dberr)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(dberr)?;
    Ok(StocktakeDetail { row, lines })
}

/// UC18 bước 1–2: tạo phiếu "Đang kiểm" với toàn bộ sản phẩm đang kinh doanh (BR-23).
#[tauri::command]
pub fn create_stocktake(state: State<AppState>, _req: Empty) -> Res<Saved> {
    let user = state.user()?;
    let mut conn = state.db();
    let tx = conn.transaction().map_err(dberr)?;
    let products: Vec<(i64, i64)> = {
        let mut st = tx.prepare("SELECT id,stock FROM san_pham WHERE active=1").map_err(dberr)?;
        let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(dberr)?;
        rows.collect::<Result<_, _>>().map_err(dberr)?
    };
    if products.is_empty() {
        return Err("Chưa có sản phẩm nào đang kinh doanh để kiểm kê.".into());
    }
    let code = gen_code(&tx, "phieu_kiem_ke", "KK")?;
    tx.execute(
        "INSERT INTO phieu_kiem_ke(code,created_by,created_at,status) VALUES (?1,?2,?3,?4)",
        params![code, user.id, now(), status::KK_COUNTING],
    )
    .map_err(dberr)?;
    let id = tx.last_insert_rowid();
    for (pid, stock) in products {
        tx.execute(
            "INSERT INTO chi_tiet_kiem_ke(stocktake_id,product_id,system_qty) VALUES (?1,?2,?3)",
            params![id, pid, stock],
        )
        .map_err(dberr)?;
    }
    log(&tx, &user, "Tạo phiếu kiểm kê", &code);
    tx.commit().map_err(dberr)?;
    Ok(Saved { id, code })
}

#[tauri::command]
pub fn list_stocktakes(state: State<AppState>, _req: Empty) -> Res<Vec<StocktakeRow>> {
    state.user()?;
    let conn = state.db();
    let sql = format!("{KK_ROW_SQL} ORDER BY k.created_at DESC, k.id DESC LIMIT 500");
    let mut st = conn.prepare(&sql).map_err(dberr)?;
    let rows = st.query_map([], kk_row).map_err(dberr)?;
    rows.collect::<Result<_, _>>().map_err(dberr)
}

#[tauri::command]
pub fn get_stocktake(state: State<AppState>, req: IdReq) -> Res<StocktakeDetail> {
    state.user()?;
    load_stocktake(&state.db(), req.id)
}

/// Lưu tạm hoặc hoàn tất kiểm kê (BR-22, BR-24, BR-25).
#[tauri::command]
pub fn save_stocktake(state: State<AppState>, req: SaveStocktakeReq) -> Res<StocktakeSaved> {
    let user = state.user()?;
    let mut conn = state.db();
    let tx = conn.transaction().map_err(dberr)?;
    let cur = load_stocktake(&tx, req.id)?;
    if cur.row.status != status::KK_COUNTING && cur.row.status != status::KK_RECOUNT {
        return Err("Phiếu kiểm kê này không còn ở trạng thái cho phép chỉnh sửa.".into());
    }
    for l in &req.lines {
        if matches!(l.actual, Some(a) if a < 0) {
            return Err(msg::MSG28.into());
        }
        tx.execute(
            "UPDATE chi_tiet_kiem_ke SET actual_qty=?1, reason=?2 WHERE stocktake_id=?3 AND product_id=?4",
            params![l.actual, l.reason.trim(), req.id, l.product_id],
        )
        .map_err(dberr)?;
    }
    let after = load_stocktake(&tx, req.id)?;
    let diff_count = after.row.diff_count;
    let mut new_status = after.row.status.clone();
    if req.complete {
        for l in &after.lines {
            let Some(actual) = l.actual_qty else { return Err(msg::MSG28.into()) };
            if actual != l.system_qty && l.reason.trim().is_empty() {
                return Err(msg::MSG29.into());
            }
        }
        new_status = if diff_count > 0 { status::KK_PENDING.into() } else { status::KK_DONE.into() };
        tx.execute("UPDATE phieu_kiem_ke SET status=?1 WHERE id=?2", params![new_status, req.id])
            .map_err(dberr)?;
        log(&tx, &user, "Hoàn tất kiểm kê", &format!("{}: {} chênh lệch", after.row.code, diff_count));
    }
    tx.commit().map_err(dberr)?;
    Ok(StocktakeSaved { id: req.id, code: after.row.code, status: new_status, diff_count })
}

/// UC19: duyệt điều chỉnh — tồn kho = số lượng thực tế, lưu lịch sử (BR-26).
#[tauri::command]
pub fn approve_stocktake(state: State<AppState>, req: IdReq) -> Res<Saved> {
    let user = state.manager()?;
    let mut conn = state.db();
    let tx = conn.transaction().map_err(dberr)?;
    let d = load_stocktake(&tx, req.id)?;
    if d.row.status != status::KK_PENDING {
        return Err("Chỉ duyệt được phiếu ở trạng thái Chờ duyệt.".into());
    }
    for l in d.lines.iter().filter(|l| l.actual_qty.is_some_and(|a| a != l.system_qty)) {
        let actual = l.actual_qty.unwrap();
        let current = inv::get_product(&tx, l.product_id)
            .map_err(dberr)?
            .map(|p| p.stock)
            .unwrap_or(0);
        if actual > current {
            inv::add_batch(&tx, l.product_id, actual - current, None).map_err(dberr)?;
        } else if actual < current {
            inv::deduct(&tx, l.product_id, current - actual, false).map_err(dberr)?;
        }
        tx.execute(
            "UPDATE chi_tiet_kiem_ke SET old_qty=?1,new_qty=?2 WHERE stocktake_id=?3 AND product_id=?4",
            params![current, actual, req.id, l.product_id],
        )
        .map_err(dberr)?;
        log(
            &tx,
            &user,
            "Điều chỉnh tồn kho",
            &format!("{}: {} {} → {} ({})", d.row.code, l.name, current, actual, l.reason),
        );
    }
    tx.execute(
        "UPDATE phieu_kiem_ke SET status=?1,approved_by=?2,approved_at=?3,note=NULL WHERE id=?4",
        params![status::KK_ADJUSTED, user.id, now(), req.id],
    )
    .map_err(dberr)?;
    tx.commit().map_err(dberr)?;
    Ok(Saved { id: req.id, code: d.row.code })
}

#[tauri::command]
pub fn request_recount(state: State<AppState>, req: ReasonReq) -> Res<Saved> {
    let user = state.manager()?;
    let reason = req_text(&req.reason, "Lý do")?;
    let mut conn = state.db();
    let tx = conn.transaction().map_err(dberr)?;
    let d = load_stocktake(&tx, req.id)?;
    if d.row.status != status::KK_PENDING {
        return Err("Chỉ yêu cầu kiểm lại được với phiếu ở trạng thái Chờ duyệt.".into());
    }
    // Làm mới số lượng hệ thống để nhân viên đếm lại theo tồn hiện tại
    tx.execute(
        "UPDATE chi_tiet_kiem_ke SET system_qty=(SELECT stock FROM san_pham WHERE id=product_id) WHERE stocktake_id=?1",
        [req.id],
    )
    .map_err(dberr)?;
    tx.execute(
        "UPDATE phieu_kiem_ke SET status=?1,note=?2 WHERE id=?3",
        params![status::KK_RECOUNT, reason, req.id],
    )
    .map_err(dberr)?;
    log(&tx, &user, "Yêu cầu kiểm lại", &format!("{}: {reason}", d.row.code));
    tx.commit().map_err(dberr)?;
    Ok(Saved { id: req.id, code: d.row.code })
}

// ───────────────────────── Báo cáo doanh thu (UC20, BR-29) ─────────────────────────

#[tauri::command]
pub fn revenue_report(state: State<AppState>, req: RevenueReq) -> Res<RevenueReport> {
    state.manager()?;
    if parse_date(&req.from).is_none() || parse_date(&req.to).is_none() {
        return Err("Khoảng ngày không hợp lệ.".into());
    }
    if req.from > req.to {
        return Err("Ngày bắt đầu phải trước hoặc bằng ngày kết thúc.".into());
    }
    let len = if req.group == "month" { 7 } else { 10 };
    let conn = state.db();
    let mut st = conn
        .prepare(
            "SELECT substr(created_at,1,?1) AS p, COUNT(*), COALESCE(SUM(total),0) FROM hoa_don
             WHERE status=?2 AND substr(created_at,1,10)>=?3 AND substr(created_at,1,10)<=?4
             GROUP BY p ORDER BY p DESC",
        )
        .map_err(dberr)?;
    let rows = st
        .query_map(params![len, status::HD_PAID, req.from, req.to], |r| {
            Ok(RevenueRow { period: r.get(0)?, invoices: r.get(1)?, revenue: r.get(2)? })
        })
        .map_err(dberr)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(dberr)?;
    let total_revenue = rows.iter().map(|r| r.revenue).sum();
    let total_invoices = rows.iter().map(|r| r.invoices).sum();
    Ok(RevenueReport { rows, total_revenue, total_invoices })
}

// ───────────────────────── Nhật ký hoạt động (BR-03) ─────────────────────────

#[tauri::command]
pub fn list_logs(state: State<AppState>, req: SearchReq) -> Res<Vec<LogRow>> {
    state.manager()?;
    let conn = state.db();
    let mut st = conn
        .prepare("SELECT at,username,action,detail FROM nhat_ky ORDER BY id DESC LIMIT 1000")
        .map_err(dberr)?;
    let rows = st
        .query_map([], |r| Ok(LogRow { at: r.get(0)?, username: r.get(1)?, action: r.get(2)?, detail: r.get(3)? }))
        .map_err(dberr)?;
    let mut out = Vec::new();
    for row in rows {
        let l = row.map_err(dberr)?;
        if contains(&req.search, &[&l.username, &l.action, &l.detail]) {
            out.push(l);
        }
    }
    Ok(out)
}

// ───────────────────────── Sao lưu thủ công ─────────────────────────

#[tauri::command]
pub fn backup_now(state: State<AppState>, _req: Empty) -> Res<String> {
    let me = state.manager()?;
    let conn = state.db();
    let path = crate::db::backup_now(&conn, &state.data_dir)?;
    log(&conn, &me, "Sao lưu dữ liệu", &path);
    Ok(path)
}
