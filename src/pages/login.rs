//! S01 – Đăng nhập (UC01)
use crate::api::fetch_or;
use crate::ctx::*;
use crate::icons::Icon;
use leptos::prelude::*;
use simod_shared::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    let c = ctx();
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if username.get().trim().is_empty() || password.get().is_empty() {
            error.set(msg::MSG10.into());
            return;
        }
        busy.set(true);
        error.set(String::new());
        let req = LoginReq { username: username.get(), password: password.get() };
        fetch_or("login", req, move |r: Result<UserInfo, String>| {
            busy.set(false);
            match r {
                Ok(u) => {
                    password.set(String::new());
                    c.go(Page::Home);
                    c.user.set(Some(u));
                }
                Err(e) => error.set(e),
            }
        });
    };

    view! {
        <div class="login">
            <aside class="login-brand">
                <div class="brand"><span class="brand-mark"><Icon name="store" size=22/></span><span class="brand-name">"Simod"</span></div>
                <div class="login-pitch">
                    <h1>"Quầy tính tiền gọn,"<br/>"kho hàng rõ."</h1>
                    <p>"Quản lý bán hàng, nhập hàng và kiểm kê cho cửa hàng tạp hóa — thay cho sổ tay và máy tính bấm."</p>
                </div>
                <ul class="login-points">
                    <li><Icon name="receipt" size=16/>"Tính tiền, trả tiền thừa, in hóa đơn"</li>
                    <li><Icon name="hourglass" size=16/>"Cảnh báo hàng sắp hết và cận date"</li>
                    <li><Icon name="shield" size=16/>"Phân quyền Quản lý / Nhân viên, có nhật ký"</li>
                </ul>
            </aside>
            <section class="login-form-wrap">
                <form class="login-form" on:submit=submit>
                    <h2>"Đăng nhập"</h2>
                    <p class="muted">"Dùng tài khoản do Quản lý cấp."</p>
                    <label>"Tên đăng nhập"
                        <input type="text" autofocus autocomplete="username"
                            prop:value=move || username.get()
                            on:input=move |ev| username.set(event_target_value(&ev))/>
                    </label>
                    <label>"Mật khẩu"
                        <input type="password" autocomplete="current-password"
                            prop:value=move || password.get()
                            on:input=move |ev| password.set(event_target_value(&ev))/>
                    </label>
                    <Show when=move || !error.get().is_empty()>
                        <div class="inline-err">{move || error.get()}</div>
                    </Show>
                    <button class="btn primary block big" type="submit" disabled=move || busy.get()>"Đăng nhập"</button>
                </form>
            </section>
        </div>
    }
}
