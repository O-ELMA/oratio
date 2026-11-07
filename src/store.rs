use rusqlite::{params, Connection};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        OnceLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

pub const DEFAULT_ASSISTANT_ID: &str = "assistant-default";
pub const DEFAULT_ASSISTANT_COLOR: &str = "#8E4955";

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(default)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub models: Vec<String>,
}

fn builtin_providers() -> Vec<Provider> {
    vec![
        Provider {
            id: "builtin-openai".into(),
            name: "OpenAI".into(),
            base_url: "https://api.openai.com/v1".into(),
            api_key: String::new(),
            models: vec!["gpt-4o".into(), "gpt-4o-mini".into()],
        },
        Provider {
            id: "builtin-anthropic".into(),
            name: "Anthropic".into(),
            base_url: "https://api.anthropic.com/v1".into(),
            api_key: String::new(),
            models: vec!["claude-sonnet-4-5".into(), "claude-haiku-4-5".into()],
        },
        Provider {
            id: "builtin-moonshot".into(),
            name: "Moonshot AI".into(),
            base_url: "https://api.moonshot.ai/v1".into(),
            api_key: String::new(),
            models: vec!["kimi-k2.5".into(), "moonshot-v1-8k".into()],
        },
        Provider {
            id: "builtin-openrouter".into(),
            name: "OpenRouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            api_key: String::new(),
            models: vec!["openrouter/auto".into()],
        },
        Provider {
            id: "builtin-deepseek".into(),
            name: "DeepSeek".into(),
            base_url: "https://api.deepseek.com/v1".into(),
            api_key: String::new(),
            models: vec!["deepseek-chat".into(), "deepseek-reasoner".into()],
        },
        Provider {
            id: "builtin-nanogpt".into(),
            name: "NanoGPT".into(),
            base_url: "https://api.nano-gpt.com/api/v1".into(),
            api_key: String::new(),
            models: Vec::new(),
        },
    ]
}

pub fn is_builtin_provider(id: &str) -> bool {
    id.starts_with("builtin-")
}

fn ensure_builtin_providers(data: &mut AppData) -> bool {
    let previous = data.providers.clone();
    let mut ordered = Vec::new();
    for preset in builtin_providers() {
        if let Some(existing) = data
            .providers
            .iter()
            .find(|provider| provider.id == preset.id)
        {
            ordered.push(existing.clone());
        } else {
            ordered.push(preset);
        }
    }
    for provider in &data.providers {
        if !ordered.iter().any(|item| item.id == provider.id) {
            ordered.push(provider.clone());
        }
    }
    let changed = ordered != previous;
    data.providers = ordered;
    changed
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct Assistant {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    pub provider_id: String,
    pub model: String,
    pub avatar_color: String,
    pub use_assistant_avatar: bool,
    pub use_gradient_background: bool,
    pub enable_web_search: bool,
    pub skill_ids: Vec<String>,
}

impl Default for Assistant {
    fn default() -> Self {
        Self {
            id: DEFAULT_ASSISTANT_ID.into(),
            name: String::new(),
            system_prompt: String::new(),
            provider_id: String::new(),
            model: String::new(),
            avatar_color: DEFAULT_ASSISTANT_COLOR.into(),
            use_assistant_avatar: false,
            use_gradient_background: false,
            enable_web_search: false,
            skill_ids: Vec::new(),
        }
    }
}

impl Assistant {
    pub fn display_name(&self) -> String {
        if self.name.trim().is_empty() {
            crate::i18n::t("common.defaultAssistant")
        } else {
            self.name.trim().to_string()
        }
    }

    pub fn initial(&self) -> String {
        self.display_name()
            .chars()
            .next()
            .unwrap_or('R')
            .to_uppercase()
            .to_string()
    }

    pub fn color(&self) -> String {
        if self.avatar_color.trim().is_empty() {
            DEFAULT_ASSISTANT_COLOR.into()
        } else {
            self.avatar_color.clone()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub reasoning: Option<String>,
    pub created_at: u64,
    pub favorite: bool,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub attachments: Vec<Attachment>,
}

impl Default for ChatMessage {
    fn default() -> Self {
        Self {
            id: String::new(),
            role: String::new(),
            content: String::new(),
            reasoning: None,
            created_at: 0,
            favorite: false,
            input_tokens: 0,
            output_tokens: 0,
            attachments: Vec::new(),
        }
    }
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            role: "user".into(),
            content: content.into(),
            created_at: now_ms(),
            ..Default::default()
        }
    }

    pub fn assistant() -> Self {
        Self {
            id: new_id(),
            role: "assistant".into(),
            created_at: now_ms(),
            ..Default::default()
        }
    }

    pub fn is_user(&self) -> bool {
        self.role == "user"
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub assistant_id: String,
    pub provider_id: String,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: u64,
    pub updated_at: u64,
    pub pinned: bool,
}

impl Default for Conversation {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: "New Chat".into(),
            assistant_id: DEFAULT_ASSISTANT_ID.into(),
            provider_id: String::new(),
            model: String::new(),
            messages: Vec::new(),
            created_at: 0,
            updated_at: 0,
            pinned: false,
        }
    }
}

impl Conversation {
    pub fn preview(&self) -> String {
        let Some(message) = self
            .messages
            .iter()
            .rev()
            .find(|message| !message.content.trim().is_empty())
        else {
            return String::new();
        };
        let line = message
            .content
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .unwrap_or("");
        let line = line.trim_start_matches(['#', '*', '`', '>', '-', ' ']);
        let mut text: String = line.chars().take(72).collect();
        if line.chars().count() > 72 {
            text.push('…');
        }
        text
    }

    pub fn display_title(&self) -> String {
        let title = self.title.trim();
        if title.is_empty() || title == "New Chat" {
            crate::i18n::t("common.newChat")
        } else if title == "Attachment" {
            crate::i18n::t("common.attachment")
        } else {
            title.to_string()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct Settings {
    pub system_prompt: String,
    pub assistant_id: String,
    pub default_provider_id: String,
    pub default_model: String,
    pub user_nickname: String,
    pub user_avatar_color: String,
    pub theme_id: String,
    pub color_mode: String,
    pub show_assistant_bubble: bool,
    pub bubble_opacity: f32,
    pub font_scale: f32,
    pub send_on_enter: bool,
    pub show_token_usage: bool,
    pub show_model_name: bool,
    pub search_provider: String,
    pub search_api_key: String,
    pub media_provider_id: String,
    pub media_model: String,
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            system_prompt: "You are a helpful assistant.".into(),
            assistant_id: DEFAULT_ASSISTANT_ID.into(),
            default_provider_id: String::new(),
            default_model: String::new(),
            user_nickname: String::new(),
            user_avatar_color: "#8E4955".into(),
            theme_id: "sakura".into(),
            color_mode: "system".into(),
            show_assistant_bubble: false,
            bubble_opacity: 1.0,
            font_scale: 1.0,
            send_on_enter: false,
            show_token_usage: true,
            show_model_name: true,
            search_provider: "none".into(),
            search_api_key: String::new(),
            media_provider_id: String::new(),
            media_model: String::new(),
            language: "en".into(),
        }
    }
}

impl Settings {
    pub fn user_name(&self) -> String {
        if self.user_nickname.trim().is_empty() {
            crate::i18n::t("common.user")
        } else {
            self.user_nickname.trim().to_string()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(default)]
pub struct UsageEntry {
    pub at: u64,
    pub provider_id: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(default)]
pub struct AppData {
    pub providers: Vec<Provider>,
    pub assistants: Vec<Assistant>,
    pub chats: Vec<Conversation>,
    pub settings: Settings,
    pub usage: Vec<UsageEntry>,
    pub prompts: Vec<Prompt>,
    pub skills: Vec<Skill>,
    pub mcp_servers: Vec<McpServer>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(default)]
pub struct Attachment {
    pub id: String,
    pub name: String,
    pub mime: String,
    pub data_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(default)]
pub struct Prompt {
    pub id: String,
    pub name: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(default)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(default)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

impl McpTool {
    pub fn display_schema(&self) -> serde_json::Value {
        if self.input_schema.is_null() || self.input_schema.as_object().is_none() {
            serde_json::json!({ "type": "object", "properties": {} })
        } else {
            self.input_schema.clone()
        }
    }
}

fn mcp_tools_from_values(values: Vec<serde_json::Value>) -> Vec<McpTool> {
    let mut tools = Vec::new();
    for value in values {
        if let Some(name) = value.as_str() {
            tools.push(McpTool {
                name: name.to_string(),
                ..Default::default()
            });
            continue;
        }
        let Some(object) = value.as_object() else {
            continue;
        };
        let name = object
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let description = object
            .get("description")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let input_schema = object
            .get("inputSchema")
            .or_else(|| object.get("input_schema"))
            .or_else(|| object.get("parameters"))
            .cloned()
            .unwrap_or_else(|| serde_json::json!({ "type": "object", "properties": {} }));
        tools.push(McpTool {
            name,
            description,
            input_schema,
        });
    }
    tools
}

pub fn parse_mcp_tools_json(text: &str) -> Vec<McpTool> {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|value| value.as_array().cloned())
        .map(mcp_tools_from_values)
        .unwrap_or_default()
}

fn deserialize_mcp_tools<'de, D>(deserializer: D) -> Result<Vec<McpTool>, D::Error>
where
    D: Deserializer<'de>,
{
    let values = Vec::<serde_json::Value>::deserialize(deserializer).unwrap_or_default();
    Ok(mcp_tools_from_values(values))
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(default)]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub url: String,
    pub api_key: String,
    pub enabled: bool,
    #[serde(deserialize_with = "deserialize_mcp_tools")]
    pub tools: Vec<McpTool>,
}

impl AppData {
    pub fn provider(&self, id: &str) -> Option<&Provider> {
        self.providers.iter().find(|provider| provider.id == id)
    }

    pub fn chat(&self, id: &str) -> Option<&Conversation> {
        self.chats.iter().find(|chat| chat.id == id)
    }

    pub fn chat_mut(&mut self, id: &str) -> Option<&mut Conversation> {
        self.chats.iter_mut().find(|chat| chat.id == id)
    }

    pub fn assistant(&self, id: &str) -> Option<&Assistant> {
        self.assistants.iter().find(|assistant| assistant.id == id)
    }

    pub fn assistant_mut(&mut self, id: &str) -> Option<&mut Assistant> {
        self.assistants
            .iter_mut()
            .find(|assistant| assistant.id == id)
    }

    pub fn current_assistant(&self) -> Assistant {
        self.assistant(&self.settings.assistant_id)
            .or_else(|| self.assistants.first())
            .cloned()
            .unwrap_or_default()
    }

    pub fn assistant_for_chat(&self, chat: &Conversation) -> Assistant {
        self.assistant(&chat.assistant_id)
            .or_else(|| self.assistant(&self.settings.assistant_id))
            .or_else(|| self.assistants.first())
            .cloned()
            .unwrap_or_default()
    }

    pub fn model_for_assistant(&self, assistant: &Assistant) -> (String, String) {
        let provider_id = if !assistant.provider_id.trim().is_empty() {
            assistant.provider_id.clone()
        } else {
            self.settings.default_provider_id.clone()
        };
        let model = if !assistant.model.trim().is_empty() {
            assistant.model.clone()
        } else {
            self.settings.default_model.clone()
        };
        if let Some(provider) = self.provider(&provider_id) {
            if !model.is_empty() && provider.models.iter().any(|m| m == &model) {
                return (provider_id, model);
            }
        }
        for provider in &self.providers {
            if let Some(model) = provider.models.first() {
                return (provider.id.clone(), model.clone());
            }
        }
        (provider_id, model)
    }

    pub fn normalize(&mut self) {
        if self.assistants.is_empty() {
            self.assistants.push(Assistant::default());
        }
        if self.assistant(&self.settings.assistant_id).is_none() {
            self.settings.assistant_id = self.assistants[0].id.clone();
        }
        if self.assistants.len() == 1
            && self.assistants[0].system_prompt.trim().is_empty()
            && !self.settings.system_prompt.trim().is_empty()
            && self.settings.system_prompt != "You are a helpful assistant."
        {
            self.assistants[0].system_prompt = self.settings.system_prompt.clone();
        }
        for assistant in &mut self.assistants {
            if assistant.id.trim().is_empty() {
                assistant.id = new_id();
            }
            assistant
                .skill_ids
                .retain(|id| self.skills.iter().any(|skill| &skill.id == id));
            if assistant.avatar_color.trim().is_empty() {
                assistant.avatar_color = DEFAULT_ASSISTANT_COLOR.into();
            }
        }
        if self.settings.color_mode != "light" && self.settings.color_mode != "dark" {
            self.settings.color_mode = "system".into();
        }
        if self.settings.theme_id.trim().is_empty() {
            self.settings.theme_id = "sakura".into();
        }
        self.settings.bubble_opacity = self.settings.bubble_opacity.clamp(0.2, 1.0);
        self.settings.font_scale = self.settings.font_scale.clamp(0.8, 1.3);
        if self.settings.user_avatar_color.trim().is_empty() {
            self.settings.user_avatar_color = "#8E4955".into();
        }
        self.settings.language = crate::i18n::normalize(&self.settings.language).to_string();

        let assistant_ids: Vec<String> = self.assistants.iter().map(|a| a.id.clone()).collect();
        let default_assistant_id = self.settings.assistant_id.clone();
        for chat in &mut self.chats {
            if chat.id.trim().is_empty() {
                chat.id = new_id();
            }
            if chat.title.trim().is_empty() {
                chat.title = "New Chat".into();
            }
            if !assistant_ids.iter().any(|id| id == &chat.assistant_id) {
                chat.assistant_id = default_assistant_id.clone();
            }
            if chat.created_at == 0 {
                chat.created_at = chat.updated_at;
            }
            if chat.updated_at == 0 {
                chat.updated_at = chat.created_at;
            }
            for message in &mut chat.messages {
                if message.id.trim().is_empty() {
                    message.id = new_id();
                }
                if message.created_at == 0 {
                    message.created_at = chat.updated_at;
                }
                if message.output_tokens == 0 && message.role == "assistant" {
                    message.output_tokens = estimate_tokens(&message.content);
                }
            }
        }

        for prompt in &mut self.prompts {
            if prompt.id.trim().is_empty() {
                prompt.id = new_id();
            }
        }
        for skill in &mut self.skills {
            if skill.id.trim().is_empty() {
                skill.id = new_id();
            }
        }
        for server in &mut self.mcp_servers {
            if server.id.trim().is_empty() {
                server.id = new_id();
            }
        }

        if self.usage.is_empty() {
            for chat in &self.chats {
                for message in &chat.messages {
                    if message.role == "assistant" && !message.content.trim().is_empty() {
                        self.usage.push(UsageEntry {
                            at: message.created_at,
                            provider_id: chat.provider_id.clone(),
                            model: chat.model.clone(),
                            input_tokens: message.input_tokens,
                            output_tokens: if message.output_tokens > 0 {
                                message.output_tokens
                            } else {
                                estimate_tokens(&message.content)
                            },
                        });
                    }
                }
            }
            self.usage.sort_by_key(|entry| entry.at);
        }
    }
}

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

pub fn new_id() -> String {
    format!(
        "{:x}{:03x}",
        now_ms(),
        ID_COUNTER.fetch_add(1, Ordering::Relaxed) & 0xfff
    )
}

pub fn estimate_tokens(text: &str) -> u32 {
    if text.trim().is_empty() {
        return 0;
    }
    ((text.chars().count() as f32) / 4.0).ceil().max(1.0) as u32
}

pub fn title_for(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    let mut title: String = line.chars().take(40).collect();
    if line.chars().count() > 40 {
        title.push('…');
    }
    if title.is_empty() {
        "New Chat".into()
    } else {
        title
    }
}

pub fn data_dir() -> &'static PathBuf {
    DATA_DIR.get_or_init(|| {
        #[cfg(target_os = "android")]
        if let Some(dir) = android_files_dir() {
            return dir;
        }
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("rustchat")
    })
}

fn legacy_json_path() -> PathBuf {
    data_dir().join("state.json")
}

fn db_path() -> PathBuf {
    data_dir().join("rustchat.db")
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    api_key TEXT NOT NULL,
    position INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS provider_models (
    provider_id TEXT NOT NULL,
    model TEXT NOT NULL,
    position INTEGER NOT NULL,
    PRIMARY KEY (provider_id, model)
);
CREATE TABLE IF NOT EXISTS assistants (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    system_prompt TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    model TEXT NOT NULL,
    avatar_color TEXT NOT NULL,
    use_assistant_avatar INTEGER NOT NULL,
    use_gradient_background INTEGER NOT NULL,
    enable_web_search INTEGER NOT NULL,
    position INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS assistant_skills (
    assistant_id TEXT NOT NULL,
    skill_id TEXT NOT NULL,
    PRIMARY KEY (assistant_id, skill_id)
);
CREATE TABLE IF NOT EXISTS conversations (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    assistant_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    model TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    pinned INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    reasoning TEXT,
    created_at INTEGER NOT NULL,
    favorite INTEGER NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    attachments TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS messages_conversation ON messages(conversation_id, position);
CREATE TABLE IF NOT EXISTS usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    at INTEGER NOT NULL,
    provider_id TEXT NOT NULL,
    model TEXT NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS prompts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    content TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS skills (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    content TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS mcp_servers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    api_key TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    tools TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS kv (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

static DB: OnceLock<Option<std::sync::Mutex<Connection>>> = OnceLock::new();

fn db() -> Option<&'static std::sync::Mutex<Connection>> {
    DB.get_or_init(|| match Connection::open(db_path()) {
        Ok(conn) => {
            if let Err(error) = conn.execute_batch(SCHEMA) {
                eprintln!("oratio: could not create database schema: {error}");
                return None;
            }
            Some(std::sync::Mutex::new(conn))
        }
        Err(error) => {
            eprintln!("oratio: could not open database: {error}");
            None
        }
    })
    .as_ref()
}

fn legacy_json() -> Option<AppData> {
    let path = legacy_json_path();
    let text = fs::read_to_string(&path).ok()?;
    match serde_json::from_str::<AppData>(&text) {
        Ok(data) => Some(data),
        Err(error) => {
            eprintln!("oratio: could not parse legacy state: {error}");
            None
        }
    }
}

fn database_is_empty(conn: &Connection) -> bool {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM providers", [], |row| row.get(0))
        .unwrap_or(0);
    let assistants: i64 = conn
        .query_row("SELECT COUNT(*) FROM assistants", [], |row| row.get(0))
        .unwrap_or(0);
    count == 0 && assistants == 0
}

fn read_all(conn: &Connection) -> rusqlite::Result<AppData> {
    let mut data = AppData::default();

    let mut stmt =
        conn.prepare("SELECT id, name, base_url, api_key FROM providers ORDER BY position ASC")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let mut models = Vec::new();
        let mut model_stmt = conn.prepare(
            "SELECT model FROM provider_models WHERE provider_id = ?1 ORDER BY position ASC",
        )?;
        let iter = model_stmt.query_map(params![id], |row| row.get::<_, String>(0))?;
        for model in iter {
            models.push(model?);
        }
        data.providers.push(Provider {
            id,
            name: row.get(1)?,
            base_url: row.get(2)?,
            api_key: row.get(3)?,
            models,
        });
    }

    let mut stmt = conn.prepare(
        "SELECT id, name, system_prompt, provider_id, model, avatar_color, use_assistant_avatar, use_gradient_background, enable_web_search FROM assistants ORDER BY position ASC",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let mut skill_ids = Vec::new();
        let mut skill_stmt =
            conn.prepare("SELECT skill_id FROM assistant_skills WHERE assistant_id = ?1")?;
        let iter = skill_stmt.query_map(params![id], |row| row.get::<_, String>(0))?;
        for skill in iter {
            skill_ids.push(skill?);
        }
        data.assistants.push(Assistant {
            id,
            name: row.get(1)?,
            system_prompt: row.get(2)?,
            provider_id: row.get(3)?,
            model: row.get(4)?,
            avatar_color: row.get(5)?,
            use_assistant_avatar: row.get::<_, i64>(6)? != 0,
            use_gradient_background: row.get::<_, i64>(7)? != 0,
            enable_web_search: row.get::<_, i64>(8)? != 0,
            skill_ids,
        });
    }

    let mut stmt = conn.prepare(
        "SELECT id, title, assistant_id, provider_id, model, created_at, updated_at, pinned FROM conversations ORDER BY updated_at DESC",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let mut messages = Vec::new();
        let mut msg_stmt = conn.prepare(
            "SELECT id, role, content, reasoning, created_at, favorite, input_tokens, output_tokens, attachments FROM messages WHERE conversation_id = ?1 ORDER BY position ASC",
        )?;
        let iter = msg_stmt.query_map(params![id], |row| {
            let attachments_json: String = row.get(8)?;
            let attachments: Vec<Attachment> =
                serde_json::from_str(&attachments_json).unwrap_or_default();
            Ok(ChatMessage {
                id: row.get(0)?,
                role: row.get(1)?,
                content: row.get(2)?,
                reasoning: row.get(3)?,
                created_at: row.get::<_, i64>(4)? as u64,
                favorite: row.get::<_, i64>(5)? != 0,
                input_tokens: row.get::<_, i64>(6)? as u32,
                output_tokens: row.get::<_, i64>(7)? as u32,
                attachments,
            })
        })?;
        for message in iter {
            messages.push(message?);
        }
        data.chats.push(Conversation {
            id,
            title: row.get(1)?,
            assistant_id: row.get(2)?,
            provider_id: row.get(3)?,
            model: row.get(4)?,
            created_at: row.get::<_, i64>(5)? as u64,
            updated_at: row.get::<_, i64>(6)? as u64,
            pinned: row.get::<_, i64>(7)? != 0,
            messages,
        });
    }

    let mut stmt = conn.prepare(
        "SELECT at, provider_id, model, input_tokens, output_tokens FROM usage ORDER BY at ASC",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        data.usage.push(UsageEntry {
            at: row.get::<_, i64>(0)? as u64,
            provider_id: row.get(1)?,
            model: row.get(2)?,
            input_tokens: row.get::<_, i64>(3)? as u32,
            output_tokens: row.get::<_, i64>(4)? as u32,
        });
    }

    let mut stmt = conn.prepare("SELECT id, name, content FROM prompts")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        data.prompts.push(Prompt {
            id: row.get(0)?,
            name: row.get(1)?,
            content: row.get(2)?,
        });
    }

    let mut stmt = conn.prepare("SELECT id, name, description, content FROM skills")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        data.skills.push(Skill {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            content: row.get(3)?,
        });
    }

    let mut stmt =
        conn.prepare("SELECT id, name, url, api_key, enabled, tools FROM mcp_servers")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let tools_json: String = row.get(5)?;
        let tools = parse_mcp_tools_json(&tools_json);
        data.mcp_servers.push(McpServer {
            id: row.get(0)?,
            name: row.get(1)?,
            url: row.get(2)?,
            api_key: row.get(3)?,
            enabled: row.get::<_, i64>(4)? != 0,
            tools,
        });
    }

    if let Ok(settings_json) =
        conn.query_row("SELECT value FROM kv WHERE key = 'settings'", [], |row| {
            row.get::<_, String>(0)
        })
    {
        if let Ok(settings) = serde_json::from_str::<Settings>(&settings_json) {
            data.settings = settings;
        }
    }

    Ok(data)
}

fn write_all(conn: &mut Connection, data: &AppData) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    for table in [
        "providers",
        "provider_models",
        "assistants",
        "assistant_skills",
        "conversations",
        "messages",
        "usage",
        "prompts",
        "skills",
        "mcp_servers",
    ] {
        tx.execute(&format!("DELETE FROM {table}"), [])?;
    }
    for (position, provider) in data.providers.iter().enumerate() {
        tx.execute(
            "INSERT INTO providers (id, name, base_url, api_key, position) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![provider.id, provider.name, provider.base_url, provider.api_key, position as i64],
        )?;
        for (index, model) in provider.models.iter().enumerate() {
            tx.execute(
                "INSERT INTO provider_models (provider_id, model, position) VALUES (?1, ?2, ?3)",
                params![provider.id, model, index as i64],
            )?;
        }
    }
    for (position, assistant) in data.assistants.iter().enumerate() {
        tx.execute(
            "INSERT INTO assistants (id, name, system_prompt, provider_id, model, avatar_color, use_assistant_avatar, use_gradient_background, enable_web_search, position) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                assistant.id,
                assistant.name,
                assistant.system_prompt,
                assistant.provider_id,
                assistant.model,
                assistant.avatar_color,
                assistant.use_assistant_avatar as i64,
                assistant.use_gradient_background as i64,
                assistant.enable_web_search as i64,
                position as i64
            ],
        )?;
        for skill_id in &assistant.skill_ids {
            tx.execute(
                "INSERT OR IGNORE INTO assistant_skills (assistant_id, skill_id) VALUES (?1, ?2)",
                params![assistant.id, skill_id],
            )?;
        }
    }
    for chat in &data.chats {
        tx.execute(
            "INSERT INTO conversations (id, title, assistant_id, provider_id, model, created_at, updated_at, pinned) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                chat.id,
                chat.title,
                chat.assistant_id,
                chat.provider_id,
                chat.model,
                chat.created_at as i64,
                chat.updated_at as i64,
                chat.pinned as i64
            ],
        )?;
        for (position, message) in chat.messages.iter().enumerate() {
            let attachments =
                serde_json::to_string(&message.attachments).unwrap_or_else(|_| "[]".into());
            tx.execute(
                "INSERT INTO messages (id, conversation_id, position, role, content, reasoning, created_at, favorite, input_tokens, output_tokens, attachments) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    message.id,
                    chat.id,
                    position as i64,
                    message.role,
                    message.content,
                    message.reasoning,
                    message.created_at as i64,
                    message.favorite as i64,
                    message.input_tokens as i64,
                    message.output_tokens as i64,
                    attachments
                ],
            )?;
        }
    }
    for entry in &data.usage {
        tx.execute(
            "INSERT INTO usage (at, provider_id, model, input_tokens, output_tokens) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                entry.at as i64,
                entry.provider_id,
                entry.model,
                entry.input_tokens as i64,
                entry.output_tokens as i64
            ],
        )?;
    }
    for prompt in &data.prompts {
        tx.execute(
            "INSERT INTO prompts (id, name, content) VALUES (?1, ?2, ?3)",
            params![prompt.id, prompt.name, prompt.content],
        )?;
    }
    for skill in &data.skills {
        tx.execute(
            "INSERT INTO skills (id, name, description, content) VALUES (?1, ?2, ?3, ?4)",
            params![skill.id, skill.name, skill.description, skill.content],
        )?;
    }
    for server in &data.mcp_servers {
        let tools = serde_json::to_string(&server.tools).unwrap_or_else(|_| "[]".into());
        tx.execute(
            "INSERT INTO mcp_servers (id, name, url, api_key, enabled, tools) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                server.id,
                server.name,
                server.url,
                server.api_key,
                server.enabled as i64,
                tools
            ],
        )?;
    }
    let settings = serde_json::to_string(&data.settings).unwrap_or_else(|_| "{}".into());
    tx.execute(
        "INSERT INTO kv (key, value) VALUES ('settings', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![settings],
    )?;
    tx.commit()
}

pub fn load() -> AppData {
    let Some(database) = db() else {
        return AppData::default();
    };
    let mut data = {
        let mut conn = match database.lock() {
            Ok(conn) => conn,
            Err(error) => {
                eprintln!("oratio: database lock poisoned: {error}");
                return AppData::default();
            }
        };
        if database_is_empty(&conn) {
            if let Some(mut legacy) = legacy_json() {
                legacy.normalize();
                if let Err(error) = write_all(&mut conn, &legacy) {
                    eprintln!("oratio: could not import legacy state: {error}");
                } else {
                    let _ = fs::rename(legacy_json_path(), data_dir().join("state.json.imported"));
                }
                legacy
            } else {
                AppData::default()
            }
        } else {
            match read_all(&conn) {
                Ok(data) => data,
                Err(error) => {
                    eprintln!("oratio: could not read database: {error}");
                    AppData::default()
                }
            }
        }
    };
    let changed = ensure_builtin_providers(&mut data);
    data.normalize();
    if changed {
        save(&data);
    }
    data
}

pub fn save(data: &AppData) {
    let Some(database) = db() else {
        return;
    };
    let mut conn = match database.lock() {
        Ok(conn) => conn,
        Err(error) => {
            eprintln!("oratio: database lock poisoned: {error}");
            return;
        }
    };
    if let Err(error) = write_all(&mut conn, data) {
        eprintln!("oratio: could not write database: {error}");
    }
}

pub fn database_size() -> u64 {
    fs::metadata(db_path()).map(|meta| meta.len()).unwrap_or(0)
}

#[cfg(target_os = "android")]
fn with_android_context<R>(
    f: impl FnOnce(&mut jni::JNIEnv, &jni::objects::JObject) -> Option<R>,
) -> Option<R> {
    let raw = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(raw.vm().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let context = unsafe { jni::objects::JObject::from_raw(raw.context().cast()) };
    f(&mut env, &context)
}

#[cfg(target_os = "android")]
fn android_files_dir() -> Option<PathBuf> {
    with_android_context(|env, context| {
        let file = env
            .call_method(context, "getFilesDir", "()Ljava/io/File;", &[])
            .ok()?
            .l()
            .ok()?;
        let path = env
            .call_method(&file, "getAbsolutePath", "()Ljava/lang/String;", &[])
            .ok()?
            .l()
            .ok()?;
        let path: jni::objects::JString = path.into();
        env.get_string(&path)
            .ok()
            .map(|text| PathBuf::from(text.to_string_lossy().into_owned()))
    })
}

pub fn open_url(url: &str) {
    #[cfg(not(target_os = "android"))]
    let _ = url;
    #[cfg(target_os = "android")]
    {
        use jni::objects::{JObject, JValue};
        let _ = with_android_context(|env, context| {
            let action = env.new_string("android.intent.action.VIEW").ok()?;
            let url_string = env.new_string(url).ok()?;
            let uri_class = env.find_class("android/net/Uri").ok()?;
            let uri = env
                .call_static_method(
                    uri_class,
                    "parse",
                    "(Ljava/lang/String;)Landroid/net/Uri;",
                    &[JValue::Object(&JObject::from(url_string))],
                )
                .ok()?
                .l()
                .ok()?;
            let intent_class = env.find_class("android/content/Intent").ok()?;
            let intent = env
                .new_object(
                    intent_class,
                    "(Ljava/lang/String;Landroid/net/Uri;)V",
                    &[JValue::Object(&JObject::from(action)), JValue::Object(&uri)],
                )
                .ok()?;
            let _ = env.call_method(
                context,
                "startActivity",
                "(Landroid/content/Intent;)V",
                &[JValue::Object(&intent)],
            );
            Some(())
        });
    }
}

pub fn finish_activity() {
    #[cfg(target_os = "android")]
    {
        let _ = with_android_context(|env, context| {
            let _ = env.call_method(context, "finish", "()V", &[]);
            Some(())
        });
    }
}

#[cfg(target_os = "android")]
fn clear_jni_exception(env: &mut jni::JNIEnv, what: &str) {
    if env.exception_check().unwrap_or(false) {
        log::warn!("oratio: JNI exception during {what}");
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
}

#[cfg(not(target_os = "android"))]
pub fn check_permission(_permission: &str) -> bool {
    false
}

#[cfg(target_os = "android")]
pub fn check_permission(permission: &str) -> bool {
    use jni::objects::JValue;
    with_android_context(|env, context| {
        let permission = env.new_string(permission).ok()?;
        let result = env.call_method(
            context,
            "checkSelfPermission",
            "(Ljava/lang/String;)I",
            &[JValue::Object(&permission)],
        );
        clear_jni_exception(env, "checkSelfPermission");
        Some(result.ok()?.i().ok()? == 0)
    })
    .unwrap_or(false)
}

#[cfg(not(target_os = "android"))]
pub fn request_permissions(_permissions: &[&str], _request_code: i32) {}

#[cfg(target_os = "android")]
pub fn request_permissions(permissions: &[&str], request_code: i32) {
    use jni::objects::{JObject, JValue};
    let _ = with_android_context(|env, context| {
        let class = env.find_class("java/lang/String");
        clear_jni_exception(env, "find java/lang/String");
        let class = class.ok()?;
        let array = env.new_object_array(permissions.len() as i32, class, JObject::null());
        clear_jni_exception(env, "new permission array");
        let array = array.ok()?;
        for (index, permission) in permissions.iter().enumerate() {
            let value = env.new_string(permission).ok()?;
            let _ = env.set_object_array_element(&array, index as i32, &value);
            clear_jni_exception(env, "fill permission array");
        }
        let result = env.call_method(
            context,
            "requestPermissions",
            "([Ljava/lang/String;I)V",
            &[JValue::Object(&array), JValue::Int(request_code)],
        );
        clear_jni_exception(env, "requestPermissions");
        result.ok()?;
        Some(())
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_takes_first_line_and_truncates() {
        assert_eq!(title_for("\n\n  Hello there \nsecond"), "Hello there");
        assert_eq!(title_for("   "), "New Chat");
        let long = "x".repeat(80);
        let title = title_for(&long);
        assert_eq!(title.chars().count(), 41);
        assert!(title.ends_with('…'));
    }

    #[test]
    fn state_round_trips() {
        let data = AppData {
            providers: vec![Provider {
                id: "p1".into(),
                name: "Test".into(),
                base_url: "https://example.com/v1".into(),
                api_key: "secret".into(),
                models: vec!["gpt-test".into()],
            }],
            assistants: vec![Assistant::default()],
            chats: vec![Conversation {
                id: "c1".into(),
                title: "Hi".into(),
                provider_id: "p1".into(),
                model: "gpt-test".into(),
                messages: vec![ChatMessage::user("hello")],
                created_at: 1,
                updated_at: 2,
                ..Default::default()
            }],
            settings: Settings::default(),
            usage: Vec::new(),
            prompts: Vec::new(),
            skills: Vec::new(),
            mcp_servers: Vec::new(),
        };
        let json = serde_json::to_string(&data).unwrap();
        assert_eq!(serde_json::from_str::<AppData>(&json).unwrap(), data);
    }

    #[test]
    fn missing_fields_get_defaults() {
        let data: AppData = serde_json::from_str(r#"{"providers":[]}"#).unwrap();
        assert!(data.settings.system_prompt.contains("helpful"));
        assert!(data.chats.is_empty());
    }

    #[test]
    fn normalize_creates_default_assistant() {
        let mut data: AppData =
            serde_json::from_str(r#"{"chats":[{"id":"c1","messages":[]}]}"#).unwrap();
        data.normalize();
        assert_eq!(data.assistants.len(), 1);
        assert_eq!(data.settings.assistant_id, DEFAULT_ASSISTANT_ID);
    }
}
