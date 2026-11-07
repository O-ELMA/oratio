use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::*, Icon};
use oratio::i18n;
use oratio::store::Conversation;

use crate::app::{
    close_drawer, delete_chat, navigate, open_chat, switch_assistant, toggle_pin, Screen, DATA,
};
use crate::ui::assistants::AssistantPickerSheet;
use crate::ui::common::{date_label, greeting, time_ago, Avatar, ConfirmButton, Sheet};

enum Row {
    Header(String),
    Chat(Conversation),
}

#[component]
pub fn Drawer(current_id: Option<String>) -> Element {
    let data = DATA();
    let settings = data.settings.clone();
    let chats = data.chats.clone();
    let current_assistant = data.current_assistant();
    let mut picker_open = use_signal(|| false);

    let mut rows = Vec::new();
    let mut pinned: Vec<Conversation> = chats.iter().filter(|chat| chat.pinned).cloned().collect();
    pinned.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    if !pinned.is_empty() {
        rows.push(Row::Header(i18n::t("drawer.pinned")));
        rows.extend(pinned.into_iter().map(Row::Chat));
    }
    let mut others: Vec<Conversation> = chats.iter().filter(|chat| !chat.pinned).cloned().collect();
    others.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    let mut last_label = String::new();
    for chat in others {
        let label = date_label(chat.updated_at);
        if label != last_label {
            rows.push(Row::Header(label.clone()));
            last_label = label;
        }
        rows.push(Row::Chat(chat));
    }

    let user_name = settings.user_name();
    let user_color = settings.user_avatar_color.clone();

    rsx! {
        div {
            class: "overlay drawer-overlay",
            onclick: move |_| close_drawer(),
            aside {
                class: "drawer",
                onclick: move |event| event.stop_propagation(),
                div { class: "drawer-scroll",
                    div { class: "profile",
                        Avatar { name: user_name.clone(), color: user_color.clone(), size: 50 }
                        div { class: "profile-text",
                            div { class: "profile-name-row",
                                span { "{user_name}" }
                            }
                            span { class: "profile-greeting", "{greeting()}" }
                        }
                    }

                    div { class: "drawer-actions",
                        div {
                            class: "drawer-entry",
                            onclick: move |_| navigate(Screen::History),
                            div { class: "drawer-entry-icon",
                                Icon { icon: LdHistory, width: 18, height: 18, fill: "currentColor" }
                            }
                            span { {i18n::t("drawer.history")} }
                            button {
                                class: "icon-btn tiny",
                                onclick: move |event| {
                                    event.stop_propagation();
                                    navigate(Screen::Search);
                                },
                                Icon { icon: LdSearch, width: 15, height: 15, fill: "currentColor" }
                            }
                        }
                        div {
                            class: "drawer-entry",
                            onclick: move |_| navigate(Screen::Assistants),
                            div { class: "drawer-entry-icon",
                                Icon { icon: LdBot, width: 18, height: 18, fill: "currentColor" }
                            }
                            span { {i18n::t("drawer.assistants")} }
                        }
                        div {
                            class: "drawer-entry",
                            onclick: move |_| navigate(Screen::Extensions),
                            div { class: "drawer-entry-icon",
                                Icon { icon: LdLayers, width: 18, height: 18, fill: "currentColor" }
                            }
                            span { {i18n::t("drawer.extensions")} }
                        }
                    }

                    div { class: "drawer-list-head",
                        div { class: "folder-chip",
                            Icon { icon: LdFolder, width: 14, height: 14, fill: "currentColor" }
                            span { {i18n::t("drawer.allChats")} }
                        }
                        span { class: "drawer-count",
                            {i18n::tn("common.chats", chats.len())}
                        }
                    }

                    div { class: "drawer-chats",
                        if chats.is_empty() {
                            div { class: "drawer-empty", {i18n::t("drawer.empty")} }
                        }
                        for row in rows {
                            {
                                match row {
                                    Row::Header(label) => rsx! {
                                        div { key: "{label}", class: "date-header", "{label}" }
                                    },
                                    Row::Chat(chat) => {
                                        let id = chat.id.clone();
                                        let selected = current_id.as_deref() == Some(id.as_str());
                                        rsx! {
                                            DrawerChatRow { key: "{id}", chat: chat.clone(), selected }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "drawer-footer",
                    div {
                        class: "assistant-picker",
                        onclick: move |_| picker_open.set(true),
                        Avatar { name: current_assistant.display_name(), color: current_assistant.color(), size: 34 }
                        div { class: "assistant-picker-text",
                            span { "{current_assistant.display_name()}" }
                            small { {i18n::t("drawer.assistant")} }
                        }
                        button {
                            class: "icon-btn small",
                            onclick: move |event| {
                                event.stop_propagation();
                                navigate(Screen::AssistantEdit(Some(current_assistant.id.clone())));
                            },
                            Icon { icon: LdSettings, width: 17, height: 17, fill: "currentColor" }
                        }
                    }
                    div { class: "drawer-bottom",
                        DrawerAction {
                            icon: rsx! { Icon { icon: LdHeart, width: 20, height: 20, fill: "currentColor" } },
                            label: i18n::t("drawer.favorites"),
                            onclick: move |_| navigate(Screen::Favorites),
                        }
                        DrawerAction {
                            icon: rsx! { Icon { icon: LdBarChart3, width: 20, height: 20, fill: "currentColor" } },
                            label: i18n::t("drawer.stats"),
                            onclick: move |_| navigate(Screen::Stats),
                        }
                        DrawerAction {
                            icon: rsx! { Icon { icon: LdSettings, width: 20, height: 20, fill: "currentColor" } },
                            label: i18n::t("drawer.settings"),
                            onclick: move |_| navigate(Screen::Settings),
                        }
                    }
                }
            }
        }

        if picker_open() {
            AssistantPickerSheet {
                current_id: current_assistant.id.clone(),
                on_close: move |_| picker_open.set(false),
                on_select: move |id: String| {
                    picker_open.set(false);
                    switch_assistant(&id);
                },
            }
        }
    }
}

#[component]
fn DrawerAction(icon: Element, label: String, onclick: EventHandler<()>) -> Element {
    rsx! {
        button { class: "drawer-action", onclick: move |_| onclick.call(()),
            {icon}
            span { "{label}" }
        }
    }
}

#[component]
fn DrawerChatRow(chat: Conversation, selected: bool) -> Element {
    let mut menu_open = use_signal(|| false);
    let id = chat.id.clone();
    let open_id = id.clone();
    let pin_id = id.clone();
    let delete_id = id.clone();
    let title = chat.display_title();
    let subtitle = chat.preview();
    rsx! {
        div {
            class: if selected { "drawer-chat selected" } else { "drawer-chat" },
            onclick: move |_| open_chat(open_id.clone()),
            div { class: "drawer-chat-main",
                div { class: "drawer-chat-title",
                    if chat.pinned {
                        Icon { icon: LdPin, width: 12, height: 12, fill: "currentColor" }
                    }
                    span { "{title}" }
                }
                if !subtitle.is_empty() {
                    div { class: "drawer-chat-preview", "{subtitle}" }
                }
                div { class: "drawer-chat-meta",
                    "{time_ago(chat.updated_at)}"
                    " · "
                    {i18n::tn("common.messages", chat.messages.len())}
                }
            }
            button {
                class: "icon-btn tiny",
                onclick: move |event| {
                    event.stop_propagation();
                    menu_open.set(true);
                },
                Icon { icon: LdEllipsis, width: 15, height: 15, fill: "currentColor" }
            }
        }
        if menu_open() {
            Sheet { title: i18n::t("drawer.chatSheet"), on_close: move |_| menu_open.set(false),
                div { class: "sheet-list",
                    button {
                        class: "sheet-row",
                        onclick: move |_| {
                            menu_open.set(false);
                            toggle_pin(&pin_id);
                        },
                        div { class: "row-icon",
                            if chat.pinned {
                                Icon { icon: LdPinOff, width: 18, height: 18, fill: "currentColor" }
                            } else {
                                Icon { icon: LdPin, width: 18, height: 18, fill: "currentColor" }
                            }
                        }
                        div { class: "sheet-row-text",
                            span { if chat.pinned { {i18n::t("drawer.unpin")} } else { {i18n::t("drawer.pin")} } }
                        }
                    }
                    ConfirmButton {
                        class: "sheet-row danger",
                        text: i18n::t("common.delete"),
                        onconfirm: move |_| {
                            menu_open.set(false);
                            delete_chat(&delete_id);
                        },
                        div { class: "row-icon danger",
                            Icon { icon: LdTrash2, width: 18, height: 18, fill: "currentColor" }
                        }
                    }
                }
            }
        }
    }
}
