//! S14/S15 – Danh sách tồn kho & cảnh báo tồn kho / hạn sử dụng (UC16, UC17)
use crate::api::{fetch, fetch_or};
use crate::ctx::*;
use crate::ui::*;
use leptos::prelude::*;
use simod_shared::*;

#[component]
pub fn StockPage() -> impl IntoView {
    let c = ctx();
    let items = RwSignal::new(Vec::<Product>::new());
    let cats = RwSignal::new(Vec::<Category>::new());
    let search = RwSignal::new(String::new());
    let cat = RwSignal::new(0i64);
    let only_low = RwSignal::new(false);

    fetch(c, "list_categories", Empty {}, move |v: Vec<Category>| cats.set(v));
    fetch(c, "list_products", ProductQuery { search: None, category_id: None, only_active: true }, move |v: Vec<Product>| items.set(v));

    let rows = move || {
        let q = search.get();
        items
            .get()
            .into_iter()
            .filter(|p| cat.get() == 0 || p.category_id == cat.get())
            .filter(|p| !only_low.get() || p.stock <= p.min_stock)
            .filter(|p| matches_query(&q, &[&p.name, &p.code]))
            .collect::<Vec<_>>()
    };

    view! {
        <PageHead title="Tồn kho"><span/></PageHead>
        <div class="toolbar">
            <input class="search" type="search" placeholder="Tìm theo tên hoặc mã sản phẩm…" prop:value=move || search.get()
                on:input=move |ev| search.set(event_target_value(&ev))/>
            <select on:change=move |ev| cat.set(event_target_value(&ev).parse().unwrap_or(0))>
                <option value="0">"Tất cả danh mục"</option>
                {move || cats.get().into_iter().map(|c| view! { <option value=c.id.to_string()>{c.name}</option> }).collect_view()}
            </select>
            <label class="check"><input type="checkbox" prop:checked=move || only_low.get()
                on:change=move |ev| only_low.set(event_target_checked(&ev))/>"Chỉ hàng sắp hết"</label>
        </div>
        <div class="card table-card">
            <table class="tbl">
                <thead><tr><th>"Mã"</th><th>"Tên sản phẩm"</th><th>"Danh mục"</th><th>"ĐVT"</th>
                    <th class="num">"Tồn kho"</th><th class="num">"Ngưỡng tối thiểu"</th><th>"HSD gần nhất"</th><th>"Tình trạng"</th></tr></thead>
                <tbody>
                    {move || rows().into_iter().map(|p| {
                        let low = p.stock <= p.min_stock;
                        let expired = p.expired_qty > 0;
                                        view! {
                            <tr>
                                <td class="mono">{p.code.clone()}</td><td>{p.name.clone()}</td><td>{p.category_name.clone()}</td><td>{p.unit.clone()}</td>
                                <td class="num" class:low=low>{p.stock}</td><td class="num">{p.min_stock}</td>
                                <td>{p.nearest_expiry.as_deref().map(fmt_date).unwrap_or_else(|| "—".into())}</td>
                                <td>
                                    {low.then(|| view! { <span class="badge red">"Sắp hết"</span>" " })}
                                    {expired.then(|| view! { <span class="badge amber">{format!("Hết hạn: {}", p.expired_qty)}</span> })}
                                    {(!low && !expired).then(|| view! { <span class="badge green">"Bình thường"</span> })}
                                </td>
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
            <NoResult show=Signal::derive(move || rows().is_empty())/>
        </div>
    }
}

#[component]
pub fn AlertsPage() -> impl IntoView {
    let c = ctx();
    let manager = c.is_manager();
    let alerts = RwSignal::new(None::<Alerts>);
    let days = RwSignal::new(String::new());
    let tick = RwSignal::new(0u32);

    Effect::new(move |_| {
        tick.get();
        fetch(c, "get_alerts", Empty {}, move |a: Alerts| {
            days.set(a.expiry_days.to_string());
            alerts.set(Some(a));
        });
    });

    let save_days = move |_| {
        let Some(d) = parse_num(&days.get()) else { return c.err("Số ngày không hợp lệ.") };
        fetch_or("set_expiry_days", SetDaysReq { days: d }, move |r: Result<(), String>| match r {
            Ok(()) => { c.ok("Đã lưu cấu hình cảnh báo cận date."); tick.update(|t| *t += 1); }
            Err(e) => c.err(e),
        });
    };

    view! {
        <PageHead title="Cảnh báo tồn kho và hạn sử dụng"><span/></PageHead>
        {move || alerts.get().map(|a| view! {
            <div class="two-col">
                <section class="card">
                    <h3>"Hàng sắp hết "<span class="badge red">{a.low.len()}</span></h3>
                    <table class="tbl compact">
                        <thead><tr><th>"Sản phẩm"</th><th class="num">"Tồn"</th><th class="num">"Ngưỡng"</th></tr></thead>
                        <tbody>{a.low.iter().map(|i| view! {
                            <tr><td>{i.name.clone()}<div class="muted small">{msg32(i)}</div></td>
                            <td class="num low">{i.stock}" "{i.unit.clone()}</td><td class="num">{i.min_stock}</td></tr>
                        }).collect_view()}</tbody>
                    </table>
                    {a.low.is_empty().then(|| view! { <p class="empty">"Không có sản phẩm nào sắp hết hàng."</p> })}
                </section>
                <section class="card">
                    <h3>"Hàng cận date / hết hạn "<span class="badge amber">{a.near_expiry.len()}</span></h3>
                    {manager.then(|| view! {
                        <div class="inline-form">
                            <label class="inline">"Cảnh báo trước"<input class="days" type="text" inputmode="numeric" prop:value=move || days.get()
                                on:input=move |ev| days.set(event_target_value(&ev))/>"ngày"</label>
                            <button class="btn small" on:click=save_days>"Lưu"</button>
                        </div>
                    })}
                    <table class="tbl compact">
                        <thead><tr><th>"Sản phẩm"</th><th class="num">"SL"</th><th>"Hạn sử dụng"</th><th>"Còn lại"</th></tr></thead>
                        <tbody>{a.near_expiry.iter().map(|i| {
                            let expired = i.days_left < 0;
                            view! {
                                <tr>
                                    <td>{i.name.clone()}<div class="muted small">{i.code.clone()}</div></td>
                                    <td class="num">{i.quantity}" "{i.unit.clone()}</td>
                                    <td>{fmt_date(&i.expiry)}</td>
                                    <td>{if expired { view! { <span class="badge red">"Đã hết hạn"</span> }.into_any() }
                                         else { view! { <span class="badge amber">{format!("{} ngày", i.days_left)}</span> }.into_any() }}</td>
                                </tr>
                            }
                        }).collect_view()}</tbody>
                    </table>
                    {a.near_expiry.is_empty().then(|| view! { <p class="empty">"Không có hàng cận date."</p> })}
                </section>
            </div>
        })}
    }
}

/// MSG32
fn msg32(i: &LowStockItem) -> String {
    format!("Sản phẩm {} sắp hết hàng (còn {}, ngưỡng tối thiểu {}).", i.name, i.stock, i.min_stock)
}
