use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::*, Icon};
use oratio::{i18n, markdown, store::Conversation};

use crate::app::{delete_all_chats, navigate, open_chat, toast, toggle_pin, Screen, DATA};
use crate::ui::common::{
    clock, date_label, time_ago, ConfirmButton, EmptyState, PageHeader, SectionCard,
};

#[component]
pub fn HistoryPage() -> Element {
    let data = DATA();
    let mut query = use_signal(String::new);
    let needle = query().to_lowercase();

    let mut chats: Vec<Conversation> = data
        .chats
        .iter()
        .filter(|chat| {
            needle.is_empty()
                || chat.title.to_lowercase().contains(&needle)
                || chat
                    .messages
                    .iter()
                    .any(|message| message.content.to_lowercase().contains(&needle))
        })
        .cloned()
        .collect();
    chats.sort_by(|a, b| {
        b.pinned
            .cmp(&a.pinned)
            .then_with(|| b.updated_at.cmp(&a.updated_at))
    });

    rsx! {
        div { class: "screen",
            PageHeader { title: i18n::t("history.title"), subtitle: i18n::tn("common.conversations", data.chats.len()),
                ConfirmButton {
                    class: "icon-btn confirm-icon",
                    title: i18n::t("history.deleteAll"),
                    onconfirm: move |_| {
                        delete_all_chats();
                        toast(i18n::t("history.deletedAll"));
                    },
                    Icon { icon: LdTrash2, width: 20, height: 20, fill: "currentColor" }
                }
            }
            div { class: "content",
                input {
                    class: "input search-input",
                    placeholder: i18n::t("history.searchPlaceholder"),
                    initial_value: "{query}",
                    oninput: move |event| query.set(event.value()),
                }
                if chats.is_empty() {
                    EmptyState {
                        title: if needle.is_empty() { i18n::t("history.emptyTitle") } else { i18n::t("history.noMatch") },
                        message: if needle.is_empty() { i18n::t("history.emptyMessage") } else { i18n::t("history.tryAnother") },
                    }
                } else {
                    for chat in chats {
                        {
                            let id = chat.id.clone();
                            let title = chat.display_title();
                            let preview = chat.preview();
                            let updated = time_ago(chat.updated_at);
                            let count = chat.messages.len();
                            let pinned = chat.pinned;
                            let date = date_label(chat.updated_at);
                            rsx! {
                                HistoryRow {
                                    key: "{id}",
                                    id, title, preview, updated, count, pinned, date
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
fn HistoryRow(
    id: String,
    title: String,
    preview: String,
    updated: String,
    count: usize,
    pinned: bool,
    date: String,
) -> Element {
    let row_id = id.clone();
    let pin_id = id.clone();
    let delete_id = id.clone();
    rsx! {
        div {
            class: "history-row",
            onclick: move |_| open_chat(row_id.clone()),
            div { class: "history-main",
                div { class: "history-title",
                    if pinned {
                        Icon { icon: LdPin, width: 13, height: 13, fill: "currentColor" }
                    }
                    span { "{title}" }
                }
                if !preview.is_empty() {
                    div { class: "history-preview", "{preview}" }
                }
                div { class: "history-meta",
                    {format!("{date} · {updated} · {}", i18n::tn("common.messages", count))}
                }
            }
            div { class: "row-actions",
                button {
                    class: "icon-btn small",
                    title: if pinned { i18n::t("history.unpin") } else { i18n::t("history.pin") },
                    onclick: move |event| {
                        event.stop_propagation();
                        toggle_pin(&pin_id);
                    },
                    if pinned {
                        Icon { icon: LdPinOff, width: 16, height: 16, fill: "currentColor" }
                    } else {
                        Icon { icon: LdPin, width: 16, height: 16, fill: "currentColor" }
                    }
                }
                ConfirmButton {
                    class: "icon-btn small confirm-icon",
                    title: i18n::t("common.delete"),
                    onconfirm: move |_| crate::app::delete_chat(&delete_id),
                    Icon { icon: LdTrash2, width: 16, height: 16, fill: "currentColor" }
                }
            }
        }
    }
}

#[component]
pub fn SearchPage() -> Element {
    let data = DATA();
    let mut query = use_signal(String::new);
    let needle = query().to_lowercase();
    let mut results = Vec::new();
    if !needle.is_empty() {
        for chat in &data.chats {
            for message in &chat.messages {
                let title_match = chat.title.to_lowercase().contains(&needle);
                let content_match = message.content.to_lowercase().contains(&needle);
                if title_match || content_match {
                    let snippet = if content_match {
                        let chars: Vec<char> = message.content.chars().collect();
                        let lower = message.content.to_lowercase();
                        let byte_position = lower.find(&needle).unwrap_or(0);
                        let char_position = lower[..byte_position].chars().count();
                        let start = char_position.saturating_sub(40);
                        let end = (char_position + 100).min(chars.len());
                        let mut text: String = chars[start..end].iter().collect();
                        text = text
                            .replace('\n', " ")
                            .replace("##", " ")
                            .replace("**", "")
                            .replace('`', "");
                        if start > 0 {
                            text = format!("…{text}");
                        }
                        if end < chars.len() {
                            text.push('…');
                        }
                        text
                    } else {
                        chat.preview()
                    };
                    results.push((
                        chat.id.clone(),
                        chat.display_title(),
                        message.role.clone(),
                        snippet,
                        message.created_at,
                    ));
                }
            }
        }
    }
    results.sort_by(|a, b| b.4.cmp(&a.4));

    rsx! {
        div { class: "screen",
            PageHeader { title: i18n::t("search.title"), subtitle: i18n::t("search.subtitle") }
            div { class: "content",
                input {
                    class: "input search-input",
                    placeholder: i18n::t("search.placeholder"),
                    initial_value: "{query}",
                    oninput: move |event| query.set(event.value()),
                }
                if needle.is_empty() {
                    EmptyState {
                        title: i18n::t("search.emptyTitle"),
                        message: i18n::t("search.emptyMessage"),
                    }
                } else if results.is_empty() {
                    EmptyState {
                        title: i18n::t("search.noMatchTitle"),
                        message: i18n::t("search.noMatchMessage"),
                    }
                } else {
                    div { class: "muted-note", {i18n::tn("common.matches", results.len())} }
                    for (index, (chat_id, title, role, snippet, at)) in results.into_iter().enumerate() {
                        {
                            let id = chat_id.clone();
                            rsx! {
                                button {
                                    key: "{index}",
                                    class: "search-result",
                                    onclick: move |_| open_chat(id.clone()),
                                    div { class: "search-result-text",
                                        div { class: "search-result-title", "{title}" }
                                        div { class: "search-result-snippet", "{snippet}" }
                                        div { class: "search-result-meta", {format!("{} · {}", role_label(&role), clock(at))} }
                                    }
                                    Icon { icon: LdArrowUpRight, width: 16, height: 16, fill: "currentColor" }
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
pub fn FavoritesPage() -> Element {
    let data = DATA();
    let mut items = Vec::new();
    for chat in &data.chats {
        for message in &chat.messages {
            if message.favorite {
                items.push((
                    chat.id.clone(),
                    chat.display_title(),
                    message.id.clone(),
                    message.role.clone(),
                    message.content.clone(),
                    message.created_at,
                ));
            }
        }
    }
    items.sort_by(|a, b| b.5.cmp(&a.5));

    rsx! {
        div { class: "screen",
            PageHeader { title: i18n::t("favorites.title"),
                subtitle: i18n::tn("common.savedMessages", items.len())
            }
            div { class: "content",
                if items.is_empty() {
                    EmptyState {
                        title: i18n::t("favorites.emptyTitle"),
                        message: i18n::t("favorites.emptyMessage"),
                    }
                } else {
                    for (chat_id, title, message_id, role, content, at) in items {
                        {
                            let id = chat_id.clone();
                            let mid = message_id.clone();
                            rsx! {
                                div {
                                    key: "{message_id}",
                                    class: "favorite-card",
                                    onclick: move |_| open_chat(id.clone()),
                                    div { class: "favorite-head",
                                        span { class: "favorite-title", "{title}" }
                                        button {
                                            class: "icon-btn small favorite",
                                            onclick: move |event| {
                                                event.stop_propagation();
                                                crate::app::toggle_favorite(&chat_id, &mid);
                                            },
                                            Icon { icon: LdStar, width: 16, height: 16, fill: "currentColor" }
                                        }
                                    }
                                    div { class: "md favorite-content", dangerous_inner_html: markdown::to_html(&content) }
                                    div { class: "favorite-meta", {format!("{} · {}", role_label(&role), clock(at))} }
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
pub fn ExtensionsPage() -> Element {
    let data = DATA();
    let skills = data.skills.len();
    let prompts = data.prompts.len();
    let mcp = data.mcp_servers.len();
    let search = if data.settings.search_provider == "none" {
        i18n::t("extensions.notConfigured")
    } else {
        data.settings.search_provider.clone()
    };
    rsx! {
        div { class: "screen",
            PageHeader { title: i18n::t("extensions.title"), subtitle: i18n::t("extensions.subtitle") }
            div { class: "content",
                SectionCard { title: i18n::t("extensions.available"), subtitle: "",
                    ExtensionRow {
                        icon: rsx! { Icon { icon: LdWrench, width: 20, height: 20, fill: "currentColor" } },
                        title: i18n::t("extensions.skills"),
                        subtitle: i18n::tn("common.skills", skills),
                        onclick: move |_| navigate(Screen::Skills),
                    }
                    ExtensionRow {
                        icon: rsx! { Icon { icon: LdBookOpen, width: 20, height: 20, fill: "currentColor" } },
                        title: i18n::t("extensions.prompts"),
                        subtitle: i18n::tn("common.prompts", prompts),
                        onclick: move |_| navigate(Screen::Prompts),
                    }
                    ExtensionRow {
                        icon: rsx! { Icon { icon: LdServer, width: 20, height: 20, fill: "currentColor" } },
                        title: i18n::t("extensions.mcp"),
                        subtitle: i18n::tn("common.servers", mcp),
                        onclick: move |_| navigate(Screen::Mcp),
                    }
                    ExtensionRow {
                        icon: rsx! { Icon { icon: LdGlobe, width: 20, height: 20, fill: "currentColor" } },
                        title: i18n::t("extensions.webSearch"),
                        subtitle: "{search}",
                        onclick: move |_| navigate(Screen::WebSearch),
                    }
                }
            }
        }
    }
}

#[component]
fn ExtensionRow(
    icon: Element,
    title: String,
    subtitle: String,
    onclick: EventHandler<()>,
) -> Element {
    rsx! {
        button { class: "setting-row", onclick: move |_| onclick.call(()),
            div { class: "row-icon primary", {icon} }
            div { class: "setting-row-text",
                span { "{title}" }
                small { "{subtitle}" }
            }
            Icon { icon: LdChevronRight, width: 18, height: 18, fill: "currentColor" }
        }
    }
}

fn role_label(role: &str) -> String {
    match role {
        "assistant" => i18n::t("common.role.assistant"),
        "summary" => i18n::t("common.role.summary"),
        _ => i18n::t("common.role.user"),
    }
}
