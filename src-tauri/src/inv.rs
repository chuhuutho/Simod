//! Nghiệp vụ tồn kho theo lô. Bất biến: san_pham.stock = Σ lo_hang.qty.
use crate::util;
use rusqlite::{params, Connection};
use simod_shared::Product;

pub const PRODUCT_SQL: &str = "SELECT p.id,p.code,p.name,p.category_id,c.name,p.unit,p.price,p.stock,p.min_stock,
 p.has_expiry,p.active,
 (SELECT MIN(expiry) FROM lo_hang l WHERE l.product_id=p.id AND l.qty>0 AND l.expiry IS NOT NULL),
 (SELECT COALESCE(SUM(qty),0) FROM lo_hang l WHERE l.product_id=p.id AND l.expiry IS NOT NULL AND l.expiry < ?1)
 FROM san_pham p JOIN danh_muc c ON c.id=p.category_id";

pub fn map_product(r: &rusqlite::Row) -> rusqlite::Result<Product> {
    let stock: i64 = r.get(7)?;
    let expired: i64 = r.get(12)?;
    Ok(Product {
        id: r.get(0)?,
        code: r.get(1)?,
        name: r.get(2)?,
        category_id: r.get(3)?,
        category_name: r.get(4)?,
        unit: r.get(5)?,
        price: r.get(6)?,
        stock,
        min_stock: r.get(8)?,
        has_expiry: r.get::<_, i64>(9)? != 0,
        active: r.get::<_, i64>(10)? != 0,
        nearest_expiry: r.get(11)?,
        expired_qty: expired,
        sellable: stock - expired,
    })
}

pub fn all_products(conn: &Connection) -> rusqlite::Result<Vec<Product>> {
    let sql = format!("{PRODUCT_SQL} ORDER BY p.name COLLATE NOCASE");
    let mut st = conn.prepare(&sql)?;
    let rows = st.query_map([util::today()], map_product)?;
    rows.collect()
}

pub fn get_product(conn: &Connection, id: i64) -> rusqlite::Result<Option<Product>> {
    let sql = format!("{PRODUCT_SQL} WHERE p.id=?2");
    let mut st = conn.prepare(&sql)?;
    let mut rows = st.query_map(params![util::today(), id], map_product)?;
    rows.next().transpose()
}

/// Nhập một lô vào kho (cộng tồn).
pub fn add_batch(conn: &Connection, pid: i64, qty: i64, expiry: Option<&str>) -> rusqlite::Result<()> {
    if qty <= 0 {
        return Ok(());
    }
    conn.execute(
        "INSERT INTO lo_hang(product_id,qty,expiry,received_at) VALUES (?1,?2,?3,?4)",
        params![pid, qty, expiry, util::now()],
    )?;
    conn.execute("UPDATE san_pham SET stock=stock+?1 WHERE id=?2", params![qty, pid])?;
    Ok(())
}

/// Trừ tồn theo thứ tự hạn dùng gần nhất trước (FIFO/FEFO).
/// `sellable_only` = true: bỏ qua lô đã hết hạn (dùng khi bán hàng).
pub fn deduct(conn: &Connection, pid: i64, qty: i64, sellable_only: bool) -> rusqlite::Result<()> {
    if qty <= 0 {
        return Ok(());
    }
    let cutoff = if sellable_only { util::today() } else { "0000-00-00".to_string() };
    let batches: Vec<(i64, i64)> = {
        let mut st = conn.prepare(
            "SELECT id,qty FROM lo_hang WHERE product_id=?1 AND qty>0 AND (expiry IS NULL OR expiry>=?2)
             ORDER BY expiry IS NULL, expiry, id",
        )?;
        let rows = st.query_map(params![pid, cutoff], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    let mut remaining = qty;
    for (bid, bq) in batches {
        if remaining == 0 {
            break;
        }
        let take = remaining.min(bq);
        conn.execute("UPDATE lo_hang SET qty=qty-?1 WHERE id=?2", params![take, bid])?;
        remaining -= take;
    }
    if remaining > 0 {
        return Err(rusqlite::Error::StatementChangedRows(0));
    }
    conn.execute("UPDATE san_pham SET stock=stock-?1 WHERE id=?2", params![qty, pid])?;
    Ok(())
}
