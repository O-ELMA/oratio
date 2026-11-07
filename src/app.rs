use dioxus::prelude::*;
use oratio::{
    api::{self, ApiMessage, Delta},
    i18n,
    store::{
        self, AppData, Assistant, Attachment, ChatMessage, Conversation, Provider, UsageEntry,
    },
};
use serde_json::Value;
use std::collections::HashSet;
use std::time::Duration;

use crate::speech::{self, SpeechEvent};

#[derive(Clone, PartialEq, Debug)]
pub enum Screen {
    Chat(Option<String>),
    History,
    Search,
    Assistants,
    AssistantEdit(Option<String>),
    Favorites,
    Stats,
    Settings,
    Theme,
    Preferences,
    Providers,
    ProviderEdit(Option<String>),
    Models,
    About,
    Extensions,
    WebSearch,
    Mcp,
    Skills,
    Prompts,
}

#[derive(Clone, PartialEq, Debug)]
pub struct StreamState {
    pub chat_id: String,
    pub running: bool,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct Draft {
    pub assistant_id: String,
    pub provider_id: String,
    pub model: String,
}

#[derive(Clone, PartialEq, Debug)]
pub enum VoiceModel {
    Missing,
    Downloading { received: u64, total: u64 },
    Ready,
    Failed,
}

impl Default for VoiceModel {
    fn default() -> Self {
        Self::Missing
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
pub enum VoicePhase {
    #[default]
    Idle,
    Recording,
    Transcribing,
    Permission,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct SpeechState {
    pub model: VoiceModel,
    pub phase: VoicePhase,
}

pub static DATA: GlobalSignal<AppData> = Signal::global(store::load);
pub static NAV: GlobalSignal<Vec<Screen>> = Signal::global(|| vec![Screen::Chat(None)]);
pub static DRAWER: GlobalSignal<bool> = Signal::global(|| false);
pub static STREAM: GlobalSignal<Option<StreamState>> = Signal::global(|| None);
pub static CHAT_ERROR: GlobalSignal<Option<(String, String)>> = Signal::global(|| None);
pub static TOAST: GlobalSignal<Option<String>> = Signal::global(|| None);
pub static DRAFT: GlobalSignal<Draft> = Signal::global(Draft::default);
pub static SYSTEM_DARK: GlobalSignal<bool> = Signal::global(|| false);
pub static STATUS: GlobalSignal<Option<String>> = Signal::global(|| None);
pub static PENDING: GlobalSignal<Vec<Attachment>> = Signal::global(Vec::new);
pub static SPEECH: GlobalSignal<SpeechState> = Signal::global(SpeechState::default);

fn set_nav(nav: Vec<Screen>) {
    let mut signal = NAV.signal();
    signal.set(nav);
}

pub fn navigate(screen: Screen) {
    let mut nav = NAV.peek().clone();
    nav.push(screen);
    set_nav(nav);
    close_drawer();
}

pub fn root(screen: Screen) {
    set_nav(vec![screen]);
    close_drawer();
}

pub fn open_chat(chat_id: String) {
    clear_attachments();
    root(Screen::Chat(Some(chat_id)));
}

pub fn back() {
    if *DRAWER.peek() {
        close_drawer();
        return;
    }
    let mut nav = NAV.peek().clone();
    if nav.len() > 1 {
        nav.pop();
        set_nav(nav);
    } else {
        store::finish_activity();
    }
}

pub fn open_drawer() {
    let mut signal = DRAWER.signal();
    signal.set(true);
}

pub fn close_drawer() {
    let mut signal = DRAWER.signal();
    signal.set(false);
}

pub fn set_stream(state: Option<StreamState>) {
    let mut signal = STREAM.signal();
    signal.set(state);
}

pub fn set_chat_error(value: Option<(String, String)>) {
    let mut signal = CHAT_ERROR.signal();
    signal.set(value);
}

pub fn set_system_dark(value: bool) {
    let mut signal = SYSTEM_DARK.signal();
    signal.set(value);
}

pub fn add_attachment(attachment: Attachment) {
    let mut signal = PENDING.signal();
    signal.write().push(attachment);
}

pub fn remove_attachment(id: &str) {
    let mut signal = PENDING.signal();
    signal.write().retain(|attachment| attachment.id != id);
}

pub fn clear_attachments() {
    let mut signal = PENDING.signal();
    signal.set(Vec::new());
}

pub fn set_status(value: Option<String>) {
    let mut signal = STATUS.signal();
    signal.set(value);
}

pub fn set_draft(value: Draft) {
    let mut signal = DRAFT.signal();
    signal.set(value);
}

pub fn mutate(update: impl FnOnce(&mut AppData)) {
    let mut data = DATA.signal();
    update(&mut data.write());
}

pub fn persist() {
    let snapshot: AppData = DATA.peek().clone();
    store::save(&snapshot);
}

pub fn toast(message: impl Into<String>) {
    let message = message.into();
    let mut signal = TOAST.signal();
    signal.set(Some(message.clone()));
    dioxus::core::spawn_forever(async move {
        tokio::time::sleep(std::time::Duration::from_millis(2600)).await;
        let same = TOAST.peek().as_deref() == Some(message.as_str());
        if same {
            let mut signal = TOAST.signal();
            signal.set(None);
        }
    });
}

fn set_voice_model(model: VoiceModel) {
    let mut signal = SPEECH.signal();
    signal.write().model = model;
}

fn set_voice_phase(phase: VoicePhase) {
    let mut signal = SPEECH.signal();
    signal.write().phase = phase;
}

pub fn start_voice_download() {
    if matches!(SPEECH.peek().model.clone(), VoiceModel::Downloading { .. }) {
        return;
    }
    if speech::model_ready() {
        set_voice_model(VoiceModel::Ready);
        return;
    }
    if !speech::supported() {
        return;
    }
    set_voice_model(VoiceModel::Downloading {
        received: 0,
        total: speech::MODEL_BYTES,
    });
    dioxus::core::spawn_forever(async move {
        let mut last_percent = 0u64;
        let result = speech::download_model(|received, total| {
            let percent = if total == 0 {
                0
            } else {
                received * 100 / total
            };
            if percent != last_percent {
                last_percent = percent;
                set_voice_model(VoiceModel::Downloading { received, total });
            }
        })
        .await;
        match result {
            Ok(()) => set_voice_model(VoiceModel::Ready),
            Err(error) => {
                log::warn!("voice model download failed: {error}");
                set_voice_model(VoiceModel::Failed);
                toast(i18n::t("chat.voiceDownloadFailed"));
            }
        }
    });
}

pub fn toggle_dictation() {
    let phase = SPEECH.peek().phase.clone();
    match phase {
        VoicePhase::Recording => stop_dictation(),
        VoicePhase::Transcribing => cancel_dictation(),
        VoicePhase::Idle => begin_dictation(),
        VoicePhase::Permission => {}
    }
}

fn begin_dictation() {
    if !matches!(SPEECH.peek().model.clone(), VoiceModel::Ready) {
        start_voice_download();
        return;
    }
    let permitted = speech::has_permission();
    log::info!("oratio voice: microphone permission={permitted}");
    if !permitted {
        set_voice_phase(VoicePhase::Permission);
        speech::request_permission();
        poll_permission();
        return;
    }
    match speech::start(&i18n::lang()) {
        Ok(()) => {}
        Err(error) if error == "busy" => {}
        Err(error) => {
            log::warn!("voice start failed: {error}");
            set_voice_phase(VoicePhase::Idle);
            toast(i18n::t("chat.voiceError"));
        }
    }
}

fn poll_permission() {
    dioxus::core::spawn_forever(async move {
        for _ in 0..100 {
            tokio::time::sleep(Duration::from_millis(300)).await;
            if !matches!(SPEECH.peek().phase.clone(), VoicePhase::Permission) {
                return;
            }
            if speech::has_permission() {
                set_voice_phase(VoicePhase::Idle);
                begin_dictation();
                return;
            }
        }
        set_voice_phase(VoicePhase::Idle);
    });
}

pub fn stop_dictation() {
    if matches!(SPEECH.peek().phase.clone(), VoicePhase::Recording) {
        speech::stop();
        set_voice_phase(VoicePhase::Transcribing);
    }
}

pub fn cancel_dictation() {
    let phase = SPEECH.peek().phase.clone();
    match phase {
        VoicePhase::Recording | VoicePhase::Transcribing => {
            speech::cancel();
            set_voice_phase(VoicePhase::Idle);
        }
        VoicePhase::Permission => set_voice_phase(VoicePhase::Idle),
        VoicePhase::Idle => {}
    }
}

fn insert_transcript(text: &str) {
    let Ok(literal) = serde_json::to_string(text) else {
        return;
    };
    let _ = document::eval(&format!(
        r#"
        const el = document.querySelector('.composer-input');
        if (el) {{
            el.disabled = false;
            const base = el.value;
            const spacer = base && !/\s$/.test(base) ? ' ' : '';
            el.value = base + spacer + {literal};
            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
            el.focus();
        }}
        "#
    ));
}

fn install_voice_events() {
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    speech::set_event_sink(sender);
    dioxus::core::spawn_forever(async move {
        while let Some(event) = receiver.recv().await {
            match event {
                SpeechEvent::Recording => set_voice_phase(VoicePhase::Recording),
                SpeechEvent::Transcribing => set_voice_phase(VoicePhase::Transcribing),
                SpeechEvent::Transcript(text) => {
                    if matches!(SPEECH.peek().phase.clone(), VoicePhase::Transcribing)
                        && !text.trim().is_empty()
                    {
                        insert_transcript(&text);
                    }
                    set_voice_phase(VoicePhase::Idle);
                }
                SpeechEvent::Error(error) => {
                    log::warn!("voice: {error}");
                    set_voice_phase(VoicePhase::Idle);
                    toast(format!("{}: {error}", i18n::t("chat.voiceError")));
                }
                SpeechEvent::Idle => set_voice_phase(VoicePhase::Idle),
            }
        }
    });
}

pub fn sync_draft(assistant_id: &str) {
    let selected = if assistant_id.trim().is_empty() {
        DATA.peek().settings.assistant_id.clone()
    } else {
        assistant_id.to_string()
    };
    let (provider_id, model) = {
        let data = DATA.peek();
        let assistant = data
            .assistant(&selected)
            .cloned()
            .unwrap_or_else(|| data.current_assistant());
        data.model_for_assistant(&assistant)
    };
    set_draft(Draft {
        assistant_id: selected,
        provider_id,
        model,
    });
}

pub fn new_chat() {
    clear_attachments();
    let assistant_id = DATA.peek().settings.assistant_id.clone();
    sync_draft(&assistant_id);
    root(Screen::Chat(None));
}

pub fn fork_chat(chat_id: &str) {
    if is_streaming(chat_id) {
        stop_generation();
    }
    let Some(mut fork) = DATA.peek().chat(chat_id).cloned() else {
        return;
    };
    let now = store::now_ms();
    let base_title = fork.display_title();
    fork.id = store::new_id();
    fork.title = i18n::ta("chat.forkTitle", &[("title", &base_title)]);
    fork.created_at = now;
    fork.updated_at = now;
    fork.pinned = false;
    for message in &mut fork.messages {
        message.id = store::new_id();
    }
    let fork_id = fork.id.clone();
    mutate(|data| data.chats.insert(0, fork));
    persist();
    open_chat(fork_id);
}

pub fn switch_assistant(assistant_id: &str) {
    mutate(|data| {
        data.settings.assistant_id = assistant_id.to_string();
    });
    persist();
    sync_draft(assistant_id);
    root(Screen::Chat(None));
}

pub fn is_streaming(chat_id: &str) -> bool {
    STREAM
        .peek()
        .as_ref()
        .map(|state| state.chat_id == chat_id && state.running)
        .unwrap_or(false)
}

pub fn stop_generation() {
    let mut state = STREAM.peek().clone();
    if let Some(state) = state.as_mut() {
        state.running = false;
        set_stream(Some(state.clone()));
    }
}

pub fn send_message(chat_id: Option<String>, text: String) -> Option<String> {
    cancel_dictation();
    let text = text.trim().to_string();
    let attachments = PENDING.peek().clone();
    if text.is_empty() && attachments.is_empty() {
        return chat_id;
    }

    let id = match chat_id {
        Some(id) => id,
        None => {
            let draft = DRAFT.peek().clone();
            let (provider_id, model) = {
                let data = DATA.peek();
                if data.provider(&draft.provider_id).is_none() || draft.model.is_empty() {
                    toast(i18n::t("chat.addProviderAndModel"));
                    navigate(Screen::Providers);
                    return None;
                }
                (draft.provider_id.clone(), draft.model.clone())
            };
            let now = store::now_ms();
            let chat = Conversation {
                id: store::new_id(),
                title: if text.is_empty() {
                    "Attachment".into()
                } else {
                    store::title_for(&text)
                },
                assistant_id: draft.assistant_id,
                provider_id,
                model,
                messages: Vec::new(),
                created_at: now,
                updated_at: now,
                pinned: false,
            };
            let chat_id = chat.id.clone();
            mutate(|data| data.chats.insert(0, chat));
            persist();
            root(Screen::Chat(Some(chat_id.clone())));
            chat_id
        }
    };

    let (provider_id, model) = {
        let data = DATA.peek();
        let Some(chat) = data.chat(&id) else {
            return Some(id);
        };
        if data.provider(&chat.provider_id).is_none() || chat.model.is_empty() {
            set_chat_error(Some((id.clone(), i18n::t("chat.noUsableModel"))));
            return Some(id);
        }
        (chat.provider_id.clone(), chat.model.clone())
    };
    let _ = (provider_id, model);

    let title = if text.is_empty() {
        "Attachment".to_string()
    } else {
        store::title_for(&text)
    };
    mutate(|data| {
        if let Some(chat) = data.chat_mut(&id) {
            if chat.messages.is_empty() {
                chat.title = title;
            }
            let mut message = ChatMessage::user(&text);
            message.attachments = attachments.clone();
            chat.messages.push(message);
            chat.messages.push(ChatMessage::assistant());
            chat.updated_at = store::now_ms();
        }
    });
    clear_attachments();
    persist();
    set_chat_error(None);
    set_stream(Some(StreamState {
        chat_id: id.clone(),
        running: true,
    }));
    let generation_id = id.clone();
    dioxus::core::spawn_forever(async move { generate(generation_id).await });
    Some(id)
}

pub fn regenerate(chat_id: &str) {
    {
        let data = DATA.peek();
        if data.chat(chat_id).is_none() {
            return;
        }
    }
    if is_streaming(chat_id) {
        stop_generation();
    }
    mutate(|data| {
        if let Some(chat) = data.chat_mut(chat_id) {
            while chat
                .messages
                .last()
                .map(|message| message.role == "assistant")
                .unwrap_or(false)
            {
                chat.messages.pop();
            }
            chat.messages.push(ChatMessage::assistant());
            chat.updated_at = store::now_ms();
        }
    });
    persist();
    set_chat_error(None);
    set_stream(Some(StreamState {
        chat_id: chat_id.to_string(),
        running: true,
    }));
    let generation_id = chat_id.to_string();
    dioxus::core::spawn_forever(async move { generate(generation_id).await });
}

#[derive(Clone)]
struct McpRoute {
    api_name: String,
    server_name: String,
    server_url: String,
    server_key: String,
    tool_name: String,
    description: String,
    schema: Value,
}

fn sanitize_tool_part(value: &str) -> String {
    let mut text: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                character
            } else {
                '_'
            }
        })
        .collect();
    if text.is_empty() {
        text = "tool".into();
    }
    text
}

fn mcp_routes(data: &AppData) -> Vec<McpRoute> {
    let mut used = HashSet::new();
    let mut routes = Vec::new();
    for server in data.mcp_servers.iter().filter(|server| server.enabled) {
        for tool in &server.tools {
            if tool.name.trim().is_empty() {
                continue;
            }
            let mut base = format!(
                "mcp_{}_{}",
                sanitize_tool_part(&server.name),
                sanitize_tool_part(&tool.name)
            );
            if base.chars().count() > 56 {
                base = base.chars().take(56).collect();
            }
            let mut api_name = base.clone();
            let mut counter = 2;
            while used.contains(&api_name) {
                api_name = format!("{base}_{counter}");
                counter += 1;
            }
            used.insert(api_name.clone());
            routes.push(McpRoute {
                api_name,
                server_name: server.name.clone(),
                server_url: server.url.clone(),
                server_key: server.api_key.clone(),
                tool_name: tool.name.clone(),
                description: format!(
                    "{} (MCP server: {})",
                    if tool.description.trim().is_empty() {
                        tool.name.as_str()
                    } else {
                        tool.description.trim()
                    },
                    server.name
                ),
                schema: tool.display_schema(),
            });
        }
    }
    routes
}

fn tools_payload(routes: &[McpRoute]) -> Vec<Value> {
    routes
        .iter()
        .map(|route| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": route.api_name,
                    "description": route.description,
                    "parameters": route.schema,
                }
            })
        })
        .collect()
}

fn append_reasoning(chat_id: &str, index: usize, text: &str) {
    let mut data = DATA.signal();
    let mut guard = data.write();
    if let Some(chat) = guard.chat_mut(chat_id) {
        if let Some(message) = chat.messages.get_mut(index) {
            message
                .reasoning
                .get_or_insert_with(String::new)
                .push_str(text);
        }
    }
}

async fn generate(chat_id: String) {
    let prepared = {
        let data = DATA.peek();
        data.chat(&chat_id).and_then(|chat| {
            let provider = data.provider(&chat.provider_id)?;
            let assistant = data.assistant_for_chat(chat);
            let mut messages = Vec::new();
            let mut system = assistant.system_prompt.trim().to_string();
            for skill_id in &assistant.skill_ids {
                if let Some(skill) = data.skills.iter().find(|skill| &skill.id == skill_id) {
                    if !skill.content.trim().is_empty() {
                        system.push_str(&format!(
                            "\n\n# Skill: {}\n{}",
                            skill.name,
                            skill.content.trim()
                        ));
                    }
                }
            }
            let system = system.trim().to_string();
            if !system.is_empty() {
                messages.push(ApiMessage {
                    role: "system".into(),
                    content: system,
                    images: Vec::new(),
                    tool_calls: Vec::new(),
                    tool_call_id: None,
                });
            }
            for message in &chat.messages {
                if message.content.is_empty() && message.attachments.is_empty() {
                    continue;
                }
                if message.role == "summary" {
                    messages.push(ApiMessage {
                        role: "system".into(),
                        content: format!("Summary of earlier conversation:\n{}", message.content),
                        images: Vec::new(),
                        tool_calls: Vec::new(),
                        tool_call_id: None,
                    });
                    continue;
                }
                if message.role == "user" || message.role == "assistant" {
                    let mut content = message.content.clone();
                    let mut images = Vec::new();
                    for attachment in &message.attachments {
                        if attachment.mime.starts_with("image/") && !attachment.data_url.is_empty()
                        {
                            images.push(attachment.data_url.clone());
                        } else if let Some(text) = &attachment.text {
                            content.push_str(&format!(
                                "\n\n[Attached file: {}]\n{}",
                                attachment.name,
                                text.chars().take(20000).collect::<String>()
                            ));
                        }
                    }
                    messages.push(ApiMessage {
                        role: message.role.clone(),
                        content,
                        images,
                        tool_calls: Vec::new(),
                        tool_call_id: None,
                    });
                }
            }
            let input_tokens: u32 = messages
                .iter()
                .map(|m| store::estimate_tokens(&m.content))
                .sum();
            Some((
                provider.base_url.clone(),
                provider.api_key.clone(),
                provider.id.clone(),
                chat.model.clone(),
                messages,
                input_tokens,
                chat.messages.len().saturating_sub(1),
            ))
        })
    };

    let Some((base_url, api_key, provider_id, model, mut messages, input_tokens, placeholder)) =
        prepared
    else {
        set_stream(None);
        set_chat_error(Some((chat_id, i18n::t("chat.providerDeleted"))));
        return;
    };

    let search_enabled = {
        let data = DATA.peek();
        data.chat(&chat_id)
            .map(|chat| data.assistant_for_chat(chat).enable_web_search)
            .unwrap_or(false)
    };
    if search_enabled {
        let settings = DATA.peek().settings.clone();
        if settings.search_provider != "none" && !settings.search_api_key.trim().is_empty() {
            if let Some(query) = messages
                .iter()
                .rev()
                .find(|message| message.role == "user")
                .map(|message| message.content.clone())
            {
                set_status(Some(i18n::t("app.searchingWeb")));
                match api::web_search(&settings.search_provider, &settings.search_api_key, &query)
                    .await
                {
                    Ok(results) if !results.is_empty() => {
                        let mut text = String::from(
                            "Live web search results for the user's latest message. Use them when relevant and cite the URLs.\n",
                        );
                        for (index, result) in results.iter().enumerate() {
                            text.push_str(&format!(
                                "\n{}. {}\n{}\n{}",
                                index + 1,
                                result.title,
                                result.url,
                                result.snippet
                            ));
                        }
                        let position = if messages
                            .first()
                            .map(|m| m.role == "system")
                            .unwrap_or(false)
                        {
                            1
                        } else {
                            0
                        };
                        messages.insert(
                            position,
                            ApiMessage {
                                role: "system".into(),
                                content: text,
                                images: Vec::new(),
                                tool_calls: Vec::new(),
                                tool_call_id: None,
                            },
                        );
                    }
                    Ok(_) => {}
                    Err(error) => {
                        log::warn!("web search failed: {error}");
                    }
                }
                set_status(None);
            }
        }
    }

    let routes = {
        let data = DATA.peek();
        mcp_routes(&data)
    };
    let tool_payload = tools_payload(&routes);
    let mut generation_error: Option<String> = None;
    let mut round = 0;
    let mut input_tokens = input_tokens;
    loop {
        let delta_chat = chat_id.clone();
        let outcome = api::stream_chat_with_tools(
            &base_url,
            &api_key,
            &model,
            &messages,
            &tool_payload,
            move |delta| {
                if !is_streaming(&delta_chat) {
                    return false;
                }
                let mut data = DATA.signal();
                let mut guard = data.write();
                if let Some(chat) = guard.chat_mut(&delta_chat) {
                    if let Some(message) = chat.messages.get_mut(placeholder) {
                        match delta {
                            Delta::Content(text) => message.content.push_str(&text),
                            Delta::Reasoning(text) => message
                                .reasoning
                                .get_or_insert_with(String::new)
                                .push_str(&text),
                        }
                    }
                }
                true
            },
        )
        .await;

        let (content, mut calls) = match outcome {
            Ok(value) => value,
            Err(error) => {
                generation_error = Some(error);
                break;
            }
        };
        if calls.is_empty() || round >= 5 {
            break;
        }
        round += 1;
        if !is_streaming(&chat_id) {
            break;
        }
        for (index, call) in calls.iter_mut().enumerate() {
            if call.id.trim().is_empty() {
                call.id = format!("call_{round}_{index}");
            }
        }
        let mut tool_request_text = String::new();
        for call in &calls {
            tool_request_text
                .push_str(&format!("\n\nTool call: {}\n{}", call.name, call.arguments));
        }
        append_reasoning(&chat_id, placeholder, &tool_request_text);
        messages.push(ApiMessage {
            role: "assistant".into(),
            content: content.clone(),
            tool_calls: calls.clone(),
            ..Default::default()
        });
        input_tokens = input_tokens
            .saturating_add(store::estimate_tokens(&content))
            .saturating_add(
                calls
                    .iter()
                    .map(|call| store::estimate_tokens(&call.arguments))
                    .sum(),
            );
        for call in &calls {
            if !is_streaming(&chat_id) {
                break;
            }
            let result =
                if let Some(route) = routes.iter().find(|route| route.api_name == call.name) {
                    set_status(Some(i18n::ta(
                        "app.runningTool",
                        &[("tool", &route.tool_name)],
                    )));
                    let value = api::mcp_call_tool(
                        &route.server_url,
                        &route.server_key,
                        &route.tool_name,
                        &call.arguments,
                    )
                    .await
                    .unwrap_or_else(|error| format!("Tool failed: {error}"));
                    set_status(None);
                    value
                } else {
                    format!("Unknown MCP tool: {}", call.name)
                };
            append_reasoning(
                &chat_id,
                placeholder,
                &format!(
                    "\nTool result ({} from {}):\n{}",
                    call.name,
                    routes
                        .iter()
                        .find(|route| route.api_name == call.name)
                        .map(|route| route.server_name.as_str())
                        .unwrap_or("unknown server"),
                    result
                ),
            );
            input_tokens = input_tokens.saturating_add(store::estimate_tokens(&result));
            messages.push(ApiMessage {
                role: "tool".into(),
                content: result,
                tool_call_id: Some(call.id.clone()),
                ..Default::default()
            });
        }
    }
    set_status(None);
    let result: Result<(), String> = match generation_error {
        Some(error) => Err(error),
        None => Ok(()),
    };

    let stream_state = STREAM.peek().clone();
    let current = stream_state
        .as_ref()
        .map(|state| state.chat_id == chat_id)
        .unwrap_or(false);
    let cancelled = stream_state
        .as_ref()
        .map(|state| !state.running)
        .unwrap_or(true);
    let succeeded = result.is_ok();

    if let Err(error) = result {
        log::warn!("generation failed: {error}");
        mutate(|data| {
            if let Some(chat) = data.chat_mut(&chat_id) {
                let empty = chat
                    .messages
                    .get(placeholder)
                    .map(|message| message.content.is_empty())
                    .unwrap_or(false);
                if empty {
                    chat.messages.remove(placeholder);
                }
            }
        });
        if current {
            set_chat_error(Some((chat_id.clone(), error)));
        }
    }

    let (output_tokens, recorded) = {
        let mut data = DATA.signal();
        let mut guard = data.write();
        let mut out = 0;
        let mut recorded = None;
        if let Some(chat) = guard.chat_mut(&chat_id) {
            if let Some(message) = chat.messages.get_mut(placeholder) {
                out = store::estimate_tokens(&message.content);
                message.output_tokens = out;
                message.input_tokens = input_tokens;
                if out > 0 || message.reasoning.as_deref().map(str::is_empty) == Some(false) {
                    recorded = Some(UsageEntry {
                        at: store::now_ms(),
                        provider_id: provider_id.clone(),
                        model: model.clone(),
                        input_tokens,
                        output_tokens: out,
                    });
                }
            }
            let empty = chat
                .messages
                .get(placeholder)
                .map(|message| {
                    message.content.is_empty()
                        && message
                            .reasoning
                            .as_deref()
                            .map(str::is_empty)
                            .unwrap_or(true)
                })
                .unwrap_or(false);
            if empty {
                chat.messages.remove(placeholder);
            }
            chat.updated_at = store::now_ms();
        }
        (out, recorded)
    };
    let _ = output_tokens;
    if let Some(entry) = recorded {
        mutate(|data| data.usage.push(entry));
    }
    if succeeded && !cancelled && output_tokens == 0 {
        set_chat_error(Some((
            chat_id.clone(),
            i18n::t("chat.emptyProviderResponse"),
        )));
    }
    persist();
    if current {
        set_stream(None);
    }
}

pub fn switch_model(chat_id: Option<&str>, provider_id: &str, model: &str) {
    match chat_id {
        Some(id) => {
            mutate(|data| {
                if let Some(chat) = data.chat_mut(id) {
                    chat.provider_id = provider_id.to_string();
                    chat.model = model.to_string();
                    chat.updated_at = store::now_ms();
                }
            });
            persist();
            let has_error = CHAT_ERROR
                .peek()
                .as_ref()
                .map(|(error_id, _)| error_id == id)
                .unwrap_or(false);
            if has_error {
                set_chat_error(None);
            }
            if is_streaming(id) {
                stop_generation();
            }
        }
        None => {
            let mut draft = DRAFT.peek().clone();
            draft.provider_id = provider_id.to_string();
            draft.model = model.to_string();
            set_draft(draft);
        }
    }
    toast(i18n::t("app.modelSwitched"));
}

pub fn delete_chat(chat_id: &str) {
    mutate(|data| data.chats.retain(|chat| chat.id != chat_id));
    persist();
    let was_streaming = STREAM
        .peek()
        .as_ref()
        .map(|state| state.chat_id == chat_id)
        .unwrap_or(false);
    if was_streaming {
        set_stream(None);
    }
    set_chat_error(None);
    let is_root = matches!(NAV.peek().last(), Some(Screen::Chat(Some(id))) if id == chat_id);
    if is_root {
        new_chat();
    }
}

pub fn toggle_pin(chat_id: &str) {
    mutate(|data| {
        if let Some(chat) = data.chat_mut(chat_id) {
            chat.pinned = !chat.pinned;
        }
    });
    persist();
}

pub fn delete_message(chat_id: &str, message_id: &str) {
    if is_streaming(chat_id) {
        toast(i18n::t("app.stopBeforeEditing"));
        return;
    }
    mutate(|data| {
        if let Some(chat) = data.chat_mut(chat_id) {
            let index = chat
                .messages
                .iter()
                .position(|message| message.id == message_id);
            if let Some(index) = index {
                let is_user = chat.messages[index].role == "user";
                chat.messages.remove(index);
                if is_user
                    && chat
                        .messages
                        .get(index)
                        .map(|message| message.role == "assistant")
                        .unwrap_or(false)
                {
                    chat.messages.remove(index);
                }
            }
        }
    });
    persist();
}

pub fn toggle_favorite(chat_id: &str, message_id: &str) {
    mutate(|data| {
        if let Some(chat) = data.chat_mut(chat_id) {
            if let Some(message) = chat.messages.iter_mut().find(|m| m.id == message_id) {
                message.favorite = !message.favorite;
            }
        }
    });
    persist();
}

pub fn save_assistant(assistant: Assistant) {
    mutate(|data| {
        if let Some(existing) = data.assistant_mut(&assistant.id) {
            *existing = assistant.clone();
        } else {
            data.assistants.push(assistant.clone());
        }
    });
    persist();
}

pub fn delete_assistant(assistant_id: &str) {
    mutate(|data| {
        data.assistants
            .retain(|assistant| assistant.id != assistant_id);
        if data.assistants.is_empty() {
            data.assistants.push(Assistant::default());
        }
        if data.settings.assistant_id == assistant_id {
            data.settings.assistant_id = data.assistants[0].id.clone();
        }
    });
    persist();
    let selected = DATA.peek().settings.assistant_id.clone();
    sync_draft(&selected);
}

pub fn save_provider(provider: Provider) {
    mutate(|data| {
        if let Some(existing) = data.providers.iter_mut().find(|p| p.id == provider.id) {
            *existing = provider.clone();
        } else {
            data.providers.push(provider.clone());
        }
        if data.settings.default_model.is_empty() {
            if let Some(model) = provider.models.first() {
                data.settings.default_provider_id = provider.id.clone();
                data.settings.default_model = model.clone();
            }
        }
    });
    persist();
}

pub fn delete_provider(provider_id: &str) {
    if store::is_builtin_provider(provider_id) {
        return;
    }
    mutate(|data| {
        data.providers.retain(|provider| provider.id != provider_id);
        if data.settings.default_provider_id == provider_id {
            data.settings.default_provider_id = String::new();
            data.settings.default_model = String::new();
        }
    });
    persist();
}

pub fn delete_all_chats() {
    mutate(|data| data.chats.clear());
    persist();
    set_stream(None);
    set_chat_error(None);
    new_chat();
}

fn install_theme_listener() {
    dioxus::core::spawn_forever(async move {
        let mut eval = document::eval(
            r#"
            const media = window.matchMedia('(prefers-color-scheme: dark)');
            dioxus.send(media.matches);
            media.addEventListener('change', (event) => dioxus.send(event.matches));
            "#,
        );
        while let Ok(dark) = eval.recv::<bool>().await {
            set_system_dark(dark);
        }
    });
}

fn install_back_guard() {
    dioxus::core::spawn_forever(async move {
        let mut eval = document::eval(
            r#"
            (function () {
                if (window.__oratioBack) return;
                window.__oratioBack = true;
                history.pushState({ oratio: 1 }, "");
                window.addEventListener("popstate", function () {
                    setTimeout(function () {
                        history.pushState({ oratio: 1 }, "");
                    }, 0);
                    dioxus.send("back");
                });
            })();
            "#,
        );
        while let Ok(message) = eval.recv::<String>().await {
            if message == "back" {
                back();
            }
        }
    });
}

fn install_link_handler() {
    dioxus::core::spawn_forever(async move {
        let mut eval = document::eval(
            r#"
            (function () {
                if (window.__oratioLinks) return;
                window.__oratioLinks = true;
                document.addEventListener('click', function (event) {
                    const link = event.target && event.target.closest ? event.target.closest('a') : null;
                    if (!link || !link.href) return;
                    event.preventDefault();
                    dioxus.send(link.href);
                });
            })();
            "#,
        );
        while let Ok(url) = eval.recv::<String>().await {
            store::open_url(&url);
        }
    });
}

fn install_overlay_watch() {
    dioxus::core::spawn_forever(async move {
        let mut eval = document::eval(
            r#"
            (function () {
                if (window.__oratioOverlayWatch) return;
                window.__oratioOverlayWatch = true;
                const sync = () => {
                    document.body.classList.toggle('overlay-open', Boolean(document.querySelector('.overlay')));
                };
                new MutationObserver(sync).observe(document.body, { childList: true, subtree: true });
                sync();
            })();
            "#,
        );
        let _ = eval.recv::<String>().await;
    });
}

fn boot() {
    i18n::set_language(&DATA.peek().settings.language);
    let assistant_id = DATA.peek().settings.assistant_id.clone();
    sync_draft(&assistant_id);
    install_theme_listener();
    install_back_guard();
    install_link_handler();
    install_overlay_watch();
    install_voice_events();
    if speech::supported() {
        if speech::model_ready() {
            set_voice_model(VoiceModel::Ready);
        } else {
            start_voice_download();
        }
    }
    log::info!("oratio data dir: {}", store::data_dir().display());
}

#[component]
pub fn App() -> Element {
    use_hook(boot);
    use_effect(move || {
        let language = i18n::lang();
        let direction = i18n::direction();
        let _ = document::eval(&format!(
            "document.documentElement.lang = {language:?}; document.documentElement.dir = {direction:?};"
        ));
    });
    let data = DATA();
    let system_dark = SYSTEM_DARK();
    let mode = match data.settings.color_mode.as_str() {
        "light" => "light",
        "dark" => "dark",
        _ => {
            if system_dark {
                "dark"
            } else {
                "light"
            }
        }
    };
    let theme = if data.settings.theme_id.trim().is_empty() {
        "sakura"
    } else {
        data.settings.theme_id.as_str()
    };
    let scale = data.settings.font_scale;
    let screen = NAV().last().cloned().unwrap_or(Screen::Chat(None));

    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        div {
            class: "app theme-{theme} mode-{mode}",
            style: "--font-scale: {scale};",
            {match screen {
                Screen::Chat(id) => rsx! { crate::ui::chat::ChatScreen { key: "{id:?}", chat_id: id } },
                Screen::History => rsx! { crate::ui::pages::HistoryPage {} },
                Screen::Search => rsx! { crate::ui::pages::SearchPage {} },
                Screen::Assistants => rsx! { crate::ui::assistants::AssistantsPage {} },
                Screen::AssistantEdit(id) => rsx! { crate::ui::assistants::AssistantEditPage { key: "{id:?}", assistant_id: id } },
                Screen::Favorites => rsx! { crate::ui::pages::FavoritesPage {} },
                Screen::Stats => rsx! { crate::ui::stats::StatsPage {} },
                Screen::Settings => rsx! { crate::ui::settings::SettingsPage {} },
                Screen::Theme => rsx! { crate::ui::settings::ThemePage {} },
                Screen::Preferences => rsx! { crate::ui::settings::PreferencesPage {} },
                Screen::Providers => rsx! { crate::ui::providers::ProvidersPage {} },
                Screen::ProviderEdit(id) => rsx! { crate::ui::providers::ProviderEditPage { key: "{id:?}", provider_id: id } },
                Screen::Models => rsx! { crate::ui::settings::ModelsPage {} },
                Screen::About => rsx! { crate::ui::settings::AboutPage {} },
                Screen::Extensions => rsx! { crate::ui::pages::ExtensionsPage {} },
                Screen::WebSearch => rsx! { crate::ui::extensions::WebSearchPage {} },
                Screen::Mcp => rsx! { crate::ui::extensions::McpPage {} },
                Screen::Skills => rsx! { crate::ui::extensions::SkillsPage {} },
                Screen::Prompts => rsx! { crate::ui::extensions::PromptsPage {} },
            }}
            crate::ui::common::Toast {}
        }
    }
}
