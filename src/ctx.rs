//! Trạng thái toàn cục và các hàm tiện ích của giao diện.
use leptos::prelude::*;
use simod_shared::{Product, UserInfo};
use std::cell::Cell;
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

/// Bốn mức thông báo nổi (toast).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ToastKind {
    Success,
    Error,
    Warning,
    Info,
}

impl ToastKind {
    pub fn class(self) -> &'static str {
        match self {
            ToastKind::Success => "ok",
            ToastKind::Error => "err",
            ToastKind::Warning => "warn",
            ToastKind::Info => "info",
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            ToastKind::Success => "Thành công",
            ToastKind::Error => "Có lỗi",
            ToastKind::Warning => "Cảnh báo",
            ToastKind::Info => "Thông báo",
        }
    }
    pub fn icon(self) -> &'static str {
        match self {
            ToastKind::Success => "tick",
            ToastKind::Error => "error",
            ToastKind::Warning => "alert",
            ToastKind::Info => "info",
        }
    }
    /// Lỗi đọc ngay (alert), các mức còn lại đọc lịch sự (status).
    pub fn role(self) -> &'static str {
        match self {
            ToastKind::Error => "alert",
            _ => "status",
        }
    }
    /// Thời gian hiển thị (ms).
    pub fn millis(self) -> u64 {
        match self {
            ToastKind::Success | ToastKind::Info => 3000,
            ToastKind::Warning => 5000,
            ToastKind::Error => 6000,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Toast {
    pub id: u64,
    pub kind: ToastKind,
    pub text: String,
}

/// Số toast tối đa hiển thị cùng lúc; toast cũ nhất bị đẩy ra.
const MAX_TOASTS: usize = 3;

thread_local! {
    static NEXT_TOAST: Cell<u64> = Cell::new(1);
}

fn next_toast_id() -> u64 {
    NEXT_TOAST.with(|c| {
        let id = c.get();
        c.set(id + 1);
        id
    })
}

/// Toast đang được rê chuột vào thì giữ lại, chưa tự tắt.
fn is_hovered(id: u64) -> bool {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.query_selector(&format!(".toast[data-id='{id}']:hover")).ok().flatten())
        .is_some()
}

fn schedule_dismiss(list: RwSignal<Vec<Toast>>, id: u64, ms: u64) {
    set_timeout(
        move || {
            if is_hovered(id) {
                schedule_dismiss(list, id, 1000);
            } else {
                list.update(|v| v.retain(|t| t.id != id));
            }
        },
        Duration::from_millis(ms),
    );
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
    pub toast: RwSignal<Vec<Toast>>,
}

impl Ctx {
    fn show(&self, kind: ToastKind, text: String) {
        let id = next_toast_id();
        self.toast.update(|v| {
            v.push(Toast { id, kind, text });
            let n = v.len();
            if n > MAX_TOASTS {
                v.drain(0..n - MAX_TOASTS);
            }
        });
        schedule_dismiss(self.toast, id, kind.millis());
    }
    pub fn ok(&self, text: impl Into<String>) {
        self.show(ToastKind::Success, text.into());
    }
    pub fn err(&self, text: impl Into<String>) {
        self.show(ToastKind::Error, text.into());
    }
    pub fn warn(&self, text: impl Into<String>) {
        self.show(ToastKind::Warning, text.into());
    }
    pub fn info(&self, text: impl Into<String>) {
        self.show(ToastKind::Info, text.into());
    }
    pub fn dismiss(&self, id: u64) {
        self.toast.update(|v| v.retain(|t| t.id != id));
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
