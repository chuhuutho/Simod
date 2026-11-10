//! S03 – Đổi mật khẩu (UC03)
use crate::api::fetch_or;
use crate::ctx::*;
use crate::ui::*;
use leptos::prelude::*;
use simod_shared::*;

#[component]
pub fn PasswordPage() -> impl IntoView {
    let c = ctx();
    let current = RwSignal::new(String::new());
    let new = RwSignal::new(String::new());
    let confirm = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if current.get().is_empty() {
            return error.set(msg::msg02("Mật khẩu hiện tại"));
        }
        if new.get().chars().count() < 6 {
            return error.set("Mật khẩu mới phải có ít nhất 6 ký tự.".into());
        }
        if new.get() != confirm.get() {
            return error.set("Mật khẩu xác nhận không khớp.".into());
        }
        error.set(String::new());
        let req = ChangePasswordReq { current: current.get(), new: new.get() };
        fetch_or("change_password", req, move |r: Result<(), String>| match r {
            Ok(()) => {
                c.ok(msg::MSG14);
                current.set(String::new());
                new.set(String::new());
                confirm.set(String::new());
            }
            Err(e) => error.set(e),
        });
    };

    view! {
        <PageHead title="Đổi mật khẩu"><span/></PageHead>
        <form class="card form narrow" on:submit=submit>
            <label>"Mật khẩu hiện tại"
                <input type="password" prop:value=move || current.get()
                    on:input=move |ev| current.set(event_target_value(&ev))/>
            </label>
            <label>"Mật khẩu mới"
                <input type="password" prop:value=move || new.get()
                    on:input=move |ev| new.set(event_target_value(&ev))/>
            </label>
            <label>"Nhập lại mật khẩu mới"
                <input type="password" prop:value=move || confirm.get()
                    on:input=move |ev| confirm.set(event_target_value(&ev))/>
            </label>
            <Show when=move || !error.get().is_empty()>
                <div class="inline-err">{move || error.get()}</div>
            </Show>
            <button class="btn primary" type="submit">"Đổi mật khẩu"</button>
        </form>
    }
}
