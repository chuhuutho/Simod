//! S02 – Trang chủ
use crate::api::fetch;
use crate::ctx::*;
use crate::icons::Icon;
use leptos::prelude::*;
use simod_shared::*;

fn today_vn_label() -> String {
    let d = js_sys::Date::new_0();
    let wd = ["Chủ nhật", "Thứ hai", "Thứ ba", "Thứ tư", "Thứ năm", "Thứ sáu", "Thứ bảy"][d.get_day() as usize % 7];
    format!("{wd}, {:02}/{:02}/{}", d.get_date(), d.get_month() + 1, d.get_full_year())
}

#[component]
pub fn HomePage() -> impl IntoView {
    let c = ctx();
    let dash = RwSignal::new(None::<Dashboard>);
    fetch(c, "get_dashboard", Empty {}, move |d: Dashboard| dash.set(Some(d)));
    let name = c.user.get_untracked().map(|u| u.full_name).unwrap_or_default();
    let manager = c.is_manager();

    let tile = move |icon: &'static str, label: &'static str, value: String, warn: bool, go: Page| {
        view! {
            <button class="tile" class:warn=warn on:click=move |_| c.go(go)>
                <span class="tile-ico"><Icon name=icon size=20/></span>
                <span class="tile-label">{label}</span>
                <span class="tile-val">{value}</span>
            </button>
        }
    };

    view! {
        <div class="hello">
            <div class="eyebrow">{today_vn_label()}</div>
            <h2>"Xin chào, "{name}</h2>
        </div>
        {move || dash.get().map(|d| view! {
            <div class="tiles">
                {tile("receipt", "Hóa đơn hôm nay", d.today_invoices.to_string(), false, Page::Invoices)}
                {d.today_revenue.map(|r| tile("coins", "Doanh thu hôm nay (VNĐ)", fmt_money(r), false, Page::Revenue))}
                {tile("trend-down", "Sản phẩm sắp hết hàng", d.low_count.to_string(), d.low_count > 0, Page::Alerts)}
                {tile("hourglass", "Hàng cận date / hết hạn", d.expiry_count.to_string(), d.expiry_count > 0, Page::Alerts)}
                {tile("truck", if manager { "Phiếu nhập chờ xác nhận" } else { "Phiếu nhập của tôi chờ xác nhận" },
                      d.pending_purchases.to_string(), d.pending_purchases > 0 && manager, Page::Purchases)}
                {tile("check", if manager { "Phiếu kiểm kê chờ duyệt" } else { "Phiếu kiểm kê cần hoàn tất" },
                      d.pending_stocktakes.to_string(), d.pending_stocktakes > 0, Page::Stocktakes)}
            </div>
        })}
        <div class="eyebrow section-label">"Thao tác nhanh"</div>
        <div class="quick">
            <button class="btn primary big" on:click=move |_| c.go(Page::Sales)><Icon name="receipt" size=18/>"Bán hàng"</button>
            <button class="btn big" on:click=move |_| c.go(Page::Purchase(None))><Icon name="truck" size=18/>"Tạo phiếu nhập"</button>
            <button class="btn big" on:click=move |_| c.go(Page::Stock)><Icon name="layers" size=18/>"Xem tồn kho"</button>
            <button class="btn big" on:click=move |_| c.go(Page::Stocktakes)><Icon name="check" size=18/>"Kiểm kê kho"</button>
        </div>
    }
}
