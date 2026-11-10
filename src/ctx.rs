//! Trạng thái toàn cục và các hàm tiện ích của giao diện.
use leptos::prelude::*;
use simod_shared::{Product, UserInfo};
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Page {
    Home,
    Password,
    Products,
    Sales,
    Invoices,
    Invoice(i64),
    Suppliers,
    Purchases,
    /// None = tạo phiếu mới
    Purchase(Option<i64>),
    Stock,
    Alerts,
    Stocktakes,
    Stocktake(i64),
    Revenue,
    Users,
    Logs,
}

#[derive(Clone, PartialEq)]
pub struct Toast {
    pub ok: bool,
    pub text: String,
}

/// Một dòng trong hóa đơn đang lập (UC09). Giữ ở trạng thái toàn cục để không mất khi đổi trang.
#[derive(Clone)]
pub struct CartItem {
    pub p: Product,
    pub qty: ArcRwSignal<i64>,
}

#[derive(Clone, Copy)]
pub struct Ctx {
    pub cart: RwSignal<Vec<CartItem>>,
    pub user: RwSignal<Option<UserInfo>>,
    pub page: RwSignal<Page>,
    pub toast: RwSignal<Option<Toast>>,
}

impl Ctx {
    fn show(&self, ok: bool, text: String) {
        self.toast.set(Some(Toast { ok, text }));
        let t = self.toast;
        set_timeout(move || t.set(None), Duration::from_secs(4));
    }
    pub fn ok(&self, text: impl Into<String>) {
        self.show(true, text.into());
    }
    pub fn err(&self, text: impl Into<String>) {
        self.show(false, text.into());
    }
    pub fn go(&self, p: Page) {
        self.page.set(p);
    }
    pub fn is_manager(&self) -> bool {
        self.user.get_untracked().map_or(false, |u| u.is_manager())
    }
}

pub fn ctx() -> Ctx {
    expect_context::<Ctx>()
}

/// "1.234.567" / "1,234" / " 50 000 " -> số nguyên
pub fn parse_num(s: &str) -> Option<i64> {
    let t: String = s.chars().filter(|c| !matches!(c, '.' | ',' | ' ')).collect();
    t.parse().ok()
}

fn pad(n: u32) -> String {
    format!("{n:02}")
}

/// Ngày hôm nay (yyyy-MM-dd) theo giờ máy.
pub fn today_str() -> String {
    let d = js_sys::Date::new_0();
    format!("{}-{}-{}", d.get_full_year(), pad(d.get_month() + 1), pad(d.get_date()))
}

pub fn month_start_str() -> String {
    let d = js_sys::Date::new_0();
    format!("{}-{}-01", d.get_full_year(), pad(d.get_month() + 1))
}

pub fn print_page() {
    if let Some(w) = web_sys::window() {
        let _ = w.print();
    }
}
