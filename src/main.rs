mod api;
mod ctx;
mod icons;
mod pages;
mod ui;

use ctx::{Ctx, Page};
use icons::Icon;
use leptos::prelude::*;
use simod_shared::*;

#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

/// Crate này chỉ chạy được dưới dạng WASM trong cửa sổ Tauri.
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("Đây là phần giao diện (WASM), không chạy trực tiếp bằng `cargo run`.");
    eprintln!("Hãy chạy ứng dụng bằng:  cargo tauri dev");
    std::process::exit(1);
}

#[component]
fn App() -> impl IntoView {
    let c = Ctx {
        cart: RwSignal::new(Vec::new()),
        user: RwSignal::new(None),
        page: RwSignal::new(Page::Home),
        toast: RwSignal::new(None),
    };
    provide_context(c);
    // Khôi phục phiên (nếu cửa sổ được tải lại)
    api::fetch(c, "current_user", Empty {}, move |u: Option<UserInfo>| c.user.set(u));

    view! {
        {move || match c.user.get() {
            None => pages::login::LoginPage().into_any(),
            Some(u) => view! { <Shell user=u/> }.into_any(),
        }}
        <Toaster/>
    }
}

#[component]
fn Toaster() -> impl IntoView {
    let c = ctx::ctx();
    view! {
        {move || c.toast.get().map(|t| view! {
            <div class="toast" class:ok=t.ok class:err=!t.ok>{t.text}</div>
        })}
    }
}

#[component]
fn Shell(user: UserInfo) -> impl IntoView {
    let c = ctx::ctx();
    let manager = user.is_manager();

    let nav = move |label: &'static str, icon: &'static str, p: Page, same: fn(&Page) -> bool| {
        view! {
            <button class="nav-item" class:active=move || same(&c.page.get()) on:click=move |_| c.go(p)>
                <Icon name=icon/><span>{label}</span>
            </button>
        }
    };

    let logout = move |_| {
        api::fetch(c, "logout", Empty {}, move |_: ()| {
            c.user.set(None);
            c.cart.set(Vec::new());
            c.go(Page::Home);
        });
    };

    view! {
        <div class="shell">
            <aside class="sidebar">
                <div class="brand"><span class="brand-mark"><Icon name="store" size=20/></span><span class="brand-name">"Simod"</span></div>
                <nav>
                    {nav("Trang chủ", "home", Page::Home, |p| matches!(p, Page::Home))}
                    <div class="nav-group">"Bán hàng"</div>
                    {nav("Bán hàng", "receipt", Page::Sales, |p| matches!(p, Page::Sales))}
                    {nav("Lịch sử hóa đơn", "history", Page::Invoices, |p| matches!(p, Page::Invoices | Page::Invoice(_)))}
                    <div class="nav-group">"Hàng hóa"</div>
                    {nav("Sản phẩm", "box", Page::Products, |p| matches!(p, Page::Products))}
                    {nav("Phiếu nhập hàng", "truck", Page::Purchases, |p| matches!(p, Page::Purchases | Page::Purchase(_)))}
                    {nav("Tồn kho", "layers", Page::Stock, |p| matches!(p, Page::Stock))}
                    {nav("Cảnh báo", "alert", Page::Alerts, |p| matches!(p, Page::Alerts))}
                    {nav("Kiểm kê kho", "check", Page::Stocktakes, |p| matches!(p, Page::Stocktakes | Page::Stocktake(_)))}
                    {manager.then(|| view! {
                        <div class="nav-group">"Quản trị"</div>
                        {nav("Nhà cung cấp", "factory", Page::Suppliers, |p| matches!(p, Page::Suppliers))}
                        {nav("Báo cáo doanh thu", "chart", Page::Revenue, |p| matches!(p, Page::Revenue))}
                        {nav("Nhân viên", "users", Page::Users, |p| matches!(p, Page::Users))}
                        {nav("Nhật ký hoạt động", "scroll", Page::Logs, |p| matches!(p, Page::Logs))}
                    })}
                </nav>
                <div class="user-box">
                    <div class="avatar">{user.full_name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()}</div>
                    <div class="user-name">{user.full_name.clone()}</div>
                    <div class="muted small">{user.role.label()}</div>
                    <div class="user-actions">
                        <button class="btn small ghost" on:click=move |_| c.go(Page::Password)><Icon name="key" size=14/>"Mật khẩu"</button>
                        <button class="btn small ghost" on:click=logout><Icon name="logout" size=14/>"Thoát"</button>
                    </div>
                </div>
            </aside>
            <main class="content">
                {move || match c.page.get() {
                    Page::Home => pages::home::HomePage().into_any(),
                    Page::Password => pages::auth::PasswordPage().into_any(),
                    Page::Products => pages::products::ProductsPage().into_any(),
                    Page::Sales => pages::sales::SalesPage().into_any(),
                    Page::Invoices => pages::invoices::InvoicesPage().into_any(),
                    Page::Invoice(id) => view! { <pages::invoices::InvoiceDetailPage id=id/> }.into_any(),
                    Page::Suppliers => pages::suppliers::SuppliersPage().into_any(),
                    Page::Purchases => pages::purchases::PurchasesPage().into_any(),
                    Page::Purchase(id) => view! { <pages::purchases::PurchasePage id=id/> }.into_any(),
                    Page::Stock => pages::stock::StockPage().into_any(),
                    Page::Alerts => pages::stock::AlertsPage().into_any(),
                    Page::Stocktakes => pages::stocktake::StocktakesPage().into_any(),
                    Page::Stocktake(id) => view! { <pages::stocktake::StocktakePage id=id/> }.into_any(),
                    Page::Revenue => pages::revenue::RevenuePage().into_any(),
                    Page::Users => pages::users::UsersPage().into_any(),
                    Page::Logs => pages::users::LogsPage().into_any(),
                }}
            </main>
        </div>
    }
}
