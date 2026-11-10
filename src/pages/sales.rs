//! S06/S07 – Bán hàng & thanh toán (UC09, UC10)
use crate::api::{fetch, fetch_or};
use crate::ctx::*;
use crate::icons::Icon;
use crate::ui::*;
use leptos::prelude::*;
use simod_shared::*;

/// BR-06, BR-07: số nguyên > 0, còn hạn, không vượt tồn.
fn check_qty(p: &Product, q: i64) -> Result<(), String> {
    if q <= 0 {
        return Err(msg::MSG15.into());
    }
    if q > p.sellable {
        return Err(if p.sellable <= 0 && p.expired_qty > 0 {
            msg::msg17(&p.name)
        } else {
            msg::msg16(&p.name, p.sellable.max(0), &p.unit)
        });
    }
    Ok(())
}

#[component]
pub fn SalesPage() -> impl IntoView {
    let c = ctx();
    let products = RwSignal::new(Vec::<Product>::new());
    let search = RwSignal::new(String::new());
    let cart = c.cart;
    let paying = RwSignal::new(false);
    let receipt = RwSignal::new(None::<InvoiceDetail>);
    let tick = RwSignal::new(0u32);

    Effect::new(move |_| {
        tick.get();
        fetch(
            c,
            "list_products",
            ProductQuery { search: None, category_id: None, only_active: true },
            move |v: Vec<Product>| products.set(v),
        );
    });

    let results = move || {
        let q = search.get();
        products
            .get()
            .into_iter()
            .filter(|p| matches_query(&q, &[&p.name, &p.code]))
            .take(40)
            .collect::<Vec<_>>()
    };
    let total = move || cart.get().iter().map(|i| i.p.price * i.qty.get()).sum::<i64>();

    let add = move |p: Product| {
        let found = cart.get_untracked().into_iter().find(|i| i.p.id == p.id);
        match found {
            Some(item) => {
                let next = item.qty.get_untracked() + 1;
                match check_qty(&item.p, next) {
                    Ok(()) => item.qty.set(next),
                    Err(e) => c.err(e),
                }
            }
            None => match check_qty(&p, 1) {
                Ok(()) => cart.update(|v| v.push(CartItem { p, qty: ArcRwSignal::new(1) })),
                Err(e) => c.err(e),
            },
        }
    };

    let to_pay = move |_| {
        if cart.get_untracked().is_empty() {
            c.err(msg::MSG18);
        } else {
            paying.set(true);
        }
    };

    view! {
        <PageHead title="Bán hàng">
            <button class="btn danger" on:click=move |_| cart.set(Vec::new())>"Hủy hóa đơn"</button>
        </PageHead>
        <div class="sales">
            <section class="card pick">
                <input class="search" type="search" autofocus placeholder="Tìm theo tên / mã, hoặc quét mã rồi Enter…"
                    prop:value=move || search.get() on:input=move |ev| search.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() != "Enter" { return; }
                        let q = fold(search.get_untracked().trim());
                        let list = results();
                        let hit = list.iter().find(|p| fold(&p.code) == q).cloned()
                            .or_else(|| if list.len() == 1 { list.first().cloned() } else { None });
                        match hit {
                            Some(p) => { add(p); search.set(String::new()); }
                            None if !q.is_empty() => c.err(msg::MSG01),
                            None => {}
                        }
                    }/>
                <div class="pick-list">
                    {move || results().into_iter().map(|p| {
                        let pp = p.clone();
                        let out = p.sellable <= 0;
                        view! {
                            <button class="pick-item" class:out=out on:click=move |_| add(pp.clone())>
                                <div class="pick-name">{p.name.clone()}</div>
                                <div class="muted small">{p.code.clone()}" · Còn "{p.sellable.max(0)}" "{p.unit.clone()}</div>
                                <div class="pick-price">{fmt_money(p.price)}" ₫"</div>
                            </button>
                        }
                    }).collect_view()}
                    <NoResult show=Signal::derive(move || results().is_empty())/>
                </div>
            </section>

            <section class="card cart">
                <h3>"Hóa đơn"</h3>
                <table class="tbl">
                    <thead><tr><th>"Sản phẩm"</th><th class="num">"Đơn giá"</th><th class="qty-col">"Số lượng"</th><th class="num">"Thành tiền"</th><th></th></tr></thead>
                    <tbody>
                        <For each=move || cart.get() key=|i| i.p.id children=move |item| {
                            let qty = item.qty.clone();
                            let (q_dec, q_inc, q_amt) = (qty.clone(), qty.clone(), qty.clone());
                            let (p_in, p_inc) = (item.p.clone(), item.p.clone());
                            let id = item.p.id;
                            let price = item.p.price;
                            let (q_set, q_val) = (qty.clone(), qty.clone());
                            view! {
                                <tr>
                                    <td>{item.p.name.clone()}<div class="muted small">{item.p.code.clone()}</div></td>
                                    <td class="num">{fmt_money(price)}</td>
                                    <td class="qty-col">
                                        <div class="stepper">
                                            <button class="btn small" on:click=move |_| {
                                                let n = q_dec.get_untracked() - 1;
                                                if n <= 0 { cart.update(|v| v.retain(|i| i.p.id != id)); } else { q_dec.set(n); }
                                            }>"−"</button>
                                            <input type="text" inputmode="numeric" prop:value=move || q_val.get()
                                                on:change=move |ev| {
                                                    let raw = event_target_value(&ev);
                                                    match parse_num(&raw).map(|n| (n, check_qty(&p_in, n))) {
                                                        Some((n, Ok(()))) => q_set.set(n),
                                                        Some((_, Err(e))) => { c.err(e); q_set.update(|_| {}); }
                                                        None => { c.err(msg::MSG15); q_set.update(|_| {}); }
                                                    }
                                                }/>
                                            <button class="btn small" on:click=move |_| {
                                                let n = q_inc.get_untracked() + 1;
                                                match check_qty(&p_inc, n) { Ok(()) => q_inc.set(n), Err(e) => c.err(e) }
                                            }>"+"</button>
                                        </div>
                                    </td>
                                    <td class="num">{move || fmt_money(price * q_amt.get())}</td>
                                    <td><button class="icon-btn" title="Xóa dòng" on:click=move |_| cart.update(|v| v.retain(|i| i.p.id != id))><Icon name="trash" size=16/></button></td>
                                </tr>
                            }
                        }/>
                    </tbody>
                </table>
                <Show when=move || cart.get().is_empty()><p class="empty">"Chưa có sản phẩm. Chọn sản phẩm ở bên trái để thêm vào hóa đơn."</p></Show>
                <div class="cart-total">
                    <span>"Tổng tiền hàng"</span>
                    <b>{move || fmt_money(total())}" VNĐ"</b>
                </div>
                <button class="btn primary big block" on:click=to_pay>"Thanh toán"</button>
            </section>
        </div>

        {move || paying.get().then(|| {
            let lines = cart
                .get_untracked()
                .iter()
                .map(|i| CartLine { product_id: i.p.id, quantity: i.qty.get_untracked() })
                .collect::<Vec<_>>();
            let sum = total();
            view! {
                <PaymentModal total=sum lines=lines
                    on_close=move |_| paying.set(false)
                    on_done=move |d: InvoiceDetail| {
                        paying.set(false);
                        cart.set(Vec::new());
                        tick.update(|t| *t += 1);
                        c.ok(msg::msg20(d.change));
                        receipt.set(Some(d));
                    }/>
            }
        })}
        {move || receipt.get().map(|d| view! {
            <Modal title="Hóa đơn" on_close=move |_| receipt.set(None)>
                <InvoiceView detail=d/>
                <div class="actions no-print">
                    <button class="btn" on:click=move |_| print_page()><Icon name="printer" size=16/>"In hóa đơn"</button>
                    <button class="btn primary" on:click=move |_| receipt.set(None)>"Xong"</button>
                </div>
            </Modal>
        })}
    }
}

/// S07 – Thanh toán
#[component]
fn PaymentModal(
    total: i64,
    lines: Vec<CartLine>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_done: Callback<InvoiceDetail>,
) -> impl IntoView {
    let method = RwSignal::new("cash".to_string());
    let paid = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let paid_n = move || parse_num(&paid.get()).unwrap_or(0);
    let diff = move || paid_n() - total;

    let confirm = move |_| {
        if diff() < 0 {
            return error.set(msg::msg19(-diff()));
        }
        busy.set(true);
        let req = CheckoutReq { lines: lines.clone(), method: method.get(), paid: paid_n() };
        fetch_or("checkout", req, move |r: Result<InvoiceDetail, String>| {
            busy.set(false);
            match r {
                Ok(d) => on_done.run(d),
                Err(e) => error.set(e),
            }
        });
    };

    let quick = [50_000i64, 100_000, 200_000, 500_000];

    view! {
        <Modal title="Thanh toán" on_close=on_close>
            <div class="pay-total"><span>"Tổng tiền hàng"</span><b>{fmt_money(total)}" VNĐ"</b></div>
            <div class="seg">
                <button class="btn" class:primary=move || method.get() == "cash"
                    on:click=move |_| method.set("cash".into())><Icon name="cash" size=16/>"Tiền mặt"</button>
                <button class="btn" class:primary=move || method.get() == "transfer"
                    on:click=move |_| { method.set("transfer".into()); paid.set(total.to_string()); }><Icon name="bank" size=16/>"Chuyển khoản"</button>
            </div>
            <label class="form">
                {move || if method.get() == "cash" { "Số tiền khách trả (VNĐ)" } else { "Số tiền đã nhận qua chuyển khoản (VNĐ)" }}
                <input type="text" inputmode="numeric" class="big-input" autofocus
                    prop:value=move || paid.get() on:input=move |ev| { paid.set(event_target_value(&ev)); error.set(String::new()); }/>
            </label>
            <div class="quick-amounts">
                <button class="btn small" on:click=move |_| paid.set(total.to_string())>"Đủ tiền"</button>
                {quick.into_iter().map(|a| view! {
                    <button class="btn small" on:click=move |_| paid.set(a.to_string())>{fmt_money(a)}</button>
                }).collect_view()}
            </div>
            {move || if paid.get().is_empty() {
                view! { <div class="pay-change muted">"Nhập số tiền khách trả"</div> }.into_any()
            } else if diff() < 0 {
                view! { <div class="inline-err">{msg::msg19(-diff())}</div> }.into_any()
            } else {
                view! { <div class="pay-change">"Tiền thừa trả khách: "<b>{fmt_money(diff())}" VNĐ"</b></div> }.into_any()
            }}
            <Show when=move || method.get() == "transfer">
                <p class="muted small">"Chỉ xác nhận khi đã nhận được tiền chuyển khoản."</p>
            </Show>
            <Show when=move || !error.get().is_empty()><div class="inline-err">{move || error.get()}</div></Show>
            <div class="actions">
                <button class="btn" on:click=move |_| on_close.run(())>"Quay lại"</button>
                <button class="btn primary" disabled=move || busy.get() || paid.get().is_empty() || diff() < 0
                    on:click=confirm>"Xác nhận thanh toán"</button>
            </div>
        </Modal>
    }
}
