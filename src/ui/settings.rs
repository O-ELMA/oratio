use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::*, Icon};
use oratio::i18n;

use crate::app::{mutate, navigate, persist, toast, Screen, DATA};
use crate::ui::common::{format_tokens, PageHeader, SectionCard, Toggle};

#[component]
fn SettingRow(
    icon: Element,
    title: String,
    #[props(default)] subtitle: String,
    onclick: EventHandler<()>,
    children: Element,
) -> Element {
    rsx! {
        button { class: "setting-row", onclick: move |_| onclick.call(()),
            {icon}
            div { class: "setting-row-text",
                span { "{title}" }
                if !subtitle.is_empty() {
                    small { "{subtitle}" }
                }
            }
            {children}
        }
    }
}

#[component]
pub fn SettingsPage() -> Element {
    let data = DATA();
    let providers = data.providers.len();
    let default_model = if data.settings.default_model.is_empty() {
        i18n::t("common.notSelected")
    } else {
        data.settings.default_model.clone()
    };
    let color_mode = data.settings.color_mode.clone();
    let language = data.settings.language.clone();

    rsx! {
        div { class: "screen",
            PageHeader { title: i18n::t("settings.title") }
            div { class: "content",
                if providers == 0 {
                    div { class: "warning-card",
                        div { class: "warning-icon",
                            Icon { icon: LdTriangleAlert, width: 26, height: 26, fill: "currentColor" }
                        }
                        div { class: "warning-text",
                            h2 { {i18n::t("settings.setupTitle")} }
                            p { {i18n::t("settings.setupMessage")} }
                        }
                        button {
                            class: "btn danger",
                            onclick: move |_| navigate(Screen::Providers),
                            {i18n::t("settings.configure")}
                        }
                    }
                }

                SectionCard { title: i18n::t("settings.general"), subtitle: "",
                    div { class: "setting-row static",
                        div { class: "row-icon primary",
                            Icon { icon: LdPalette, width: 18, height: 18, fill: "currentColor" }
                        }
                        div { class: "setting-row-text",
                            span { {i18n::t("settings.colorMode")} }
                            small { {i18n::t("settings.colorModeNote")} }
                        }
                    }
                    div { class: "connected-row",
                        for (value, label, icon) in [
                            ("system", i18n::t("settings.system"), 0),
                            ("light", i18n::t("settings.light"), 1),
                            ("dark", i18n::t("settings.dark"), 2),
                        ] {
                            {
                                let selected = color_mode == value;
                                rsx! {
                                    button {
                                        class: if selected { "connected choice selected" } else { "connected choice" },
                                        onclick: move |_| {
                                            mutate(|data| data.settings.color_mode = value.to_string());
                                            persist();
                                        },
                                        if icon == 0 {
                                            Icon { icon: LdSmartphone, width: 17, height: 17, fill: "currentColor" }
                                        } else if icon == 1 {
                                            Icon { icon: LdSun, width: 17, height: 17, fill: "currentColor" }
                                        } else {
                                            Icon { icon: LdMoon, width: 17, height: 17, fill: "currentColor" }
                                        }
                                        "{label}"
                                    }
                                }
                            }
                        }
                    }
                    SettingRow {
                        icon: rsx! { div { class: "row-icon primary",
                            Icon { icon: LdSlidersHorizontal, width: 18, height: 18, fill: "currentColor" }
                        } },
                        title: i18n::t("settings.preferences"),
                        subtitle: i18n::t("settings.preferencesSub"),
                        onclick: move |_| navigate(Screen::Preferences),
                        Icon { icon: LdChevronRight, width: 18, height: 18, fill: "currentColor" }
                    }
                    SettingRow {
                        icon: rsx! { div { class: "row-icon tertiary",
                            Icon { icon: LdSparkles, width: 18, height: 18, fill: "currentColor" }
                        } },
                        title: i18n::t("settings.theme"),
                        subtitle: i18n::t("settings.themeSub"),
                        onclick: move |_| navigate(Screen::Theme),
                        Icon { icon: LdChevronRight, width: 18, height: 18, fill: "currentColor" }
                    }
                }

                SectionCard { title: i18n::t("settings.language"), subtitle: "",
                    div { class: "connected-row",
                        for (code, label) in [("en", "English"), ("fr", "Français"), ("ar", "العربية")] {
                            button {
                                class: if language == code { "connected choice selected" } else { "connected choice" },
                                onclick: move |_| {
                                    mutate(|data| data.settings.language = code.to_string());
                                    i18n::set_language(code);
                                    persist();
                                },
                                "{label}"
                            }
                        }
                    }
                }

                SectionCard { title: i18n::t("settings.modelServices"), subtitle: "",
                    SettingRow {
                        icon: rsx! { div { class: "row-icon primary",
                            Icon { icon: LdSparkles, width: 18, height: 18, fill: "currentColor" }
                        } },
                        title: i18n::t("settings.defaultModel"),
                        subtitle: "{default_model}",
                        onclick: move |_| navigate(Screen::Models),
                        Icon { icon: LdChevronRight, width: 18, height: 18, fill: "currentColor" }
                    }
                    SettingRow {
                        icon: rsx! { div { class: "row-icon tertiary",
                            Icon { icon: LdBrain, width: 18, height: 18, fill: "currentColor" }
                        } },
                        title: i18n::t("settings.providers"),
                        subtitle: i18n::tn("common.endpoints", providers),
                        onclick: move |_| navigate(Screen::Providers),
                        Icon { icon: LdChevronRight, width: 18, height: 18, fill: "currentColor" }
                    }
                }

                SectionCard { title: i18n::t("settings.about"), subtitle: "",
                    SettingRow {
                        icon: rsx! { div { class: "row-icon primary",
                            Icon { icon: LdInfo, width: 18, height: 18, fill: "currentColor" }
                        } },
                        title: "Oratio",
                        subtitle: i18n::t("settings.version"),
                        onclick: move |_| navigate(Screen::About),
                        Icon { icon: LdChevronRight, width: 18, height: 18, fill: "currentColor" }
                    }
                }
                div { class: "version", {i18n::t("settings.builtWith")} }
            }
        }
    }
}

#[component]
pub fn ThemePage() -> Element {
    let data = DATA();
    let selected = data.settings.theme_id.clone();
    let themes = [
        (
            "sakura", "Sakura", "#8E4955", "#FFD9DD", "#FFF8F7", "#1A1112",
        ),
        ("ocean", "Ocean", "#116682", "#BDE9FF", "#F6FAFD", "#0F1417"),
        (
            "spring", "Spring", "#4C662B", "#CDEDA3", "#F9FAEF", "#12140E",
        ),
        (
            "autumn", "Autumn", "#735C0C", "#FFE08B", "#FFF8F1", "#16130B",
        ),
        ("black", "Black", "#606060", "#E6E6E6", "#FFFFFF", "#1C1C1C"),
        (
            "minimal", "Minimal", "#2563EB", "#E7F0FE", "#FFFFFF", "#0F1012",
        ),
        (
            "purple", "Purple", "#6A4C93", "#EADDFF", "#FDF7FF", "#141218",
        ),
        (
            "claude", "Claude", "#C96442", "#F7E3D9", "#FAF9F5", "#1F1E1D",
        ),
    ];
    rsx! {
        div { class: "screen",
            PageHeader { title: i18n::t("settings.theme") }
            div { class: "content",
                div { class: "theme-grid",
                    for (id, name, primary, container, light, dark) in themes {
                        {
                            let is_selected = selected == id;
                            rsx! {
                                button {
                                    key: "{id}",
                                    class: if is_selected { "theme-card selected" } else { "theme-card" },
                                    onclick: move |_| {
                                        mutate(|data| data.settings.theme_id = id.to_string());
                                        persist();
                                    },
                                    div { class: "theme-preview",
                                        div { class: "theme-preview-bar", style: "background:{primary};" }
                                        div { class: "theme-preview-body",
                                            div { class: "theme-preview-dot", style: "background:{container};" }
                                            div { class: "theme-preview-lines",
                                                div { style: "background:{light};" }
                                                div { style: "background:{dark};opacity:.35;" }
                                            }
                                        }
                                    }
                                    div { class: "theme-name",
                                        span { "{name}" }
                                        if is_selected {
                                            Icon { icon: LdCheck, width: 16, height: 16, fill: "currentColor" }
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

#[component]
pub fn PreferencesPage() -> Element {
    let data = DATA();
    let settings = data.settings.clone();
    let mut nickname = use_signal(|| settings.user_nickname.clone());
    let mut opacity = use_signal(|| (settings.bubble_opacity * 100.0).round() as i32);
    let mut font_scale = use_signal(|| (settings.font_scale * 100.0).round() as i32);
    let current_color = settings.user_avatar_color.clone();

    rsx! {
        div { class: "screen",
            PageHeader { title: i18n::t("settings.preferencesTitle") }
            div { class: "content",
                SectionCard { title: i18n::t("settings.profile"), subtitle: "",
                    div { class: "field",
                        label { {i18n::t("settings.nickname")} }
                        input {
                            class: "input",
                            placeholder: i18n::t("settings.nicknamePlaceholder"),
                            initial_value: "{nickname}",
                            oninput: move |event| nickname.set(event.value()),
                            onchange: move |_| {
                                let value = nickname().trim().to_string();
                                mutate(|data| data.settings.user_nickname = value);
                                persist();
                            },
                        }
                    }
                    div { class: "field",
                        label { {i18n::t("settings.avatarColor")} }
                        div { class: "color-row",
                            for color in ["#8E4955", "#116682", "#4C662B", "#735C0C", "#6A4C93", "#A64B2A", "#3D6B5C", "#7A3E65"] {
                                button {
                                    class: if current_color == color { "color-dot selected" } else { "color-dot" },
                                    style: "background:{color};",
                                    onclick: move |_| {
                                        mutate(|data| data.settings.user_avatar_color = color.to_string());
                                        persist();
                                    },
                                }
                            }
                        }
                    }
                }

                SectionCard { title: i18n::t("settings.messages"), subtitle: "",
                    div { class: "setting-row static",
                        div { class: "row-icon primary",
                            Icon { icon: LdMessageSquare, width: 18, height: 18, fill: "currentColor" }
                        }
                        div { class: "setting-row-text",
                            span { {i18n::t("settings.assistantBubbles")} }
                            small { {i18n::t("settings.assistantBubblesNote")} }
                        }
                        Toggle {
                            checked: settings.show_assistant_bubble,
                            onchange: move |value| {
                                mutate(|data| data.settings.show_assistant_bubble = value);
                                persist();
                            },
                        }
                    }
                    div { class: "setting-row static",
                        div { class: "row-icon secondary",
                            Icon { icon: LdBrain, width: 18, height: 18, fill: "currentColor" }
                        }
                        div { class: "setting-row-text",
                            span { {i18n::t("settings.showModelName")} }
                            small { {i18n::t("settings.showModelNameNote")} }
                        }
                        Toggle {
                            checked: settings.show_model_name,
                            onchange: move |value| {
                                mutate(|data| data.settings.show_model_name = value);
                                persist();
                            },
                        }
                    }
                    div { class: "setting-row static",
                        div { class: "row-icon tertiary",
                            Icon { icon: LdBarChart3, width: 18, height: 18, fill: "currentColor" }
                        }
                        div { class: "setting-row-text",
                            span { {i18n::t("settings.tokenUsage")} }
                            small { {i18n::t("settings.tokenUsageNote")} }
                        }
                        Toggle {
                            checked: settings.show_token_usage,
                            onchange: move |value| {
                                mutate(|data| data.settings.show_token_usage = value);
                                persist();
                            },
                        }
                    }
                    div { class: "setting-row static",
                        div { class: "row-icon",
                            Icon { icon: LdCornerDownLeft, width: 18, height: 18, fill: "currentColor" }
                        }
                        div { class: "setting-row-text",
                            span { {i18n::t("settings.sendOnEnter")} }
                            small { {i18n::t("settings.sendOnEnterNote")} }
                        }
                        Toggle {
                            checked: settings.send_on_enter,
                            onchange: move |value| {
                                mutate(|data| data.settings.send_on_enter = value);
                                persist();
                            },
                        }
                    }
                }

                SectionCard { title: i18n::t("settings.sizing"), subtitle: "",
                    div { class: "slider-row",
                        label { {i18n::t("settings.bubbleOpacity")} }
                        span { "{opacity()}%" }
                        input {
                            r#type: "range",
                            min: "20",
                            max: "100",
                            step: "5",
                            initial_value: "{opacity}",
                            oninput: move |event| {
                                let value = event.value().parse::<i32>().unwrap_or(100);
                                opacity.set(value);
                                mutate(|data| data.settings.bubble_opacity = value as f32 / 100.0);
                                persist();
                            },
                        }
                    }
                    div { class: "slider-row",
                        label { {i18n::t("settings.textSize")} }
                        span { "{font_scale()}%" }
                        input {
                            r#type: "range",
                            min: "80",
                            max: "130",
                            step: "5",
                            initial_value: "{font_scale}",
                            oninput: move |event| {
                                let value = event.value().parse::<i32>().unwrap_or(100);
                                font_scale.set(value);
                                mutate(|data| data.settings.font_scale = value as f32 / 100.0);
                                persist();
                            },
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ModelsPage() -> Element {
    let data = DATA();
    let providers = data.providers.clone();
    let default_provider = data.settings.default_provider_id.clone();
    let default_model = data.settings.default_model.clone();
    let mut query = use_signal(String::new);
    let needle = query().to_lowercase();
    rsx! {
        div { class: "screen",
            PageHeader { title: i18n::t("settings.modelsTitle"),
                if !default_model.is_empty() {
                    button {
                        class: "icon-btn",
                        title: i18n::t("settings.clearDefault"),
                        onclick: move |_| {
                            mutate(|data| {
                                data.settings.default_provider_id.clear();
                                data.settings.default_model.clear();
                            });
                            persist();
                            toast(i18n::t("settings.defaultCleared"));
                        },
                        Icon { icon: LdX, width: 20, height: 20, fill: "currentColor" }
                    }
                }
            }
            div { class: "content",
                input {
                    class: "input search-input",
                    placeholder: i18n::t("settings.searchModels"),
                    initial_value: "{query}",
                    oninput: move |event| query.set(event.value()),
                }
                if providers.is_empty() {
                    div { class: "muted-note", {i18n::t("settings.addProviderFirst")} }
                } else {
                    for provider in providers {
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
                                                let selected = provider.id == default_provider && model == default_model;
                                                let pid = provider.id.clone();
                                                let model_name = model.clone();
                                                rsx! {
                                                    button {
                                                        key: "{model}",
                                                        class: if selected { "model-row selected" } else { "model-row" },
                                                        onclick: move |_| {
                                                            mutate(|data| {
                                                                data.settings.default_provider_id = pid.clone();
                                                                data.settings.default_model = model_name.clone();
                                                            });
                                                            persist();
                                                            toast(i18n::t("settings.defaultSet"));
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
                div { class: "muted-note",
                    {i18n::t("settings.modelsNote")}
                }
            }
        }
    }
}

#[component]
pub fn AboutPage() -> Element {
    let data = DATA();
    let providers = data.providers.len();
    let chats = data.chats.len();
    let messages: usize = data.chats.iter().map(|chat| chat.messages.len()).sum();
    let tokens: u32 = data
        .usage
        .iter()
        .map(|entry| entry.input_tokens + entry.output_tokens)
        .sum();
    rsx! {
        div { class: "screen",
            PageHeader { title: i18n::t("settings.aboutTitle"), subtitle: i18n::t("settings.aboutSubtitle") }
            div { class: "content",
                div { class: "about-hero",
                    div { class: "about-logo",
                        Icon { icon: LdSparkles, width: 34, height: 34, fill: "currentColor" }
                    }
                    h2 { "Oratio" }
                    p { {i18n::t("settings.versionAndroid")} }
                    span { class: "badge primary", "Rust + Dioxus" }
                }
                div { class: "stats-grid",
                    div { class: "stat-card",
                        span { class: "stat-value", "{providers}" }
                        span { class: "stat-label", {i18n::t("settings.statProviders")} }
                    }
                    div { class: "stat-card",
                        span { class: "stat-value", "{chats}" }
                        span { class: "stat-label", {i18n::t("settings.statChats")} }
                    }
                    div { class: "stat-card",
                        span { class: "stat-value", "{messages}" }
                        span { class: "stat-label", {i18n::t("settings.statMessages")} }
                    }
                    div { class: "stat-card",
                        span { class: "stat-value", "{format_tokens(tokens)}" }
                        span { class: "stat-label", {i18n::t("settings.statTokens")} }
                    }
                }
                SectionCard { title: i18n::t("settings.inside"), subtitle: "",
                    div { class: "about-line", {i18n::t("settings.inside1")} }
                    div { class: "about-line", {i18n::t("settings.inside2")} }
                    div { class: "about-line", {i18n::t("settings.inside3")} }
                }
                SectionCard { title: i18n::t("settings.credits"), subtitle: "",
                    div { class: "about-line", {i18n::t("settings.credit1")} }
                    div { class: "about-line", {i18n::t("settings.credit2")} }
                    div { class: "about-line", {i18n::t("settings.credit3")} }
                    div { class: "about-line", {i18n::t("settings.credit4")} }
                }
                div { class: "version", {i18n::t("settings.privacy")} }
            }
        }
    }
}
