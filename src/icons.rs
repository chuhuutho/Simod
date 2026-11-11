//! Bộ icon SVG nét mảnh (stroke) dùng thay emoji.
use leptos::prelude::*;

fn body(name: &str) -> &'static str {
    match name {
        "home" => r#"<path d="M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z"/>"#,
        "receipt" => r#"<path d="M5 3h14v18l-2.3-1.6L14.3 21 12 19.4 9.7 21l-2.4-1.6L5 21z"/><path d="M9 8h6M9 12h6M9 16h3"/>"#,
        "history" => r#"<path d="M4 12a8 8 0 1 0 2.6-5.9L4 8.5"/><path d="M4 4v4.5h4.5M12 8v4.2l2.8 1.8"/>"#,
        "box" => r#"<path d="M21 8 12 3 3 8v8l9 5 9-5z"/><path d="m3 8 9 5 9-5M12 13v8"/>"#,
        "truck" => r#"<path d="M3 6.5h11V16H3zM14 9.5h4l3 3.2V16h-7z"/><circle cx="7.2" cy="17.8" r="1.7"/><circle cx="17" cy="17.8" r="1.7"/>"#,
        "layers" => r#"<path d="m12 3 9 5-9 5-9-5z"/><path d="m3 12.5 9 5 9-5M3 16.5l9 5 9-5"/>"#,
        "alert" => r#"<path d="M12 3.5 2.5 20h19z"/><path d="M12 10v4.5M12 17.4v.2"/>"#,
        "check" => r#"<rect x="4" y="4" width="16" height="16" rx="3.5"/><path d="m8.5 12.2 2.4 2.4 4.6-5"/>"#,
        "factory" => r#"<path d="M3 21V10l6 3.5V10l6 3.5V5h6v16z"/><path d="M7 17h2M12 17h2M17 17h1"/>"#,
        "chart" => r#"<path d="M4 4v16h16"/><path d="m8 15 3.2-4.2 3 2.2L19 7.5"/>"#,
        "users" => r#"<circle cx="9" cy="8.2" r="3.2"/><path d="M3 20c0-3.4 2.6-5.6 6-5.6s6 2.2 6 5.6"/><path d="M16 5.3a3 3 0 0 1 0 5.8M18 14.8c1.8.7 3 2.4 3 5.2"/>"#,
        "scroll" => r#"<path d="M6 3h12v18H6z"/><path d="M9.5 8h5M9.5 12h5M9.5 16h3"/>"#,
        "key" => r#"<circle cx="8" cy="15.5" r="4"/><path d="m11 12.5 8.5-8.5M16 7l3 3"/>"#,
        "logout" => r#"<path d="M10 4H5v16h5M15.5 8 20 12l-4.5 4M20 12h-10"/>"#,
        "trash" => r#"<path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/>"#,
        "printer" => r#"<path d="M7 9V3h10v6"/><rect x="3" y="9" width="18" height="8" rx="2"/><path d="M7 14h10v7H7z"/>"#,
        "cash" => r#"<rect x="3" y="6" width="18" height="12" rx="2"/><circle cx="12" cy="12" r="2.6"/><path d="M6.5 9.5v.1M17.5 14.5v.1"/>"#,
        "bank" => r#"<path d="m3 9 9-5 9 5M5 10.5V18M9.7 10.5V18M14.3 10.5V18M19 10.5V18M3 20.5h18"/>"#,
        "plus" => r#"<path d="M12 5v14M5 12h14"/>"#,
        "x" => r#"<path d="M6 6l12 12M18 6 6 18"/>"#,
        "arrow-left" => r#"<path d="M19 12H5M11 6l-6 6 6 6"/>"#,
        "coins" => r#"<circle cx="12" cy="12" r="9"/><path d="M14.6 9.3c-.5-.8-1.5-1.3-2.6-1.3-1.5 0-2.6.8-2.6 2 0 3 5.2 1.4 5.2 4.2 0 1.2-1.1 2-2.6 2-1.3 0-2.3-.5-2.9-1.4M12 6.3v1.7M12 16v1.7"/>"#,
        "hourglass" => r#"<path d="M6.5 3h11M6.5 21h11M7.5 3c0 5 4.5 6 4.5 9s-4.5 4-4.5 9M16.5 3c0 5-4.5 6-4.5 9s4.5 4 4.5 9"/>"#,
        "trend-down" => r#"<path d="m3 7 6 6 4-4 8 8"/><path d="M21 11.5V17h-5.5"/>"#,
        "store" => r#"<path d="M4 9.5 5.6 4h12.8L20 9.5"/><path d="M4 9.5c0 1.5 1.1 2.7 2.5 2.7S9 11 9 9.5c0 1.5 1.1 2.7 2.5 2.7h1c1.4 0 2.5-1.2 2.5-2.7 0 1.5 1.1 2.7 2.5 2.7s2.5-1.2 2.5-2.7"/><path d="M5.5 12.5V20h13v-7.5M10 20v-4.5h4V20"/>"#,
        "scan" => r#"<path d="M4 8V5a1 1 0 0 1 1-1h3M16 4h3a1 1 0 0 1 1 1v3M20 16v3a1 1 0 0 1-1 1h-3M8 20H5a1 1 0 0 1-1-1v-3M8 9v6M12 9v6M16 9v6"/>"#,
        "shield" => r#"<path d="M12 3 4.5 6v5.5c0 4.4 3 8 7.5 9.5 4.5-1.5 7.5-5.100 7.500-9.500V6z"/><path d="m9 12 2.200 2.200 4-4.400"/>"#,
        "zap" => r#"<path d="M13 3 5 13.500h6L10 21l8-10.500h-6z"/>"#,
        "tick" => r#"<path d="M5 12.5l4.5 4.5L19 7.5"/>"#,
        "info" => r#"<circle cx="12" cy="12" r="9"/><path d="M12 11v5.5M12 7.5v.01"/>"#,
        "error" => r#"<circle cx="12" cy="12" r="9"/><path d="M12 7.5v5.5M12 16.5v.01"/>"#,
        _ => "",
    }
}

#[component]
pub fn Icon(name: &'static str, #[prop(optional)] size: Option<u32>) -> impl IntoView {
    let s = size.unwrap_or(18).to_string();
    view! {
        <svg class="icon" width=s.clone() height=s viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" inner_html=body(name)></svg>
    }
}
