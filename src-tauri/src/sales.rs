//! Bán hàng: tạo hóa đơn + thanh toán (UC09, UC10), tra cứu lịch sử (UC11).
use crate::inv;
use crate::util::*;
use crate::AppState;
use rusqlite::{params, Connection};
use simod_shared::*;
use tauri::State;

fn load_detail(conn: &Connection, user: &UserInfo, id: i64) -> Res<InvoiceDetail> {
    let (row, created_by, paid, change) = conn
        .query_row(
            "SELECT h.id,h.code,h.created_at,u.full_name,h.method,h.total,h.status,h.created_by,h.paid,h.change
             FROM hoa_don h JOIN nguoi_dung u ON u.id=h.created_by WHERE h.id=?1",
            [id],
            |r| {
                Ok((
                    InvoiceRow {
                        id: r.get(0)?,
                        code: r.get(1)?,
                        created_at: r.get(2)?,
                        created_by_name: r.get(3)?,
                        method: r.get(4)?,
                        total: r.get(5)?,
                        status: r.get(6)?,
                    },
                    r.get::<_, i64>(7)?,
                    r.get::<_, i64>(8)?,
                    r.get::<_, i64>(9)?,
                ))
            },
        )
        .map_err(|_| "Không tìm thấy hóa đơn.".to_string())?;
    if !user.is_manager() && created_by != user.id {
        return Err(msg::MSG08.into()); // nhân viên chỉ xem hóa đơn do mình lập
    }
    let mut st = conn
        .prepare(
            "SELECT p.code,p.name,p.unit,d.qty,d.unit_price,d.amount
             FROM chi_tiet_hoa_don d JOIN san_pham p ON p.id=d.product_id WHERE d.invoice_id=?1 ORDER BY d.id",
        )
        .map_err(dberr)?;
    let lines = st
        .query_map([id], |r| {
            Ok(InvoiceLine {
                code: r.get(0)?,
                name: r.get(1)?,
                unit: r.get(2)?,
                quantity: r.get(3)?,
                unit_price: r.get(4)?,
                amount: r.get(5)?,
            })
        })
        .map_err(dberr)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(dberr)?;
    Ok(InvoiceDetail { row, paid, change, lines })
}

/// Thanh toán: kiểm tra, lưu hóa đơn "Đã thanh toán", trừ tồn kho (BR-06..BR-14) trong một giao dịch.
#[tauri::command]
pub fn checkout(state: State<AppState>, req: CheckoutReq) -> Res<InvoiceDetail> {
    let user = state.user()?;
    if req.lines.is_empty() {
        return Err(msg::MSG18.into());
    }
    if req.method != "cash" && req.method != "transfer" {
        return Err("Hình thức thanh toán không hợp lệ.".into());
    }
    let mut conn = state.db();
    let tx = conn.transaction().map_err(dberr)?;

    // Gộp các dòng trùng sản phẩm
    let mut merged: Vec<(i64, i64)> = Vec::new();
    for l in &req.lines {
        if l.quantity <= 0 {
            return Err(msg::MSG15.into());
        }
        match merged.iter_mut().find(|(p, _)| *p == l.product_id) {
            Some(m) => m.1 += l.quantity,
            None => merged.push((l.product_id, l.quantity)),
        }
    }

    let mut total = 0i64;
    let mut priced: Vec<(i64, i64, i64, i64)> = Vec::new(); // (pid, qty, price, amount)
    for (pid, qty) in &merged {
        let p = inv::get_product(&tx, *pid)
            .map_err(dberr)?
            .ok_or_else(|| "Sản phẩm không tồn tại.".to_string())?;
        if !p.active {
            return Err(format!("Sản phẩm {} đã ngừng kinh doanh.", p.name));
        }
        if p.sellable < *qty {
            return Err(if p.sellable <= 0 && p.expired_qty > 0 {
                msg::msg17(&p.name)
            } else {
                msg::msg16(&p.name, p.sellable.max(0), &p.unit)
            });
        }
        let amount = p.price * qty;
        total += amount;
        priced.push((*pid, *qty, p.price, amount));
    }
    if req.paid < total {
        return Err(msg::msg19(total - req.paid));
    }
    let change = req.paid - total;
    let code = gen_code(&tx, "hoa_don", "HD")?;
    tx.execute(
        "INSERT INTO hoa_don(code,created_by,created_at,method,total,paid,change,status) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![code, user.id, now(), req.method, total, req.paid, change, status::HD_PAID],
    )
    .map_err(dberr)?;
    let inv_id = tx.last_insert_rowid();
    for (pid, qty, price, amount) in priced {
        tx.execute(
            "INSERT INTO chi_tiet_hoa_don(invoice_id,product_id,qty,unit_price,amount) VALUES (?1,?2,?3,?4,?5)",
            params![inv_id, pid, qty, price, amount],
        )
        .map_err(dberr)?;
        inv::deduct(&tx, pid, qty, true).map_err(dberr)?;
    }
    log(&tx, &user, "Lập hóa đơn", &format!("{code}: {} VNĐ ({})", fmt_money(total), method_label(&req.method)));
    let detail = load_detail(&tx, &user, inv_id)?;
    tx.commit().map_err(dberr)?;
    Ok(detail)
}

#[tauri::command]
pub fn list_invoices(state: State<AppState>, req: InvoiceQuery) -> Res<Vec<InvoiceRow>> {
    let user = state.user()?;
    let conn = state.db();
    let mut st = conn
        .prepare(
            "SELECT h.id,h.code,h.created_at,u.full_name,h.method,h.total,h.status
             FROM hoa_don h JOIN nguoi_dung u ON u.id=h.created_by
             WHERE (?1 IS NULL OR h.created_by=?1)
               AND (?2 IS NULL OR substr(h.created_at,1,10) >= ?2)
               AND (?3 IS NULL OR substr(h.created_at,1,10) <= ?3)
             ORDER BY h.created_at DESC, h.id DESC LIMIT 1000",
        )
        .map_err(dberr)?;
    let only_mine: Option<i64> = if user.is_manager() { None } else { Some(user.id) };
    let from = req.from.clone().filter(|s| !s.is_empty());
    let to = req.to.clone().filter(|s| !s.is_empty());
    let rows = st
        .query_map(params![only_mine, from, to], |r| {
            Ok(InvoiceRow {
                id: r.get(0)?,
                code: r.get(1)?,
                created_at: r.get(2)?,
                created_by_name: r.get(3)?,
                method: r.get(4)?,
                total: r.get(5)?,
                status: r.get(6)?,
            })
        })
        .map_err(dberr)?;
    let mut out = Vec::new();
    for row in rows {
        let r = row.map_err(dberr)?;
        if contains(&req.search, &[&r.code, &r.created_by_name]) {
            out.push(r);
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn get_invoice(state: State<AppState>, req: IdReq) -> Res<InvoiceDetail> {
    let user = state.user()?;
    let conn = state.db();
    load_detail(&conn, &user, req.id)
}
