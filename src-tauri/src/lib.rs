mod auth;
mod catalog;
pub mod db;
mod inv;
mod purchase;
mod sales;
mod stock;
mod util;

use rusqlite::Connection;
use simod_shared::{msg, UserInfo};
use std::sync::{Mutex, MutexGuard};
use tauri::Manager;

pub struct AppState {
    conn: Mutex<Connection>,
    data_dir: std::path::PathBuf,
    session: Mutex<Option<UserInfo>>,
}

impl AppState {
    pub fn new(conn: Connection, data_dir: std::path::PathBuf) -> Self {
        AppState { conn: Mutex::new(conn), data_dir, session: Mutex::new(None) }
    }
    pub fn db(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn session(&self) -> MutexGuard<'_, Option<UserInfo>> {
        self.session.lock().unwrap_or_else(|e| e.into_inner())
    }
    /// Yêu cầu đã đăng nhập.
    pub fn user(&self) -> Result<UserInfo, String> {
        self.session().clone().ok_or_else(|| "Vui lòng đăng nhập để tiếp tục.".to_string())
    }
    /// Yêu cầu quyền Quản lý (BR-01).
    pub fn manager(&self) -> Result<UserInfo, String> {
        let u = self.user()?;
        if u.is_manager() {
            Ok(u)
        } else {
            Err(msg::MSG08.to_string())
        }
    }
}

/// Danh sách lệnh Tauri (dùng chung cho ứng dụng và test).
pub fn handler<R: tauri::Runtime>() -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
            auth::login,
            auth::logout,
            auth::current_user,
            auth::change_password,
            catalog::list_users,
            catalog::save_user,
            catalog::set_user_active,
            catalog::list_categories,
            catalog::save_category,
            catalog::list_products,
            catalog::save_product,
            catalog::set_product_active,
            catalog::list_suppliers,
            catalog::save_supplier,
            catalog::set_supplier_active,
            sales::checkout,
            sales::list_invoices,
            sales::get_invoice,
            purchase::save_purchase,
            purchase::list_purchases,
            purchase::get_purchase,
            purchase::confirm_purchase,
            purchase::reject_purchase,
            purchase::cancel_purchase,
            stock::get_alerts,
            stock::set_expiry_days,
            stock::get_dashboard,
            stock::create_stocktake,
            stock::list_stocktakes,
            stock::get_stocktake,
            stock::save_stocktake,
            stock::approve_stocktake,
            stock::request_recount,
            stock::revenue_report,
            stock::list_logs,
            stock::backup_now,
    ]
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join("simod.db"))?;
            db::backup_daily(&conn, &dir);
            app.manage(AppState::new(conn, dir));
            Ok(())
        })
        .invoke_handler(handler())
        .run(tauri::generate_context!())
        .expect("không thể chạy ứng dụng Simod");
}
