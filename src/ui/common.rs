use chrono::{Datelike, Local, TimeZone, Timelike};
use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::*, Icon};
use oratio::i18n;

use crate::app::{back, toast, TOAST};

#[component]
pub fn Avatar(name: String, color: String, size: u32) -> Element {
    let initial = name
        .trim()
        .chars()
        .next()
        .unwrap_or('R')
        .to_uppercase()
        .to_string();
    rsx! {
        div {
            class: "avatar",
            style: "width:{size}px;height:{size}px;font-size:{size / 2}px;background:{color};",
            span { "{initial}" }
        }
    }
}

#[component]
pub fn UserAvatar(nickname: String, color: String, size: u32) -> Element {
    rsx! { Avatar { name: nickname, color, size } }
}

#[component]
pub fn PageHeader(title: String, #[props(default)] subtitle: String, children: Element) -> Element {
    rsx! {
        header { class: "page-header",
            button { class: "icon-btn large", onclick: move |_| back(),
                Icon { icon: LdArrowLeft, width: 22, height: 22, fill: "currentColor" }
            }
            div { class: "page-header-text",
                h1 { "{title}" }
                if !subtitle.is_empty() {
                    p { "{subtitle}" }
                }
            }
            div { class: "page-header-actions", {children} }
        }
    }
}

#[component]
pub fn Toast() -> Element {
    let message = TOAST();
    rsx! {
        if let Some(message) = message {
            div { class: "toast", "{message}" }
        }
    }
}

#[component]
pub fn Sheet(title: String, on_close: EventHandler<()>, children: Element) -> Element {
    rsx! {
        div { class: "overlay", onclick: move |_| on_close.call(()),
            div { class: "sheet", onclick: move |event| event.stop_propagation(),
                div { class: "sheet-handle" }
                div { class: "sheet-head",
                    if !title.is_empty() {
                        h2 { "{title}" }
                    }
                    button { class: "icon-btn small", onclick: move |_| on_close.call(()),
                        Icon { icon: LdX, width: 18, height: 18, fill: "currentColor" }
                    }
                }
                {children}
            }
        }
    }
}

#[component]
pub fn ConfirmButton(
    #[props(default)] text: String,
    #[props(default)] title: String,
    #[props(default)] confirm_label: String,
    #[props(default)] class: String,
    onconfirm: EventHandler<()>,
    children: Element,
) -> Element {
    let mut armed = use_signal(|| false);
    let mut nonce = use_signal(|| 0u32);
    let confirm_label = if confirm_label.is_empty() {
        i18n::t("common.really")
    } else {
        confirm_label
    };
    let unarmed_title = if !title.is_empty() {
        title.clone()
    } else if !text.is_empty() {
        text.clone()
    } else {
        i18n::t("common.delete")
    };
    let current_title = if armed() {
        confirm_label.clone()
    } else {
        unarmed_title
    };
    rsx! {
        button {
            class: if armed() { format!("confirm-button armed {class}") } else { format!("confirm-button {class}") },
            title: "{current_title}",
            "aria-label": "{current_title}",
            onclick: move |event| {
                event.stop_propagation();
                if armed() {
                    armed.set(false);
                    onconfirm.call(());
                    return;
                }
                armed.set(true);
                let token = nonce() + 1;
                nonce.set(token);
                dioxus::prelude::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
                    if armed() && nonce() == token {
                        armed.set(false);
                    }
                });
            },
            onblur: move |_| armed.set(false),
            onkeydown: move |event| {
                if event.key() == Key::Escape {
                    armed.set(false);
                }
            },
            {children}
            if armed() {
                span { class: "confirm-label", "{confirm_label}" }
            } else if !text.is_empty() {
                span { class: "confirm-label", "{text}" }
            }
        }
    }
}

#[component]
pub fn ConfirmDialog(
    title: String,
    message: String,
    confirm_label: String,
    #[props(default = true)] danger: bool,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "overlay center", onclick: move |_| on_cancel.call(()),
            div { class: "modal", onclick: move |event| event.stop_propagation(),
                h2 { "{title}" }
                p { class: "muted", "{message}" }
                div { class: "modal-actions",
                    button { class: "btn ghost", onclick: move |_| on_cancel.call(()), {i18n::t("common.cancel")} }
                    button {
                        class: if danger { "btn danger" } else { "btn" },
                        onclick: move |_| on_confirm.call(()),
                        "{confirm_label}"
                    }
                }
            }
        }
    }
}

#[component]
pub fn SectionCard(
    title: String,
    #[props(default)] subtitle: String,
    children: Element,
) -> Element {
    rsx! {
        section { class: "card-group",
            div { class: "card-group-head",
                h2 { "{title}" }
                if !subtitle.is_empty() {
                    p { "{subtitle}" }
                }
            }
            {children}
        }
    }
}

#[component]
pub fn EmptyState(title: String, message: String, children: Element) -> Element {
    rsx! {
        div { class: "empty",
            div { class: "empty-mark",
                Icon { icon: LdSparkles, width: 28, height: 28, fill: "currentColor" }
            }
            h2 { "{title}" }
            p { "{message}" }
            {children}
        }
    }
}

#[component]
pub fn Toggle(checked: bool, onchange: EventHandler<bool>) -> Element {
    rsx! {
        button {
            class: if checked { "toggle on" } else { "toggle" },
            onclick: move |_| onchange.call(!checked),
            div { class: "toggle-knob" }
        }
    }
}

pub fn copy_text(text: &str) {
    if let Ok(literal) = serde_json::to_string(text) {
        let _ = document::eval(&format!(
            "(function(){{const t=document.createElement('textarea');t.value={literal};document.body.appendChild(t);t.select();document.execCommand('copy');t.remove();}})();"
        ));
    }
    toast(i18n::t("common.copied"));
}

pub fn time_ago(timestamp: u64) -> String {
    if timestamp == 0 {
        return String::new();
    }
    let now = chrono::Local::now().timestamp_millis() as u64;
    let seconds = now.saturating_sub(timestamp) / 1000;
    match seconds {
        0..=59 => i18n::t("common.time.now"),
        60..=3599 => i18n::ta(
            "common.time.minutes",
            &[("count", &(seconds / 60).to_string())],
        ),
        3600..=86399 => i18n::ta(
            "common.time.hours",
            &[("count", &(seconds / 3600).to_string())],
        ),
        86400..=604799 => i18n::ta(
            "common.time.days",
            &[("count", &(seconds / 86400).to_string())],
        ),
        _ => i18n::ta(
            "common.time.weeks",
            &[("count", &(seconds / 604800).to_string())],
        ),
    }
}

pub fn clock(timestamp: u64) -> String {
    Local
        .timestamp_millis_opt(timestamp as i64)
        .single()
        .map(|time| format!("{:02}:{:02}", time.hour(), time.minute()))
        .unwrap_or_default()
}

pub fn greeting() -> String {
    let hour = Local::now().hour();
    match hour {
        5..=11 => i18n::t("common.greeting.morning"),
        12..=17 => i18n::t("common.greeting.afternoon"),
        18..=22 => i18n::t("common.greeting.evening"),
        _ => i18n::t("common.greeting.late"),
    }
}

pub fn format_tokens(tokens: u32) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f32 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f32 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

pub fn assistant_color(name: &str) -> &'static str {
    const COLORS: [&str; 8] = [
        "#8E4955", "#116682", "#4C662B", "#735C0C", "#6A4C93", "#A64B2A", "#3D6B5C", "#7A3E65",
    ];
    let index = name
        .bytes()
        .fold(0usize, |acc, byte| acc.wrapping_add(byte as usize))
        % COLORS.len();
    COLORS[index]
}

pub fn date_label(timestamp: u64) -> String {
    let Some(time) = Local.timestamp_millis_opt(timestamp as i64).single() else {
        return String::new();
    };
    let today = Local::now().date_naive();
    let date = time.date_naive();
    if date == today {
        i18n::t("common.date.today")
    } else if date == today.pred_opt().unwrap_or(today) {
        i18n::t("common.date.yesterday")
    } else if (today - date).num_days() < 7 {
        i18n::weekday_long(time.weekday())
    } else if date.year() == today.year() {
        i18n::ta(
            "common.date.monthDay",
            &[
                ("month", &i18n::month_short(time.month())),
                ("day", &format!("{:02}", time.day())),
            ],
        )
    } else {
        time.format("%Y-%m-%d").to_string()
    }
}

#[component]
pub fn ModelPickerSheet(
    current_provider: String,
    current_model: String,
    on_select: EventHandler<(String, String)>,
    on_close: EventHandler<()>,
) -> Element {
    let data = crate::app::DATA();
    let mut query = use_signal(String::new);
    let needle = query().to_lowercase();
    let providers = data.providers.clone();
    rsx! {
        Sheet { title: i18n::t("common.selectModel"), on_close,
            input {
                class: "input search-input",
                placeholder: i18n::t("settings.searchModels"),
                initial_value: "{query}",
                oninput: move |event| query.set(event.value()),
            }
            if providers.is_empty() {
                div { class: "muted-note",
                    {i18n::t("settings.addProviderFirst")}
                }
                div { class: "sheet-actions",
                    button {
                        class: "btn",
                        onclick: move |_| {
                            on_close.call(());
                            crate::app::navigate(crate::app::Screen::Providers);
                        },
                        {i18n::t("common.create")}
                    }
                }
            } else {
                for provider in providers.iter() {
                    {
                        let provider = provider.clone();
                        let models: Vec<String> = provider
                            .models
                            .iter()
                            .filter(|model| needle.is_empty() || model.to_lowercase().contains(&needle))
                            .cloned()
                            .collect();
                        if models.is_empty() {
                            rsx! {}
                        } else {
                            rsx! {
                                div { key: "{provider.id}", class: "model-group",
                                    div { class: "model-group-title",
                                        span { "{provider.name}" }
                                        span { class: "model-group-count", "{models.len()}" }
                                    }
                                    for model in models {
                                        {
                                            let selected = provider.id == current_provider && model == current_model;
                                            let provider_id = provider.id.clone();
                                            let model_name = model.clone();
                                            rsx! {
                                                button {
                                                    key: "{model}",
                                                    class: if selected { "model-row selected" } else { "model-row" },
                                                    onclick: move |_| {
                                                        on_select.call((provider_id.clone(), model_name.clone()));
                                                        on_close.call(());
                                                    },
                                                    div { class: "model-row-icon",
                                                        Icon { icon: LdSparkles, width: 16, height: 16, fill: "currentColor" }
                                                    }
                                                    span { class: "model-row-name", "{model}" }
                                                    if selected {
                                                        Icon { icon: LdCheck, width: 18, height: 18, fill: "currentColor" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn pick_image(camera: bool) {
    let capture = if camera {
        "input.capture = 'environment';".to_string()
    } else {
        String::new()
    };
    let choose_error =
        serde_json::to_string(&i18n::t("common.chooseImageFile")).unwrap_or_default();
    let script = format!(
        r#"
        (function () {{
            const input = document.createElement('input');
            input.type = 'file';
            input.accept = 'image/*';
            {capture}
            input.style.display = 'none';
            document.body.appendChild(input);
            input.onchange = () => {{
                const file = input.files[0];
                input.remove();
                if (!file) return;
                if (!file.type.startsWith('image/')) {{
                    dioxus.send(JSON.stringify({{ error: {choose_error} }}));
                    return;
                }}
                const reader = new FileReader();
                reader.onload = () => {{
                    const img = new Image();
                    img.onload = () => {{
                        let width = img.width;
                        let height = img.height;
                        const max = 1280;
                        if (width > max || height > max) {{
                            const scale = max / Math.max(width, height);
                            width = Math.round(width * scale);
                            height = Math.round(height * scale);
                        }}
                        const canvas = document.createElement('canvas');
                        canvas.width = width;
                        canvas.height = height;
                        canvas.getContext('2d').drawImage(img, 0, 0, width, height);
                        dioxus.send(JSON.stringify({{
                            name: file.name || 'Image.jpg',
                            mime: 'image/jpeg',
                            data_url: canvas.toDataURL('image/jpeg', 0.84)
                        }}));
                    }};
                    img.src = reader.result;
                }};
                reader.readAsDataURL(file);
            }};
            input.click();
        }})();
        "#
    );
    receive_pick(script);
}

pub fn pick_text_file() {
    let script = r#"
    (function () {
        const input = document.createElement('input');
        input.type = 'file';
        input.accept = '.txt,.md,.json,.csv,.log,.toml,.yaml,.yml,.rs,.py,.js,.ts,.tsx,.jsx,.html,.css,.xml,.sh';
        input.style.display = 'none';
        document.body.appendChild(input);
        input.onchange = () => {
            const file = input.files[0];
            input.remove();
            if (!file) return;
            const reader = new FileReader();
            reader.onload = () => {
                const text = String(reader.result || '');
                dioxus.send(JSON.stringify({
                    name: file.name || 'file.txt',
                    mime: 'text/plain',
                    text: text.slice(0, 120000)
                }));
            };
            reader.readAsText(file);
        };
        input.click();
    })();
    "#;
    receive_pick(script.to_string());
}

pub fn open_sketch() {
    let script = r#"
    (function () {
        if (document.getElementById('__oratio_sketch')) return;
        const overlay = document.createElement('div');
        overlay.id = '__oratio_sketch';
        overlay.style.cssText = 'position:fixed;inset:0;z-index:9999;background:rgba(0,0,0,.5);display:flex;align-items:center;justify-content:center;padding:12px;';
        const panel = document.createElement('div');
        panel.style.cssText = 'width:100%;max-width:620px;background:#fff;border-radius:24px;padding:10px;display:flex;flex-direction:column;gap:8px;box-shadow:0 20px 60px rgba(0,0,0,.4);';
        const canvas = document.createElement('canvas');
        canvas.width = 1000;
        canvas.height = 1000;
        canvas.style.cssText = 'width:100%;aspect-ratio:1;background:#fff;border-radius:16px;touch-action:none;display:block;';
        const bar = document.createElement('div');
        bar.style.cssText = 'display:flex;gap:8px;justify-content:flex-end;';
        const cancel = document.createElement('button');
        cancel.textContent = '__CANCEL__';
        cancel.style.cssText = 'border:none;background:#eee;color:#333;padding:10px 16px;border-radius:999px;font-size:14px;';
        const save = document.createElement('button');
        save.textContent = '__SAVE__';
        save.style.cssText = 'border:none;background:#8e4955;color:#fff;padding:10px 16px;border-radius:999px;font-size:14px;';
        bar.append(cancel, save);
        panel.append(canvas, bar);
        overlay.append(panel);
        document.body.append(overlay);
        const ctx = canvas.getContext('2d');
        ctx.lineWidth = 6;
        ctx.lineCap = 'round';
        ctx.lineJoin = 'round';
        ctx.strokeStyle = '#1b1b1b';
        let drawing = false;
        let last = null;
        function pos(event) {
            const rect = canvas.getBoundingClientRect();
            return {
                x: (event.clientX - rect.left) * canvas.width / rect.width,
                y: (event.clientY - rect.top) * canvas.height / rect.height
            };
        }
        canvas.addEventListener('pointerdown', (event) => {
            drawing = true;
            last = pos(event);
            canvas.setPointerCapture(event.pointerId);
        });
        canvas.addEventListener('pointermove', (event) => {
            if (!drawing || !last) return;
            const point = pos(event);
            ctx.beginPath();
            ctx.moveTo(last.x, last.y);
            ctx.lineTo(point.x, point.y);
            ctx.stroke();
            last = point;
        });
        canvas.addEventListener('pointerup', () => { drawing = false; last = null; });
        cancel.onclick = () => {
            overlay.remove();
            dioxus.send(JSON.stringify({ error: 'Sketch cancelled' }));
        };
        save.onclick = () => {
            const url = canvas.toDataURL('image/png');
            overlay.remove();
            dioxus.send(JSON.stringify({ name: 'Sketch.png', mime: 'image/png', data_url: url }));
        };
    })();
    "#;
    receive_pick(
        script
            .replace("__CANCEL__", &i18n::t("common.cancel"))
            .replace("__SAVE__", &i18n::t("common.saveSketch")),
    );
}

fn receive_pick(script: String) {
    dioxus::core::spawn_forever(async move {
        let mut eval = document::eval(&script);
        while let Ok(message) = eval.recv::<String>().await {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&message) else {
                continue;
            };
            if let Some(error) = value.get("error").and_then(|value| value.as_str()) {
                if error != "Sketch cancelled" {
                    toast(error.to_string());
                }
                continue;
            }
            let name = value
                .get("name")
                .and_then(|value| value.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| i18n::t("common.attachment"));
            let mime = value
                .get("mime")
                .and_then(|value| value.as_str())
                .unwrap_or("application/octet-stream")
                .to_string();
            let data_url = value
                .get("data_url")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string();
            let text = value
                .get("text")
                .and_then(|value| value.as_str())
                .map(str::to_string);
            if data_url.is_empty() && text.is_none() {
                toast(i18n::t("common.nothingAttached"));
                continue;
            }
            crate::app::add_attachment(oratio::store::Attachment {
                id: oratio::store::new_id(),
                name,
                mime,
                data_url,
                text,
            });
            toast(i18n::t("common.attached"));
        }
    });
}
