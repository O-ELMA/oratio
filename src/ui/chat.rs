use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::*, Icon};
use oratio::{
    api::{self, ApiMessage, Delta},
    i18n, markdown,
    store::{self, Assistant, ChatMessage},
};

use crate::app::{
    self, cancel_dictation, delete_chat, delete_message, fork_chat, is_streaming, mutate, navigate,
    new_chat, persist, regenerate, send_message, set_chat_error, set_status, stop_generation,
    switch_model, toast, toggle_dictation, toggle_favorite, Screen, VoiceModel, VoicePhase,
    CHAT_ERROR, DATA, DRAFT, DRAWER, PENDING, SPEECH, STATUS, STREAM,
};
use crate::ui::common::{
    assistant_color, clock, copy_text, format_tokens, open_sketch, pick_image, pick_text_file,
    Avatar, ConfirmButton, ConfirmDialog, EmptyState, ModelPickerSheet, Sheet,
};

#[component]
pub fn ChatScreen(chat_id: Option<String>) -> Element {
    let data = DATA();
    let stream_state = STREAM();
    let drawer_open = DRAWER();
    let status = STATUS();
    let speech = SPEECH();
    let pending = PENDING();
    let draft_state = DRAFT();
    let chat = chat_id.as_ref().and_then(|id| data.chat(id)).cloned();

    if chat_id.is_some() && chat.is_none() {
        return rsx! {
            div { class: "screen",
                header { class: "page-header",
                    button { class: "icon-btn large", onclick: move |_| new_chat(),
                        Icon { icon: LdArrowLeft, width: 22, height: 22, fill: "currentColor" }
                    }
                    div { class: "page-header-text", h1 { {i18n::t("chat.notFound")} } }
                }
                div { class: "content",
                    EmptyState { title: i18n::t("chat.notFound"), message: i18n::t("chat.notFoundMessage"),
                        button { class: "btn", onclick: move |_| new_chat(), {i18n::t("chat.startNew")} }
                    }
                }
            }
        };
    }

    let assistant = match &chat {
        Some(chat) => data.assistant_for_chat(chat),
        None => data
            .assistant(&draft_state.assistant_id)
            .or_else(|| data.assistant(&data.settings.assistant_id))
            .cloned()
            .unwrap_or_else(|| data.current_assistant()),
    };
    let provider_id = chat
        .as_ref()
        .map(|chat| chat.provider_id.clone())
        .unwrap_or_else(|| draft_state.provider_id.clone());
    let model = chat
        .as_ref()
        .map(|chat| chat.model.clone())
        .unwrap_or_else(|| draft_state.model.clone());
    let provider_name = data
        .provider(&provider_id)
        .map(|provider| provider.name.clone())
        .unwrap_or_else(|| i18n::t("common.noProvider"));
    let title = chat
        .as_ref()
        .map(|chat| chat.display_title())
        .unwrap_or_else(|| i18n::t("common.newChat"));
    let messages = chat
        .as_ref()
        .map(|chat| chat.messages.clone())
        .unwrap_or_default();
    let can_fork = !messages.is_empty();
    let settings = data.settings.clone();
    let user_name = settings.user_name();
    let user_color = if settings.user_avatar_color.trim().is_empty() {
        "#8E4955".to_string()
    } else {
        settings.user_avatar_color.clone()
    };

    let streaming = chat_id
        .as_ref()
        .map(|id| {
            stream_state
                .as_ref()
                .map(|state| state.chat_id == *id && state.running)
                .unwrap_or(false)
        })
        .unwrap_or(false);
    let error = chat_id.as_ref().and_then(|id| {
        CHAT_ERROR()
            .filter(|(error_id, _)| error_id == id)
            .map(|(_, message)| message)
    });

    let mut text = use_signal(String::new);
    let mut model_open = use_signal(|| false);
    let mut menu_open = use_signal(|| false);
    let mut rename_open = use_signal(|| false);
    let rename_initial = title.clone();
    let mut rename_text = use_signal(move || rename_initial);
    let mut clear_open = use_signal(|| false);
    let mut prompt_open = use_signal(|| false);
    let prompt_initial = assistant.system_prompt.clone();
    let mut prompt_text = use_signal(move || prompt_initial);
    let mut picker_open = use_signal(|| false);
    let mut prompt_picker_open = use_signal(|| false);
    let mut tools_open = use_signal(|| false);
    let mut compressing = use_signal(|| false);
    let mut mode = use_signal(|| 0u8);
    let chat_id_initial = chat_id.clone();
    let chat_state = use_signal(move || chat_id_initial);

    let chat_title = title.clone();
    let rename_title = title.clone();
    let prompt_source = assistant.clone();
    let assistant_for_prompt = assistant.clone();
    let assistant_for_search = assistant.clone();
    let assistant_edit_id = assistant.id.clone();
    let assistant_name = assistant.display_name();
    let send_on_enter = settings.send_on_enter;
    let voice_recording = matches!(speech.phase.clone(), VoicePhase::Recording);
    let voice_transcribing = matches!(speech.phase.clone(), VoicePhase::Transcribing);
    let voice_permission = matches!(speech.phase.clone(), VoicePhase::Permission);
    let voice_input_locked = voice_recording || voice_transcribing;
    let voice_downloading = matches!(speech.model.clone(), VoiceModel::Downloading { .. });
    let voice_percent = match speech.model.clone() {
        VoiceModel::Downloading { received, total } if total > 0 => {
            (received * 100 / total).min(100)
        }
        _ => 0,
    };
    let last_assistant_index = messages
        .iter()
        .rposition(|message| message.role == "assistant");

    use_effect(move || {
        let _ = DATA();
        let _ = STREAM();
        let _ = document::eval(
            "const m = document.getElementById('messages'); if (m) { m.scrollTop = m.scrollHeight; }",
        );
    });

    use_drop(|| cancel_dictation());

    use_effect(move || {
        let _ = text();
        let _ = document::eval(
            "const el = document.querySelector('.composer-input'); if (el) { el.style.height = 'auto'; el.style.height = Math.min(el.scrollHeight, 150) + 'px'; }",
        );
    });

    rsx! {
        div { class: "screen chat-screen",
            if assistant.use_gradient_background {
                div { class: "mesh-bg" }
            }
            header { class: "chat-topbar",
                div { class: "top-capsule",
                    button { class: "icon-btn", onclick: move |_| app::open_drawer(),
                        Icon { icon: LdMenu, width: 20, height: 20, fill: "currentColor" }
                    }
                }
                div {
                    class: "chat-heading",
                    onclick: move |_| {
                        if chat_state().is_some() {
                            rename_open.set(true);
                        }
                    },
                    if chat_id.is_some() {
                        h1 { "{chat_title}" }
                    }
                }
                div { class: "top-spacer" }
            }

            main { id: "messages", class: "chat-body",
                if mode() == 1 {
                    crate::ui::media::MediaPanel {}
                } else if mode() == 2 {
                    crate::ui::translator::TranslatorPanel {}
                } else if messages.is_empty() {
                    div { class: "chat-welcome",
                        Avatar {
                            name: assistant_name.clone(),
                            color: assistant.color(),
                            size: 64
                        }
                        h2 { "{assistant_name}" }
                        p { {i18n::t("chat.welcome")} }
                    }
                } else {
                    for (index, message) in messages.iter().enumerate() {
                        MessageView {
                            key: "{message.id}",
                            message: message.clone(),
                            chat_id: chat_id.clone(),
                            assistant: assistant.clone(),
                            model: model.clone(),
                            provider_name: provider_name.clone(),
                            user_name: user_name.clone(),
                            user_color: user_color.clone(),
                            show_bubble: settings.show_assistant_bubble,
                            bubble_opacity: settings.bubble_opacity,
                            show_tokens: settings.show_token_usage,
                            show_model_name: settings.show_model_name,
                            streaming: streaming && index + 1 == messages.len() && message.role == "assistant",
                            last_assistant: Some(index) == last_assistant_index,
                        }
                    }
                    if streaming && messages.last().map(|m| m.role == "user").unwrap_or(false) {
                        div { class: "msg assistant",
                            div { class: "assistant-head",
                                Avatar { name: assistant_name.clone(), color: assistant.color(), size: 28 }
                                span { {i18n::t("chat.thinking")} }
                            }
                            div { class: "typing", span {} span {} span {} }
                        }
                    }
                }
            }

            if mode() == 0 {
                if let Some(error) = error {
                    div { class: "errorbar",
                        Icon { icon: LdTriangleAlert, width: 18, height: 18, fill: "currentColor" }
                        span { class: "error-text", "{error}" }
                        if let Some(id) = chat_id.clone() {
                            button {
                                class: "btn ghost small",
                                onclick: move |_| regenerate(&id),
                                {i18n::t("chat.retry")}
                            }
                        }
                        button {
                            class: "icon-btn small",
                            onclick: move |_| set_chat_error(None),
                            Icon { icon: LdX, width: 16, height: 16, fill: "currentColor" }
                        }
                    }
                }
            }

            footer { class: "composer-wrap",
                div { class: "mode-pills",
                    for (value, label) in [(0u8, i18n::t("chat.mode.chat")), (1u8, i18n::t("chat.mode.media")), (2u8, i18n::t("chat.mode.translator"))] {
                        button {
                            class: if mode() == value { "mode-pill selected" } else { "mode-pill" },
                            onclick: move |_| mode.set(value),
                            "{label}"
                        }
                    }
                }
                if mode() == 0 {
                    if voice_downloading {
                        div { class: "voice-progress",
                            div { class: "voice-progress-label",
                                span { {i18n::t("chat.voiceDownloading")} }
                                span { "{voice_percent}%" }
                            }
                            div { class: "voice-progress-track",
                                div { class: "voice-progress-fill", style: "width: {voice_percent}%;" }
                            }
                        }
                    }
                    if matches!(speech.model.clone(), VoiceModel::Failed) {
                        div { class: "voice-status", {i18n::t("chat.voiceDownloadFailed")} }
                    }
                    if matches!(speech.phase.clone(), VoicePhase::Permission) {
                        div { class: "voice-status", {i18n::t("chat.voicePermission")} }
                    }
                    if !pending.is_empty() {
                    div { class: "pending-row",
                        for attachment in pending.clone() {
                            div { key: "{attachment.id}", class: "pending-item",
                                if attachment.mime.starts_with("image/") && !attachment.data_url.is_empty() {
                                    img { src: "{attachment.data_url}", alt: "{attachment.name}" }
                                } else {
                                    div { class: "pending-file",
                                        Icon { icon: LdFileText, width: 16, height: 16, fill: "currentColor" }
                                        span { "{attachment.name}" }
                                    }
                                }
                                button {
                                    class: "pending-remove",
                                    onclick: move |_| crate::app::remove_attachment(&attachment.id),
                                    Icon { icon: LdX, width: 12, height: 12, fill: "currentColor" }
                                }
                            }
                        }
                    }
                }
                if streaming || status.is_some() {
                    div { class: "stream-status",
                        span { class: "pulse-dot" }
                        if let Some(status) = status.clone() {
                            "{status}"
                        } else {
                            {i18n::t("chat.generating")}
                        }
                        if streaming {
                            button {
                                class: "link-btn",
                                onclick: move |_| stop_generation(),
                                {i18n::t("chat.stop")}
                            }
                        }
                    }
                }
                div { class: "composer",
                    if voice_transcribing {
                        div { class: "voice-spinner" }
                    }
                    textarea {
                        class: if voice_transcribing { "composer-input transcribing" } else { "composer-input" },
                        rows: "1",
                        initial_value: "{text}",
                        disabled: voice_input_locked,
                        oninput: move |event| text.set(event.value()),
                        onkeydown: move |event| {
                            if !send_on_enter
                                || event.key() != dioxus::prelude::Key::Enter
                                || event.modifiers().shift()
                            {
                                return;
                            }
                            event.prevent_default();
                            if streaming {
                                return;
                            }
                            let value = text().trim().to_string();
                            if value.is_empty() && PENDING.peek().is_empty() {
                                return;
                            }
                            if DATA.peek().providers.is_empty() {
                                toast(i18n::t("chat.addProviderFirst"));
                                navigate(Screen::Providers);
                                return;
                            }
                            if send_message(chat_state(), value).is_some() {
                                text.set(String::new());
                                let _ = document::eval(
                                    "const el=document.querySelector('.composer-input'); if(el){el.value='';}",
                                );
                            }
                        },
                    }
                    div { class: "composer-toolbar",
                        button {
                            class: "icon-btn toolbar",
                            title: i18n::t("chat.newChatTooltip"),
                            onclick: move |_| new_chat(),
                            Icon { icon: LdMessageSquarePlus, width: 19, height: 19, fill: "currentColor" }
                        }
                        if can_fork {
                            button {
                                class: "icon-btn toolbar",
                                title: i18n::t("chat.fork"),
                                onclick: move |_| {
                                    if let Some(id) = chat_state() {
                                        fork_chat(&id);
                                    }
                                },
                                Icon { icon: LdGitFork, width: 19, height: 19, fill: "currentColor" }
                            }
                        }
                        button {
                            class: "model-chip",
                            onclick: move |_| model_open.set(true),
                            Icon { icon: LdSparkles, width: 15, height: 15, fill: "currentColor" }
                            span { if model.is_empty() { {i18n::t("chat.selectModel")} } else { "{model}" } }
                            Icon { icon: LdChevronDown, width: 14, height: 14, fill: "currentColor" }
                        }
                        div { class: "composer-spacer" }
                        button {
                            class: "icon-btn toolbar",
                            onclick: move |_| tools_open.set(true),
                            Icon { icon: LdPlus, width: 20, height: 20, fill: "currentColor" }
                        }
                        button {
                            class: if voice_recording || voice_transcribing { "icon-btn toolbar listening" } else { "icon-btn toolbar" },
                            disabled: voice_permission || voice_downloading,
                            title: if voice_recording {
                                i18n::t("chat.voiceStop")
                            } else if voice_transcribing {
                                i18n::t("chat.voiceCancel")
                            } else {
                                i18n::t("chat.voiceStart")
                            },
                            onclick: move |_| toggle_dictation(),
                            if voice_transcribing {
                                Icon { icon: LdSquare, width: 15, height: 15, fill: "currentColor" }
                            } else {
                                Icon { icon: LdMic, width: 19, height: 19, fill: "currentColor" }
                            }
                        }
                        button {
                            class: if streaming { "send-btn stop" } else { "send-btn" },
                            disabled: !streaming && text().trim().is_empty() && PENDING().is_empty(),
                            onclick: move |_| {
                                if streaming {
                                    stop_generation();
                                    return;
                                }
                                let value = text().trim().to_string();
                                if value.is_empty() && PENDING.peek().is_empty() {
                                    return;
                                }
                                if DATA.peek().providers.is_empty() {
                                    toast(i18n::t("chat.addProviderFirst"));
                                    navigate(Screen::Providers);
                                    return;
                                }
                                if send_message(chat_state(), value).is_some() {
                                    text.set(String::new());
                                    let _ = document::eval(
                                        "const el=document.querySelector('.composer-input'); if (el) { el.value=''; }",
                                    );
                                }
                            },
                            if streaming {
                                Icon { icon: LdSquare, width: 15, height: 15, fill: "currentColor" }
                            } else {
                                Icon { icon: LdArrowUp, width: 20, height: 20, fill: "currentColor" }
                            }
                        }
                    }
                }
                }
            }

            if tools_open() {
                Sheet { title: i18n::t("chat.addToChat"), on_close: move |_| tools_open.set(false),
                    div { class: "tool-grid",
                        button {
                            class: "tool-tile",
                            onclick: move |_| {
                                tools_open.set(false);
                                pick_image(true);
                            },
                            div { class: "row-icon primary", Icon { icon: LdCamera, width: 20, height: 20, fill: "currentColor" } }
                            span { {i18n::t("chat.camera")} }
                        }
                        button {
                            class: "tool-tile",
                            onclick: move |_| {
                                tools_open.set(false);
                                pick_image(false);
                            },
                            div { class: "row-icon secondary", Icon { icon: LdImages, width: 20, height: 20, fill: "currentColor" } }
                            span { {i18n::t("chat.gallery")} }
                        }
                        button {
                            class: "tool-tile",
                            onclick: move |_| {
                                tools_open.set(false);
                                open_sketch();
                            },
                            div { class: "row-icon tertiary", Icon { icon: LdPenTool, width: 20, height: 20, fill: "currentColor" } }
                            span { {i18n::t("chat.sketch")} }
                        }
                        button {
                            class: "tool-tile",
                            onclick: move |_| {
                                tools_open.set(false);
                                pick_text_file();
                            },
                            div { class: "row-icon", Icon { icon: LdFileText, width: 20, height: 20, fill: "currentColor" } }
                            span { {i18n::t("chat.textFile")} }
                        }
                        button {
                            class: "tool-tile",
                            onclick: move |_| {
                                tools_open.set(false);
                                prompt_picker_open.set(true);
                            },
                            div { class: "row-icon primary", Icon { icon: LdBookOpen, width: 20, height: 20, fill: "currentColor" } }
                            span { {i18n::t("chat.prompt")} }
                        }
                        button {
                            class: "tool-tile",
                            onclick: move |_| {
                                tools_open.set(false);
                                menu_open.set(true);
                            },
                            div { class: "row-icon secondary", Icon { icon: LdSlidersHorizontal, width: 20, height: 20, fill: "currentColor" } }
                            span { {i18n::t("chat.moreTools")} }
                        }
                    }
                    div { class: "hint",
                        {i18n::t("chat.toolsHint")}
                    }
                }
            }

            if drawer_open {
                crate::ui::drawer::Drawer { current_id: chat_id.clone() }
            }

            if model_open() {
                ModelPickerSheet {
                    current_provider: provider_id.clone(),
                    current_model: model.clone(),
                    on_close: move |_| model_open.set(false),
                    on_select: move |value: (String, String)| {
                        switch_model(chat_state().as_deref(), &value.0, &value.1);
                    },
                }
            }

            if menu_open() {
                Sheet { title: i18n::t("drawer.chatSheet"), on_close: move |_| menu_open.set(false),
                    div { class: "sheet-list",
                        button {
                            class: "sheet-row",
                            onclick: move |_| {
                                menu_open.set(false);
                                picker_open.set(true);
                            },
                            div { class: "row-icon primary",
                                Icon { icon: LdBot, width: 18, height: 18, fill: "currentColor" }
                            }
                            div { class: "sheet-row-text",
                                span { {i18n::t("chat.assistant")} }
                                small { "{assistant.display_name()}" }
                            }
                            Icon { icon: LdChevronRight, width: 18, height: 18, fill: "currentColor" }
                        }
                        button {
                            class: "sheet-row",
                            onclick: move |_| {
                                prompt_text.set(prompt_source.system_prompt.clone());
                                menu_open.set(false);
                                prompt_open.set(true);
                            },
                            div { class: "row-icon tertiary",
                                Icon { icon: LdScrollText, width: 18, height: 18, fill: "currentColor" }
                            }
                            div { class: "sheet-row-text",
                                span { {i18n::t("chat.systemPrompt")} }
                                small { if assistant.system_prompt.is_empty() { {i18n::t("chat.notSet")} } else { {i18n::t("chat.customInstructions")} } }
                            }
                            Icon { icon: LdChevronRight, width: 18, height: 18, fill: "currentColor" }
                        }
                        button {
                            class: "sheet-row",
                            onclick: move |_| {
                                menu_open.set(false);
                                model_open.set(true);
                            },
                            div { class: "row-icon secondary",
                                Icon { icon: LdSparkles, width: 18, height: 18, fill: "currentColor" }
                            }
                            div { class: "sheet-row-text",
                                span { {i18n::t("chat.model")} }
                                small { "{provider_name} · {model}" }
                            }
                            Icon { icon: LdChevronRight, width: 18, height: 18, fill: "currentColor" }
                        }
                        if !data.prompts.is_empty() {
                            button {
                                class: "sheet-row",
                                onclick: move |_| {
                                    menu_open.set(false);
                                    prompt_picker_open.set(true);
                                },
                                div { class: "row-icon primary",
                                    Icon { icon: LdBookOpen, width: 18, height: 18, fill: "currentColor" }
                                }
                                div { class: "sheet-row-text",
                                    span { {i18n::t("chat.insertPrompt")} }
                                    small { {i18n::tn("common.prompts", data.prompts.len())} }
                                }
                                Icon { icon: LdChevronRight, width: 18, height: 18, fill: "currentColor" }
                            }
                        }
                        button {
                            class: "sheet-row",
                            onclick: move |_| {
                                let data = DATA.peek();
                                let enabled = data
                                    .chat(&chat_state().unwrap_or_default())
                                    .map(|chat| data.assistant_for_chat(chat).enable_web_search)
                                    .unwrap_or_else(|| assistant_for_prompt.enable_web_search);
                                let settings = data.settings.clone();
                                drop(data);
                                if !enabled && (settings.search_provider == "none" || settings.search_api_key.trim().is_empty()) {
                                    toast(i18n::t("chat.searchNotConfigured"));
                                    menu_open.set(false);
                                    navigate(Screen::WebSearch);
                                    return;
                                }
                                let mut edited = assistant_for_search.clone();
                                edited.enable_web_search = !enabled;
                                app::save_assistant(edited);
                                toast(if enabled { i18n::t("chat.webSearchOffToast") } else { i18n::t("chat.webSearchOnToast") });
                            },
                            div { class: "row-icon secondary",
                                Icon { icon: LdGlobe, width: 18, height: 18, fill: "currentColor" }
                            }
                            div { class: "sheet-row-text",
                                span { {i18n::t("chat.webSearch")} }
                                small { if assistant.enable_web_search { {i18n::t("chat.webSearchOn")} } else { {i18n::t("chat.webSearchOff")} } }
                            }
                        }
                        if messages.len() > 6 {
                            button {
                                class: "sheet-row",
                                onclick: move |_| {
                                    menu_open.set(false);
                                    let Some(id) = chat_state() else { return };
                                    if is_streaming(&id) {
                                        toast(i18n::t("chat.stopCurrent"));
                                        return;
                                    }
                                    if compressing() { return; }
                                    let prepared = {
                                        let data = DATA.peek();
                                        let Some(chat) = data.chat(&id) else { return };
                                        let Some(provider) = data.provider(&chat.provider_id) else {
                                            toast(i18n::t("chat.providerNotFound"));
                                            return;
                                        };
                                        if chat.messages.len() <= 6 { return }
                                        let keep = 4;
                                        let older = &chat.messages[..chat.messages.len() - keep];
                                        let transcript = older
                                            .iter()
                                            .map(|message| format!("{}: {}\n", message.role, message.content))
                                            .collect::<String>();
                                        (
                                            provider.base_url.clone(),
                                            provider.api_key.clone(),
                                            chat.model.clone(),
                                            keep,
                                            transcript,
                                        )
                                    };
                                    let (base_url, api_key, model, keep, transcript) = prepared;
                                    compressing.set(true);
                                    set_status(Some(i18n::t("chat.compressing")));
                                    let mut compressing_signal = compressing;
                                    dioxus::prelude::spawn(async move {
                                        let request = vec![
                                            ApiMessage {
                                                role: "system".into(),
                                                content: "Summarize the earlier conversation as a compact context checkpoint. Keep facts, names, decisions, code identifiers and open questions. Use Markdown and no preamble.".into(),
                                                images: Vec::new(), tool_calls: Vec::new(), tool_call_id: None,
                                            },
                                            ApiMessage { role: "user".into(), content: transcript, images: Vec::new(), tool_calls: Vec::new(), tool_call_id: None },
                                        ];
                                        let mut summary = String::new();
                                        let result = api::stream_chat(&base_url, &api_key, &model, &request, |delta| {
                                            if let Delta::Content(text) = delta {
                                                summary.push_str(&text);
                                            }
                                            true
                                        })
                                        .await;
                                        match result {
                                            Ok(()) if !summary.trim().is_empty() => {
                                                mutate(|data| {
                                                    if let Some(chat) = data.chat_mut(&id) {
                                                        if chat.messages.len() > keep {
                                                            let tail = chat.messages.split_off(chat.messages.len() - keep);
                                                            chat.messages.clear();
                                                            chat.messages.push(ChatMessage {
                                                                role: "summary".into(),
                                                                content: summary.trim().to_string(),
                                                                created_at: store::now_ms(),
                                                                ..Default::default()
                                                            });
                                                            chat.messages.extend(tail);
                                                            chat.updated_at = store::now_ms();
                                                        }
                                                    }
                                                });
                                                persist();
                                                toast(i18n::t("chat.contextCompressed"));
                                            }
                                            Ok(()) => toast(i18n::t("chat.emptySummary")),
                                            Err(error) => toast(i18n::ta("chat.compressionFailed", &[("error", &error)])),
                                        }
                                        compressing_signal.set(false);
                                        set_status(None);
                                    });
                                },
                                div { class: "row-icon tertiary",
                                    Icon { icon: LdEraser, width: 18, height: 18, fill: "currentColor" }
                                }
                                div { class: "sheet-row-text",
                                    span { {i18n::t("chat.compress")} }
                                    small { {i18n::t("chat.compressNote")} }
                                }
                            }
                        }
                        if chat_id.is_some() {
                            button {
                                class: "sheet-row",
                                onclick: move |_| {
                                    menu_open.set(false);
                                    rename_text.set(rename_title.clone());
                                    rename_open.set(true);
                                },
                                div { class: "row-icon",
                                    Icon { icon: LdPencil, width: 18, height: 18, fill: "currentColor" }
                                }
                                div { class: "sheet-row-text", span { {i18n::t("chat.rename")} } }
                            }
                            button {
                                class: "sheet-row",
                                onclick: move |_| {
                                    menu_open.set(false);
                                    clear_open.set(true);
                                },
                                div { class: "row-icon",
                                    Icon { icon: LdEraser, width: 18, height: 18, fill: "currentColor" }
                                }
                                div { class: "sheet-row-text", span { {i18n::t("chat.clear")} } }
                            }
                            ConfirmButton {
                                class: "sheet-row danger",
                                text: i18n::t("common.delete"),
                                onconfirm: move |_| {
                                    menu_open.set(false);
                                    if let Some(id) = chat_state() {
                                        delete_chat(&id);
                                    }
                                },
                                div { class: "row-icon danger",
                                    Icon { icon: LdTrash2, width: 18, height: 18, fill: "currentColor" }
                                }
                            }
                        }
                    }
                }
            }

            if picker_open() {
                Sheet { title: i18n::t("chat.selectAssistant"), on_close: move |_| picker_open.set(false),
                    div { class: "sheet-list",
                        for item in data.assistants.iter() {
                            {
                                let item = item.clone();
                                let selected = item.id == assistant.id;
                                rsx! {
                                    button {
                                        key: "{item.id}",
                                        class: if selected { "assistant-row selected" } else { "assistant-row" },
                                        onclick: move |_| {
                                            picker_open.set(false);
                                            if let Some(id) = chat_state() {
                                                let (provider, model) = {
                                                    let data = DATA.peek();
                                                    data.model_for_assistant(&item)
                                                };
                                                mutate(|data| {
                                                    if let Some(chat) = data.chat_mut(&id) {
                                                        chat.assistant_id = item.id.clone();
                                                        chat.provider_id = provider;
                                                        chat.model = model;
                                                    }
                                                });
                                                persist();
                                                toast(i18n::t("chat.assistantSwitched"));
                                            } else {
                                                app::switch_assistant(&item.id);
                                            }
                                        },
                                        Avatar { name: item.display_name(), color: item.color(), size: 40 }
                                        div { class: "assistant-row-text",
                                            span { "{item.display_name()}" }
                                            small {
                                                if item.system_prompt.is_empty() { {i18n::t("chat.noSystemPrompt")} } else { {i18n::t("chat.customSystemPrompt")} }
                                            }
                                        }
                                        if selected {
                                            Icon { icon: LdCheck, width: 18, height: 18, fill: "currentColor" }
                                        }
                                    }
                                }
                            }
                        }
                        button {
                            class: "sheet-row",
                            onclick: move |_| {
                                picker_open.set(false);
                                navigate(Screen::AssistantEdit(Some(assistant_edit_id.clone())));
                            },
                            div { class: "row-icon",
                                Icon { icon: LdSettings, width: 18, height: 18, fill: "currentColor" }
                            }
                            div { class: "sheet-row-text", span { {i18n::t("chat.editAssistant")} } }
                        }
                    }
                }
            }

            if prompt_picker_open() {
                Sheet { title: i18n::t("chat.insertPrompt"), on_close: move |_| prompt_picker_open.set(false),
                    div { class: "sheet-list",
                        for prompt in data.prompts.iter() {
                            {
                                let prompt = prompt.clone();
                                rsx! {
                                    button {
                                        key: "{prompt.id}",
                                        class: "assistant-row",
                                        onclick: move |_| {
                                            prompt_picker_open.set(false);
                                            text.set(prompt.content.clone());
                                            if let Ok(literal) = serde_json::to_string(&prompt.content) {
                                                let _ = document::eval(&format!(
                                                    "const el=document.querySelector('.composer-input'); if(el){{el.value={literal};el.focus();}}"
                                                ));
                                            }
                                        },
                                        div { class: "row-icon primary",
                                            Icon { icon: LdBookOpen, width: 18, height: 18, fill: "currentColor" }
                                        }
                                        div { class: "assistant-row-text",
                                            span { "{prompt.name}" }
                                            small { "{prompt.content}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if rename_open() {
                div { class: "overlay center", onclick: move |_| rename_open.set(false),
                    div { class: "modal", onclick: move |event| event.stop_propagation(),
                        h2 { {i18n::t("chat.rename")} }
                        input {
                            class: "input",
                            initial_value: "{rename_text}",
                            oninput: move |event| rename_text.set(event.value()),
                        }
                        div { class: "modal-actions",
                            button { class: "btn ghost", onclick: move |_| rename_open.set(false), {i18n::t("common.cancel")} }
                            button {
                                class: "btn",
                                onclick: move |_| {
                                    let value = rename_text().trim().to_string();
                                    if !value.is_empty() {
                                        if let Some(id) = chat_state() {
                                            mutate(|data| {
                                                if let Some(chat) = data.chat_mut(&id) {
                                                    chat.title = value;
                                                }
                                            });
                                            persist();
                                        }
                                    }
                                    rename_open.set(false);
                                },
                                {i18n::t("common.save")}
                            }
                        }
                    }
                }
            }

            if prompt_open() {
                div { class: "overlay center", onclick: move |_| prompt_open.set(false),
                    div { class: "modal", onclick: move |event| event.stop_propagation(),
                        h2 { {i18n::t("chat.systemPrompt")} }
                        p { class: "muted",
                            {i18n::t("chat.systemPromptNote")}
                        }
                        textarea {
                            class: "input prompt-input",
                            rows: "7",
                            initial_value: "{prompt_text}",
                            oninput: move |event| prompt_text.set(event.value()),
                        }
                        div { class: "modal-actions",
                            button { class: "btn ghost", onclick: move |_| prompt_open.set(false), {i18n::t("common.cancel")} }
                            button {
                                class: "btn",
                                onclick: move |_| {
                                    let value = prompt_text().trim().to_string();
                                    let mut edited = assistant_for_prompt.clone();
                                    edited.system_prompt = value;
                                    app::save_assistant(edited);
                                    prompt_open.set(false);
                                    toast(i18n::t("chat.systemPromptSaved"));
                                },
                                {i18n::t("common.save")}
                            }
                        }
                    }
                }
            }

            if clear_open() {
                ConfirmDialog {
                    title: i18n::t("chat.clearTitle"),
                    message: i18n::t("chat.clearMessage"),
                    confirm_label: i18n::t("chat.clearConfirm"),
                    on_cancel: move |_| clear_open.set(false),
                    on_confirm: move |_| {
                        clear_open.set(false);
                        if let Some(id) = chat_state() {
                            mutate(|data| {
                                if let Some(chat) = data.chat_mut(&id) {
                                    chat.messages.clear();
                                    chat.title = "New Chat".into();
                                    chat.updated_at = store::now_ms();
                                }
                            });
                            persist();
                            set_chat_error(None);
                            toast(i18n::t("chat.cleared"));
                        }
                    },
                }
            }
        }
    }
}

#[component]
fn MessageView(
    message: ChatMessage,
    chat_id: Option<String>,
    assistant: Assistant,
    model: String,
    provider_name: String,
    user_name: String,
    user_color: String,
    show_bubble: bool,
    bubble_opacity: f32,
    show_tokens: bool,
    show_model_name: bool,
    streaming: bool,
    last_assistant: bool,
) -> Element {
    let is_user = message.is_user();
    let html = markdown::to_html(&message.content);
    let reasoning = message.reasoning.clone().unwrap_or_default();
    let assistant_label = if assistant.use_assistant_avatar {
        assistant.display_name()
    } else {
        model.clone()
    };
    let assistant_color_value = if assistant.use_assistant_avatar {
        assistant.color()
    } else {
        assistant_color(&model).to_string()
    };
    let opacity = (bubble_opacity.clamp(0.2, 1.0) * 100.0) as i32;
    let message_id = message.id.clone();
    let chat_id_init = chat_id.clone();
    let chat_id_state = use_signal(move || chat_id_init);
    let message_id_init = message_id.clone();
    let message_id_state = use_signal(move || message_id_init);
    let copy_one = message.content.clone();

    if message.role == "summary" {
        let summary_html = markdown::to_html(&message.content);
        return rsx! {
            div { class: "summary-card",
                div { class: "summary-head",
                    div { class: "row-icon tertiary",
                        Icon { icon: LdLayers, width: 16, height: 16, fill: "currentColor" }
                    }
                    span { {i18n::t("chat.checkpoint")} }
                    button {
                        class: "action-btn",
                        onclick: move |_| copy_text(&message.content),
                        Icon { icon: LdCopy, width: 14, height: 14, fill: "currentColor" }
                    }
                }
                div { class: "md summary-body", dangerous_inner_html: summary_html }
            }
        };
    }

    rsx! {
        div { class: if is_user { "msg user" } else { "msg assistant" },
            div { class: "msg-head",
                if !is_user {
                    Avatar { name: assistant_label.clone(), color: assistant_color_value.clone(), size: 28 }
                    div { class: "msg-head-text",
                        span { "{assistant_label}" }
                        if show_model_name {
                            small { "{provider_name}" }
                        }
                    }
                } else {
                    div { class: "msg-head-text right",
                        span { "{user_name}" }
                    }
                    Avatar { name: user_name.clone(), color: user_color.clone(), size: 28 }
                }
            }

            if !is_user && !reasoning.is_empty() {
                details { class: "cot", open: streaming,
                    summary {
                        div { class: "cot-title",
                            Icon { icon: LdBrain, width: 15, height: 15, fill: "currentColor" }
                            if streaming { {i18n::t("chat.thinking")} } else { {i18n::t("chat.thoughtProcess")} }
                        }
                        Icon { icon: LdChevronDown, width: 15, height: 15, fill: "currentColor" }
                    }
                    div { class: "cot-body", "{reasoning}" }
                }
            }

            if is_user && !message.attachments.is_empty() {
                div { class: "attachments",
                    for attachment in message.attachments.iter() {
                        if attachment.mime.starts_with("image/") && !attachment.data_url.is_empty() {
                            img {
                                key: "{attachment.id}",
                                class: "attachment-img",
                                src: "{attachment.data_url}",
                                alt: "{attachment.name}",
                            }
                        } else {
                            div { key: "{attachment.id}", class: "attachment-chip",
                                Icon { icon: LdFileText, width: 14, height: 14, fill: "currentColor" }
                                span { "{attachment.name}" }
                            }
                        }
                    }
                }
            }

            if is_user {
                div {
                    class: "user-bubble",
                    style: "background: color-mix(in srgb, var(--primary-container) {opacity}%, transparent);",
                    div { class: "md", dangerous_inner_html: html }
                }
            } else if show_bubble {
                div {
                    class: "assistant-bubble",
                    style: "background: color-mix(in srgb, var(--surface-container-high) {opacity}%, transparent);",
                    if streaming && message.content.is_empty() && reasoning.is_empty() {
                        div { class: "typing", span {} span {} span {} }
                    } else if message.content.is_empty() {
                        div { class: "muted", {i18n::t("chat.emptyResponse")} }
                    } else {
                        div { class: "md", dangerous_inner_html: html }
                        if streaming { span { class: "cursor" } }
                    }
                }
            } else {
                if streaming && message.content.is_empty() && reasoning.is_empty() {
                    div { class: "typing", span {} span {} span {} }
                } else if message.content.is_empty() {
                    div { class: "muted", {i18n::t("chat.emptyResponse")} }
                } else {
                    div { class: "md", dangerous_inner_html: html }
                    if streaming { span { class: "cursor" } }
                }
            }

            if !streaming {
                div { class: "msg-actions",
                    button {
                        class: "action-btn",
                        title: i18n::t("chat.copy"),
                        onclick: move |_| copy_text(&copy_one),
                        Icon { icon: LdCopy, width: 15, height: 15, fill: "currentColor" }
                    }
                    if !is_user && last_assistant {
                        button {
                            class: "action-btn",
                            title: i18n::t("chat.regenerate"),
                            onclick: move |_| {
                                if let Some(id) = chat_id_state() {
                                    regenerate(&id);
                                }
                            },
                            Icon { icon: LdRefreshCw, width: 15, height: 15, fill: "currentColor" }
                        }
                    }
                    button {
                        class: if message.favorite { "action-btn favorite" } else { "action-btn" },
                        title: i18n::t("chat.favorite"),
                        onclick: move |_| {
                            if let Some(id) = chat_id_state() {
                                toggle_favorite(&id, &message_id_state());
                            }
                        },
                        Icon { icon: LdStar, width: 15, height: 15, fill: "currentColor" }
                    }
                    ConfirmButton {
                        class: "action-btn confirm-icon",
                        title: i18n::t("common.delete"),
                        onconfirm: move |_| {
                            if let Some(id) = chat_id_state() {
                                delete_message(&id, &message_id_state());
                            }
                        },
                        Icon { icon: LdTrash2, width: 15, height: 15, fill: "currentColor" }
                    }
                    if show_tokens {
                        span { class: "msg-meta",
                            "{clock(message.created_at)}"
                            if !is_user && message.output_tokens > 0 {
                                " · "
                                {i18n::ta("chat.tokens", &[("count", &format_tokens(message.input_tokens + message.output_tokens))])}
                            }
                        }
                    }
                }
            }

        }
    }
}
