//! Kiểu dữ liệu dùng chung giữa giao diện (Leptos/WASM) và lõi (Tauri/Rust).
//! Quy ước: mỗi lệnh Tauri nhận đúng một tham số `req` và trả về `Result<T, String>`.
use serde::{Deserialize, Serialize};

// ───────────────────────── Định dạng (mục 5.3.1) ─────────────────────────

/// 1234567 -> "1.234.567"
pub fn fmt_money(v: i64) -> String {
    let s = v.abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(ch);
    }
    if v < 0 {
        format!("-{out}")
    } else {
        out
    }
}

/// "2026-10-28 14:05:00" -> "28/10/2026 14:05"
pub fn fmt_dt(s: &str) -> String {
    if s.len() >= 16 && s.is_ascii() {
        format!("{}/{}/{} {}", &s[8..10], &s[5..7], &s[0..4], &s[11..16])
    } else {
        fmt_date(s)
    }
}

/// "2026-10-28" -> "28/10/2026"
pub fn fmt_date(s: &str) -> String {
    if s.len() >= 10 && s.is_ascii() {
        format!("{}/{}/{}", &s[8..10], &s[5..7], &s[0..4])
    } else {
        s.to_string()
    }
}

/// Bỏ dấu tiếng Việt + chữ thường, phục vụ tìm kiếm không phân biệt dấu.
pub fn fold(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'ạ' | 'ả' | 'ã' | 'â' | 'ầ' | 'ấ' | 'ậ' | 'ẩ' | 'ẫ' | 'ă' | 'ằ' | 'ắ' | 'ặ' | 'ẳ' | 'ẵ' => 'a',
            'è' | 'é' | 'ẹ' | 'ẻ' | 'ẽ' | 'ê' | 'ề' | 'ế' | 'ệ' | 'ể' | 'ễ' => 'e',
            'ì' | 'í' | 'ị' | 'ỉ' | 'ĩ' => 'i',
            'ò' | 'ó' | 'ọ' | 'ỏ' | 'õ' | 'ô' | 'ồ' | 'ố' | 'ộ' | 'ổ' | 'ỗ' | 'ơ' | 'ờ' | 'ớ' | 'ợ' | 'ở' | 'ỡ' => 'o',
            'ù' | 'ú' | 'ụ' | 'ủ' | 'ũ' | 'ư' | 'ừ' | 'ứ' | 'ự' | 'ử' | 'ữ' => 'u',
            'ỳ' | 'ý' | 'ỵ' | 'ỷ' | 'ỹ' => 'y',
            'đ' => 'd',
            c => c,
        })
        .collect()
}

/// `q` rỗng => khớp tất cả; ngược lại khớp nếu có trường chứa `q` (không phân biệt dấu/hoa thường).
pub fn matches_query(q: &str, fields: &[&str]) -> bool {
    let q = fold(q.trim());
    q.is_empty() || fields.iter().any(|f| fold(f).contains(&q))
}

// ───────────────────────── Hằng số trạng thái ─────────────────────────

pub mod status {
    pub const PN_PENDING: &str = "Chờ xác nhận";
    pub const PN_CONFIRMED: &str = "Đã xác nhận";
    pub const PN_REJECTED: &str = "Cần chỉnh sửa";
    pub const PN_CANCELLED: &str = "Đã hủy";

    pub const KK_COUNTING: &str = "Đang kiểm";
    pub const KK_PENDING: &str = "Chờ duyệt";
    pub const KK_ADJUSTED: &str = "Đã điều chỉnh";
    pub const KK_RECOUNT: &str = "Cần kiểm lại";
    pub const KK_DONE: &str = "Hoàn tất";

    pub const HD_PAID: &str = "Đã thanh toán";
}

/// Thông báo hệ thống dùng chung (mục 5.2)
pub mod msg {
    pub const MSG01: &str = "Không tìm thấy kết quả phù hợp.";
    pub const MSG08: &str = "Bạn không có quyền thực hiện chức năng này.";
    pub const MSG09: &str = "Tên đăng nhập hoặc mật khẩu không đúng. Vui lòng kiểm tra lại.";
    pub const MSG10: &str = "Vui lòng nhập tên đăng nhập và mật khẩu.";
    pub const MSG11: &str = "Tài khoản của bạn đã bị khóa. Vui lòng liên hệ Quản lý.";
    pub const MSG12: &str = "Bạn đã nhập sai 5 lần liên tiếp. Tài khoản bị khóa trong 30 phút.";
    pub const MSG13: &str = "Mật khẩu hiện tại không đúng.";
    pub const MSG14: &str = "Đổi mật khẩu thành công.";
    pub const MSG15: &str = "Số lượng phải là số nguyên lớn hơn 0.";
    pub const MSG18: &str = "Hóa đơn chưa có sản phẩm nào. Vui lòng thêm sản phẩm.";
    pub const MSG21: &str = "Vui lòng chọn nhà cung cấp hợp lệ.";
    pub const MSG23: &str = "Hạn sử dụng phải sau ngày nhập hàng.";
    pub const MSG24: &str = "Phiếu nhập phải có ít nhất một sản phẩm.";
    pub const MSG28: &str = "Số lượng thực tế phải là số nguyên lớn hơn hoặc bằng 0.";
    pub const MSG29: &str = "Vui lòng nhập lý do cho sản phẩm có chênh lệch.";
    pub const MSG31: &str = "Điều chỉnh tồn kho thành công.";
    pub const MSG34: &str = "Đã xảy ra lỗi. Vui lòng thử lại sau.";

    pub fn msg02(field: &str) -> String {
        format!("Trường {field} là bắt buộc.")
    }
    pub fn msg06(field: &str) -> String {
        format!("{field} đã tồn tại. Vui lòng nhập giá trị khác.")
    }
    pub fn msg16(name: &str, stock: i64, unit: &str) -> String {
        format!("Sản phẩm {name} chỉ còn {stock} {unit} trong kho.")
    }
    pub fn msg17(name: &str) -> String {
        format!("Sản phẩm {name} đã hết hạn sử dụng, không được bán. Vui lòng báo Quản lý.")
    }
    pub fn msg19(missing: i64) -> String {
        format!("Số tiền khách trả chưa đủ. Còn thiếu {} VNĐ.", super::fmt_money(missing))
    }
    pub fn msg20(change: i64) -> String {
        format!("Thanh toán thành công. Tiền thừa trả khách: {} VNĐ.", super::fmt_money(change))
    }
    pub fn msg25(code: &str) -> String {
        format!("Tạo phiếu nhập {code} thành công. Phiếu đang chờ Quản lý xác nhận.")
    }
    pub fn msg26(code: &str) -> String {
        format!("Đã xác nhận phiếu nhập {code}. Tồn kho đã được cập nhật.")
    }
    pub fn msg27(code: &str) -> String {
        format!("Đã từ chối phiếu nhập {code}. Phiếu được trả về để chỉnh sửa.")
    }
    pub fn msg30(n: i64) -> String {
        format!("Đã lưu phiếu kiểm kê. Có {n} sản phẩm chênh lệch.")
    }
}

// ───────────────────────── Chung ─────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Empty {}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IdReq {
    pub id: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetActiveReq {
    pub id: i64,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReasonReq {
    pub id: i64,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Saved {
    pub id: i64,
    pub code: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SearchReq {
    pub search: Option<String>,
}

// ───────────────────────── Xác thực & tài khoản ─────────────────────────

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Manager,
    Staff,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Manager => "manager",
            Role::Staff => "staff",
        }
    }
    pub fn from_db(s: &str) -> Role {
        if s == "manager" {
            Role::Manager
        } else {
            Role::Staff
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Role::Manager => "Quản lý",
            Role::Staff => "Nhân viên",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub full_name: String,
    pub role: Role,
}

impl UserInfo {
    pub fn is_manager(&self) -> bool {
        self.role == Role::Manager
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginReq {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChangePasswordReq {
    pub current: String,
    pub new: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserRow {
    pub id: i64,
    pub username: String,
    pub full_name: String,
    pub role: Role,
    pub active: bool,
    pub locked_until: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveUserReq {
    pub id: Option<i64>,
    pub username: String,
    pub full_name: String,
    pub role: Role,
    /// Bắt buộc khi tạo mới; khi cập nhật, nếu có sẽ đặt lại mật khẩu.
    pub password: Option<String>,
}

// ───────────────────────── Danh mục / sản phẩm ─────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Category {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveCategoryReq {
    pub id: Option<i64>,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Product {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub category_id: i64,
    pub category_name: String,
    pub unit: String,
    pub price: i64,
    pub stock: i64,
    pub min_stock: i64,
    pub has_expiry: bool,
    pub active: bool,
    /// Số lượng còn bán được (không tính hàng đã hết hạn)
    pub sellable: i64,
    pub expired_qty: i64,
    pub nearest_expiry: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProductQuery {
    pub search: Option<String>,
    pub category_id: Option<i64>,
    pub only_active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveProductReq {
    pub id: Option<i64>,
    pub code: String,
    pub name: String,
    pub category_id: i64,
    pub unit: String,
    pub price: i64,
    pub min_stock: i64,
    pub has_expiry: bool,
    /// Chỉ dùng khi tạo mới: tồn đầu kỳ
    pub initial_stock: i64,
    pub initial_expiry: Option<String>,
}

// ───────────────────────── Bán hàng ─────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CartLine {
    pub product_id: i64,
    pub quantity: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CheckoutReq {
    pub lines: Vec<CartLine>,
    /// "cash" | "transfer"
    pub method: String,
    pub paid: i64,
}

pub fn method_label(m: &str) -> &'static str {
    if m == "transfer" {
        "Chuyển khoản"
    } else {
        "Tiền mặt"
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InvoiceRow {
    pub id: i64,
    pub code: String,
    pub created_at: String,
    pub created_by_name: String,
    pub method: String,
    pub total: i64,
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InvoiceLine {
    pub code: String,
    pub name: String,
    pub unit: String,
    pub quantity: i64,
    pub unit_price: i64,
    pub amount: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InvoiceDetail {
    pub row: InvoiceRow,
    pub paid: i64,
    pub change: i64,
    pub lines: Vec<InvoiceLine>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InvoiceQuery {
    pub search: Option<String>,
    /// yyyy-MM-dd
    pub from: Option<String>,
    pub to: Option<String>,
}

// ───────────────────────── Nhập hàng ─────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Supplier {
    pub id: i64,
    pub name: String,
    pub phone: String,
    pub address: String,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SupplierQuery {
    pub search: Option<String>,
    pub only_active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveSupplierReq {
    pub id: Option<i64>,
    pub name: String,
    pub phone: String,
    pub address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PurchaseLineIn {
    pub product_id: i64,
    pub quantity: i64,
    pub unit_cost: i64,
    pub expiry: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SavePurchaseReq {
    /// Có giá trị khi sửa lại phiếu ở trạng thái "Cần chỉnh sửa"
    pub id: Option<i64>,
    pub supplier_id: Option<i64>,
    pub lines: Vec<PurchaseLineIn>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PurchaseRow {
    pub id: i64,
    pub code: String,
    pub supplier_name: String,
    pub created_by_name: String,
    pub created_at: String,
    pub confirmed_by_name: Option<String>,
    pub confirmed_at: Option<String>,
    pub total: i64,
    pub status: String,
    pub note: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PurchaseLineOut {
    pub product_id: i64,
    pub code: String,
    pub name: String,
    pub unit: String,
    pub has_expiry: bool,
    pub quantity: i64,
    pub unit_cost: i64,
    pub expiry: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PurchaseDetail {
    pub row: PurchaseRow,
    pub supplier_id: i64,
    pub lines: Vec<PurchaseLineOut>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PurchaseQuery {
    pub status: Option<String>,
    pub supplier_id: Option<i64>,
    pub from: Option<String>,
    pub to: Option<String>,
}

// ───────────────────────── Tồn kho / cảnh báo ─────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LowStockItem {
    pub code: String,
    pub name: String,
    pub unit: String,
    pub stock: i64,
    pub min_stock: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExpiryItem {
    pub code: String,
    pub name: String,
    pub unit: String,
    pub quantity: i64,
    pub expiry: String,
    pub days_left: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Alerts {
    pub low: Vec<LowStockItem>,
    pub near_expiry: Vec<ExpiryItem>,
    pub expiry_days: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetDaysReq {
    pub days: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Dashboard {
    pub low_count: i64,
    pub expiry_count: i64,
    pub pending_purchases: i64,
    pub pending_stocktakes: i64,
    /// Chỉ Quản lý
    pub today_revenue: Option<i64>,
    pub today_invoices: i64,
}

// ───────────────────────── Kiểm kê ─────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StocktakeRow {
    pub id: i64,
    pub code: String,
    pub created_by_name: String,
    pub created_at: String,
    pub approved_by_name: Option<String>,
    pub approved_at: Option<String>,
    pub status: String,
    pub diff_count: i64,
    pub note: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StocktakeLine {
    pub product_id: i64,
    pub code: String,
    pub name: String,
    pub unit: String,
    pub system_qty: i64,
    pub actual_qty: Option<i64>,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StocktakeDetail {
    pub row: StocktakeRow,
    pub lines: Vec<StocktakeLine>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StocktakeLineIn {
    pub product_id: i64,
    pub actual: Option<i64>,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveStocktakeReq {
    pub id: i64,
    pub lines: Vec<StocktakeLineIn>,
    /// false = Lưu tạm; true = Hoàn tất kiểm kê
    pub complete: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StocktakeSaved {
    pub id: i64,
    pub code: String,
    pub status: String,
    pub diff_count: i64,
}

// ───────────────────────── Báo cáo / nhật ký ─────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RevenueReq {
    pub from: String,
    pub to: String,
    /// "day" | "month"
    pub group: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RevenueRow {
    pub period: String,
    pub invoices: i64,
    pub revenue: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RevenueReport {
    pub rows: Vec<RevenueRow>,
    pub total_revenue: i64,
    pub total_invoices: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogRow {
    pub at: String,
    pub username: String,
    pub action: String,
    pub detail: String,
}
