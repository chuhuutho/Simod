//! S19 – Báo cáo doanh thu (UC20)
use crate::api::fetch;
use crate::ctx::*;
use crate::ui::*;
use leptos::prelude::*;
use simod_shared::*;

#[component]
pub fn RevenuePage() -> impl IntoView {
    let c = ctx();
    let from = RwSignal::new(month_start_str());
    let to = RwSignal::new(today_str());
    let group = RwSignal::new("day".to_string());
    let report = RwSignal::new(None::<RevenueReport>);
    let invoices = RwSignal::new(Vec::<InvoiceRow>::new());

    Effect::new(move |_| {
        let req = RevenueReq { from: from.get(), to: to.get(), group: group.get() };
        fetch(c, "revenue_report", req, move |r: RevenueReport| report.set(Some(r)));
        let q = InvoiceQuery { search: None, from: Some(from.get()), to: Some(to.get()) };
        fetch(c, "list_invoices", q, move |v: Vec<InvoiceRow>| invoices.set(v));
    });

    let period_label = move |p: &str| if p.len() == 7 { format!("{}/{}", &p[5..7], &p[0..4]) } else { fmt_date(p) };

    view! {
        <PageHead title="Báo cáo doanh thu"><span/></PageHead>
        <div class="toolbar">
            <label class="inline">"Từ ngày"<input type="date" prop:value=move || from.get() on:input=move |ev| from.set(event_target_value(&ev))/></label>
            <label class="inline">"Đến ngày"<input type="date" prop:value=move || to.get() on:input=move |ev| to.set(event_target_value(&ev))/></label>
            <div class="seg">
                <button class="btn" class:primary=move || group.get() == "day" on:click=move |_| group.set("day".into())>"Theo ngày"</button>
                <button class="btn" class:primary=move || group.get() == "month" on:click=move |_| group.set("month".into())>"Theo tháng"</button>
            </div>
        </div>
        {move || report.get().map(|r| view! {
            <div class="tiles">
                <div class="tile static"><div class="tile-val">{fmt_money(r.total_revenue)}</div><div class="tile-label">"Tổng doanh thu (VNĐ)"</div></div>
                <div class="tile static"><div class="tile-val">{r.total_invoices}</div><div class="tile-label">"Số hóa đơn đã thanh toán"</div></div>
                <div class="tile static"><div class="tile-val">{if r.total_invoices > 0 { fmt_money(r.total_revenue / r.total_invoices) } else { "0".into() }}</div><div class="tile-label">"Trung bình / hóa đơn (VNĐ)"</div></div>
            </div>
            <div class="card table-card">
                <table class="tbl">
                    <thead><tr><th>{move || if group.get() == "month" { "Tháng" } else { "Ngày" }}</th><th class="num">"Số hóa đơn"</th><th class="num">"Doanh thu (VNĐ)"</th></tr></thead>
                    <tbody>{r.rows.iter().map(|x| view! {
                        <tr><td>{period_label(&x.period)}</td><td class="num">{x.invoices}</td><td class="num">{fmt_money(x.revenue)}</td></tr>
                    }).collect_view()}</tbody>
                </table>
                {r.rows.is_empty().then(|| view! { <p class="empty">{msg::MSG01}</p> })}
            </div>
        })}
        <h3 class="section">"Chi tiết hóa đơn trong kỳ"</h3>
        <div class="card table-card">
            <table class="tbl clickable">
                <thead><tr><th>"Mã hóa đơn"</th><th>"Thời gian"</th><th>"Người lập"</th><th>"Hình thức"</th><th class="num">"Tổng tiền (VNĐ)"</th></tr></thead>
                <tbody>{move || invoices.get().into_iter().map(|r| {
                    let id = r.id;
                    view! {
                        <tr on:click=move |_| c.go(Page::Invoice(id))>
                            <td class="mono">{r.code.clone()}</td><td>{fmt_dt(&r.created_at)}</td><td>{r.created_by_name.clone()}</td>
                            <td>{method_label(&r.method)}</td><td class="num">{fmt_money(r.total)}</td>
                        </tr>
                    }
                }).collect_view()}</tbody>
            </table>
            <NoResult show=Signal::derive(move || invoices.get().is_empty())/>
        </div>
    }
}
