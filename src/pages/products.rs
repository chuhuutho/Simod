//! S04/S05 – Danh sách & chi tiết sản phẩm (UC05–UC08), danh mục
use crate::api::{fetch, fetch_or};
use crate::ctx::*;
use crate::ui::*;
use leptos::prelude::*;
use simod_shared::*;

#[component]
pub fn ProductsPage() -> impl IntoView {
    let c = ctx();
    let manager = c.is_manager();
    let items = RwSignal::new(Vec::<Product>::new());
    let cats = RwSignal::new(Vec::<Category>::new());
    let search = RwSignal::new(String::new());
    let cat = RwSignal::new(0i64);
    let show_inactive = RwSignal::new(false);
    let tick = RwSignal::new(0u32);
    // Some(None) = thêm mới, Some(Some(p)) = sửa
    let editing = RwSignal::new(None::<Option<Product>>);
    let cat_modal = RwSignal::new(false);
    let confirm = RwSignal::new(None::<Product>);

    Effect::new(move |_| {
        tick.get();
        fetch(c, "list_categories", Empty {}, move |v: Vec<Category>| cats.set(v));
        fetch(
            c,
            "list_products",
            ProductQuery { search: None, category_id: None, only_active: false },
            move |v: Vec<Product>| items.set(v),
        );
    });

    let rows = move || {
        items
            .get()
            .into_iter()
            .filter(|p| show_inactive.get() || p.active)
            .filter(|p| cat.get() == 0 || p.category_id == cat.get())
            .filter(|p| matches_query(&search.get(), &[&p.name, &p.code]))
            .collect::<Vec<_>>()
    };

    let toggle = move |p: Product, active: bool| {
        fetch(c, "set_product_active", SetActiveReq { id: p.id, active }, move |_: ()| {
            c.ok(if active { "Đã kinh doanh lại sản phẩm." } else { "Đã ngừng kinh doanh sản phẩm." });
            tick.update(|t| *t += 1);
        });
    };

    view! {
        <PageHead title="Sản phẩm">
            {manager.then(|| view! {
                <button class="btn" on:click=move |_| cat_modal.set(true)>"Danh mục"</button>
                <button class="btn primary" on:click=move |_| editing.set(Some(None))>"+ Thêm sản phẩm"</button>
            })}
        </PageHead>
        <div class="toolbar">
            <input class="search" type="search" placeholder="Tìm theo tên hoặc mã sản phẩm…"
                prop:value=move || search.get() on:input=move |ev| search.set(event_target_value(&ev))/>
            <select on:change=move |ev| cat.set(event_target_value(&ev).parse().unwrap_or(0))>
                <option value="0">"Tất cả danh mục"</option>
                {move || cats.get().into_iter().map(|c| view! { <option value=c.id.to_string()>{c.name}</option> }).collect_view()}
            </select>
            {manager.then(|| view! {
                <label class="check"><input type="checkbox" prop:checked=move || show_inactive.get()
                    on:change=move |ev| show_inactive.set(event_target_checked(&ev))/>"Hiện cả ngừng kinh doanh"</label>
            })}
        </div>
        <div class="card table-card">
            <table class="tbl">
                <thead><tr>
                    <th>"Mã"</th><th>"Tên sản phẩm"</th><th>"Danh mục"</th><th>"ĐVT"</th>
                    <th class="num">"Giá bán (VNĐ)"</th><th class="num">"Tồn"</th><th>"Trạng thái"</th>
                    {manager.then(|| view! { <th></th> })}
                </tr></thead>
                <tbody>
                    {move || rows().into_iter().map(|p| {
                        let p_edit = p.clone();
                        let p_off = p.clone();
                        let p_on = p.clone();
                        let low = p.stock <= p.min_stock;
                        view! {
                            <tr class:dim=!p.active>
                                <td class="mono">{p.code.clone()}</td>
                                <td>{p.name.clone()}</td>
                                <td>{p.category_name.clone()}</td>
                                <td>{p.unit.clone()}</td>
                                <td class="num">{fmt_money(p.price)}</td>
                                <td class="num" class:low=low>{p.stock}</td>
                                <td>{if p.active { view! { <span class="badge green">"Đang kinh doanh"</span> } } else { view! { <span class="badge gray">"Ngừng kinh doanh"</span> } }}</td>
                                {manager.then(|| view! {
                                    <td class="row-actions">
                                        <button class="btn small" on:click=move |_| editing.set(Some(Some(p_edit.clone())))>"Sửa"</button>
                                        {if p.active {
                                            view! { <button class="btn small danger" on:click=move |_| confirm.set(Some(p_off.clone()))>"Ngừng KD"</button> }.into_any()
                                        } else {
                                            view! { <button class="btn small" on:click=move |_| toggle(p_on.clone(), true)>"Kinh doanh lại"</button> }.into_any()
                                        }}
                                    </td>
                                })}
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
            <NoResult show=Signal::derive(move || rows().is_empty())/>
        </div>

        {move || editing.get().map(|prod| view! {
            <ProductForm product=prod cats=cats.get()
                on_close=move |_| editing.set(None)
                on_saved=move |_| { editing.set(None); tick.update(|t| *t += 1); }/>
        })}
        {move || cat_modal.get().then(|| view! {
            <CategoryModal cats=cats on_close=move |_| cat_modal.set(false) on_changed=move |_| tick.update(|t| *t += 1)/>
        })}
        {move || confirm.get().map(|p| {
            let name = p.name.clone();
            view! {
                <Confirm message=format!("Bạn có chắc chắn muốn ngừng kinh doanh sản phẩm {name}?")
                    on_cancel=move |_| confirm.set(None)
                    on_ok=move |_| { confirm.set(None); toggle(p.clone(), false); }/>
            }
        })}
    }
}

#[component]
fn ProductForm(
    product: Option<Product>,
    cats: Vec<Category>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_saved: Callback<()>,
) -> impl IntoView {
    let c = ctx();
    let id = product.as_ref().map(|p| p.id);
    let p = product.clone();
    let code = RwSignal::new(p.as_ref().map(|x| x.code.clone()).unwrap_or_default());
    let name = RwSignal::new(p.as_ref().map(|x| x.name.clone()).unwrap_or_default());
    let unit = RwSignal::new(p.as_ref().map(|x| x.unit.clone()).unwrap_or_default());
    let price = RwSignal::new(p.as_ref().map(|x| x.price.to_string()).unwrap_or_default());
    let min_stock = RwSignal::new(p.as_ref().map(|x| x.min_stock.to_string()).unwrap_or_else(|| "0".into()));
    let has_expiry = RwSignal::new(p.as_ref().map_or(false, |x| x.has_expiry));
    let category = RwSignal::new(p.as_ref().map(|x| x.category_id).or(cats.first().map(|c| c.id)).unwrap_or(0));
    let initial_stock = RwSignal::new("0".to_string());
    let initial_expiry = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());

    let save = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(price_n) = parse_num(&price.get()) else { return error.set(msg::msg02("Giá bán")) };
        let Some(min_n) = parse_num(&min_stock.get()) else { return error.set(msg::msg02("Ngưỡng tồn tối thiểu")) };
        let init_n = parse_num(&initial_stock.get()).unwrap_or(0);
        let req = SaveProductReq {
            id,
            code: code.get(),
            name: name.get(),
            category_id: category.get(),
            unit: unit.get(),
            price: price_n,
            min_stock: min_n,
            has_expiry: has_expiry.get(),
            initial_stock: init_n,
            initial_expiry: Some(initial_expiry.get()).filter(|s| !s.is_empty()),
        };
        fetch_or("save_product", req, move |r: Result<i64, String>| match r {
            Ok(_) => {
                c.ok(if id.is_some() { "Cập nhật sản phẩm thành công." } else { "Thêm sản phẩm thành công." });
                on_saved.run(());
            }
            Err(e) => error.set(e),
        });
    };

    view! {
        <Modal title=if id.is_some() { "Cập nhật sản phẩm" } else { "Thêm sản phẩm" } on_close=on_close>
            <form class="form grid2" on:submit=save>
                <label>"Mã sản phẩm *"<input type="text" prop:value=move || code.get() on:input=move |ev| code.set(event_target_value(&ev))/></label>
                <label>"Tên sản phẩm *"<input type="text" prop:value=move || name.get() on:input=move |ev| name.set(event_target_value(&ev))/></label>
                <label>"Danh mục *"
                    <select on:change=move |ev| category.set(event_target_value(&ev).parse().unwrap_or(0))>
                        {cats.into_iter().map(|c| { let cid = c.id; view! {
                            <option value=c.id.to_string() selected=move || category.get() == cid>{c.name}</option>
                        }}).collect_view()}
                    </select>
                </label>
                <label>"Đơn vị tính *"<input type="text" placeholder="chai, gói, lon…" prop:value=move || unit.get() on:input=move |ev| unit.set(event_target_value(&ev))/></label>
                <label>"Giá bán niêm yết (VNĐ) *"<input type="text" inputmode="numeric" prop:value=move || price.get() on:input=move |ev| price.set(event_target_value(&ev))/></label>
                <label>"Ngưỡng tồn tối thiểu"<input type="text" inputmode="numeric" prop:value=move || min_stock.get() on:input=move |ev| min_stock.set(event_target_value(&ev))/></label>
                <label class="check span2"><input type="checkbox" prop:checked=move || has_expiry.get() on:change=move |ev| has_expiry.set(event_target_checked(&ev))/>"Sản phẩm có hạn sử dụng"</label>
                {id.is_none().then(|| view! {
                    <label>"Tồn đầu kỳ"<input type="text" inputmode="numeric" prop:value=move || initial_stock.get() on:input=move |ev| initial_stock.set(event_target_value(&ev))/></label>
                    <Show when=move || has_expiry.get()>
                        <label>"Hạn sử dụng của tồn đầu kỳ"<input type="date" prop:value=move || initial_expiry.get() on:input=move |ev| initial_expiry.set(event_target_value(&ev))/></label>
                    </Show>
                })}
                <Show when=move || !error.get().is_empty()><div class="inline-err span2">{move || error.get()}</div></Show>
                <div class="actions span2">
                    <button type="button" class="btn" on:click=move |_| on_close.run(())>"Hủy bỏ"</button>
                    <button type="submit" class="btn primary">"Lưu"</button>
                </div>
            </form>
        </Modal>
    }
}

#[component]
fn CategoryModal(
    cats: RwSignal<Vec<Category>>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_changed: Callback<()>,
) -> impl IntoView {
    let c = ctx();
    let name = RwSignal::new(String::new());
    let edit_id = RwSignal::new(None::<i64>);
    let error = RwSignal::new(String::new());

    let save = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let req = SaveCategoryReq { id: edit_id.get(), name: name.get() };
        let was_edit = req.id.is_some();
        fetch_or("save_category", req, move |r: Result<i64, String>| match r {
            Ok(_) => {
                c.ok(if was_edit { "Cập nhật danh mục thành công." } else { "Thêm danh mục thành công." });
                name.set(String::new());
                edit_id.set(None);
                error.set(String::new());
                on_changed.run(());
            }
            Err(e) => error.set(e),
        });
    };

    view! {
        <Modal title="Danh mục sản phẩm" on_close=on_close>
            <form class="inline-form" on:submit=save>
                <input type="text" placeholder="Tên danh mục" prop:value=move || name.get() on:input=move |ev| name.set(event_target_value(&ev))/>
                <button class="btn primary" type="submit">{move || if edit_id.get().is_some() { "Cập nhật" } else { "Thêm" }}</button>
                <Show when=move || edit_id.get().is_some()>
                    <button type="button" class="btn" on:click=move |_| { edit_id.set(None); name.set(String::new()); }>"Bỏ chọn"</button>
                </Show>
            </form>
            <Show when=move || !error.get().is_empty()><div class="inline-err">{move || error.get()}</div></Show>
            <table class="tbl compact">
                <tbody>
                    {move || cats.get().into_iter().map(|cat| {
                        let cc = cat.clone();
                        view! {
                            <tr><td>{cat.name.clone()}</td>
                            <td class="row-actions"><button class="btn small" on:click=move |_| { edit_id.set(Some(cc.id)); name.set(cc.name.clone()); }>"Sửa"</button></td></tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
        </Modal>
    }
}
