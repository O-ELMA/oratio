use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::store::McpTool;
use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ApiToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ApiMessage {
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ApiToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Delta {
    Content(String),
    Reasoning(String),
}

fn client() -> Result<&'static reqwest::Client, String> {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client);
    }
    let built = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .user_agent("Oratio/0.2 (Android)")
        .build()
        .map_err(|error| error.to_string())?;
    Ok(CLIENT.get_or_init(|| built))
}

pub fn endpoint(base_url: &str, path: &str) -> String {
    let mut base = base_url.trim().trim_end_matches('/');
    for suffix in ["/chat/completions", "/models", "/images/generations"] {
        if let Some(stripped) = base.strip_suffix(suffix) {
            base = stripped.trim_end_matches('/');
            break;
        }
    }
    format!("{base}/{path}")
}

fn network_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "Request timed out. Check the base URL and your connection.".into()
    } else if error.is_connect() {
        format!("Could not connect: {error}")
    } else {
        error.to_string()
    }
}

fn truncate(text: &str, max: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let mut short: String = text.chars().take(max).collect();
        short.push('…');
        short
    }
}

fn error_from_value(value: &Value) -> String {
    match value.get("error") {
        Some(Value::String(message)) => message.clone(),
        Some(Value::Object(map)) => map
            .get("message")
            .and_then(Value::as_str)
            .or_else(|| map.get("error").and_then(Value::as_str))
            .map(str::to_string)
            .unwrap_or_else(|| value.to_string()),
        Some(other) => other.to_string(),
        None => value.to_string(),
    }
}

fn http_error(status: reqwest::StatusCode, body: &str) -> String {
    let detail = serde_json::from_str::<Value>(body)
        .ok()
        .map(|value| error_from_value(&value))
        .unwrap_or_else(|| truncate(body, 300));
    if detail.is_empty() {
        format!("HTTP {status}")
    } else {
        format!("HTTP {status}: {detail}")
    }
}

fn model_ids(value: &Value) -> Vec<String> {
    let list = value
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| value.get("models").and_then(Value::as_array))
        .or_else(|| value.as_array());
    let mut models = Vec::new();
    if let Some(list) = list {
        for item in list {
            let id = if let Some(id) = item.as_str() {
                Some(id.to_string())
            } else {
                ["id", "name", "model"]
                    .iter()
                    .find_map(|key| item.get(*key).and_then(Value::as_str))
                    .map(str::to_string)
            };
            if let Some(id) = id {
                let id = id.trim().to_string();
                if !id.is_empty() {
                    models.push(id);
                }
            }
        }
    }
    models.sort();
    models.dedup();
    models
}

pub async fn fetch_models(base_url: &str, api_key: &str) -> Result<Vec<String>, String> {
    let url = endpoint(base_url, "models");
    let mut request = client()?.get(&url);
    let key = api_key.trim();
    if !key.is_empty() {
        request = request.bearer_auth(key);
    }
    let response = request.send().await.map_err(network_error)?;
    let status = response.status();
    let body = response.text().await.map_err(network_error)?;
    if !status.is_success() {
        return Err(http_error(status, &body));
    }
    let value: Value = serde_json::from_str(&body)
        .map_err(|error| format!("The provider did not return a model list: {error}"))?;
    Ok(model_ids(&value))
}

fn text_of(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => {
            let mut text = String::new();
            for part in parts {
                if let Some(inner) = part.get("text").and_then(Value::as_str) {
                    text.push_str(inner);
                }
            }
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        }
        _ => None,
    }
}

fn emit(value: &Value, on_delta: &mut impl FnMut(Delta) -> bool) -> Result<bool, String> {
    if let Some(error) = value.get("error") {
        if !error.is_null() {
            return Err(error_from_value(value));
        }
    }
    let Some(choices) = value.get("choices").and_then(Value::as_array) else {
        return Ok(true);
    };
    for choice in choices {
        let delta = choice.get("delta").or_else(|| choice.get("message"));
        let Some(delta) = delta else { continue };
        for key in ["reasoning_content", "reasoning"] {
            if let Some(text) = delta.get(key).and_then(text_of) {
                if !text.is_empty() && !on_delta(Delta::Reasoning(text)) {
                    return Ok(false);
                }
            }
        }
        if let Some(text) = delta.get("content").and_then(text_of) {
            if !text.is_empty() && !on_delta(Delta::Content(text)) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

#[derive(Default)]
pub struct SseParser {
    buffer: String,
    pub done: bool,
}

impl SseParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, chunk: &str) -> Vec<String> {
        self.buffer.push_str(chunk);
        let mut payloads = Vec::new();
        while let Some(position) = self.buffer.find('\n') {
            let line: String = self.buffer.drain(..=position).collect();
            if let Some(payload) = self.parse_line(line.trim_end_matches(['\n', '\r'])) {
                payloads.push(payload);
            }
        }
        payloads
    }

    pub fn finish(&mut self) -> Vec<String> {
        let rest = std::mem::take(&mut self.buffer);
        self.parse_line(rest.trim_end_matches(['\n', '\r']))
            .into_iter()
            .collect()
    }

    fn parse_line(&mut self, line: &str) -> Option<String> {
        let data = line.strip_prefix("data:")?.trim();
        if data == "[DONE]" {
            self.done = true;
            return None;
        }
        if data.is_empty() {
            return None;
        }
        Some(data.to_string())
    }
}

#[derive(Default)]
struct ToolCallState {
    id: String,
    name: String,
    arguments: String,
}

fn collect_tool_calls(value: &Value, state: &mut BTreeMap<usize, ToolCallState>) {
    let Some(choices) = value.get("choices").and_then(Value::as_array) else {
        return;
    };
    for choice in choices {
        let container = choice.get("delta").or_else(|| choice.get("message"));
        let Some(calls) = container
            .and_then(|container| container.get("tool_calls"))
            .and_then(Value::as_array)
        else {
            continue;
        };
        for (fallback_index, call) in calls.iter().enumerate() {
            let index = call
                .get("index")
                .and_then(Value::as_u64)
                .unwrap_or(fallback_index as u64) as usize;
            let entry = state.entry(index).or_default();
            if let Some(id) = call.get("id").and_then(Value::as_str) {
                if !id.is_empty() {
                    entry.id = id.to_string();
                }
            }
            let function = call.get("function");
            if let Some(name) = function
                .and_then(|function| function.get("name"))
                .and_then(Value::as_str)
            {
                if !name.is_empty() {
                    entry.name.push_str(name);
                }
            }
            if let Some(arguments) = function.and_then(|function| function.get("arguments")) {
                match arguments {
                    Value::String(text) => entry.arguments.push_str(text),
                    other if !other.is_null() => entry.arguments = other.to_string(),
                    _ => {}
                }
            }
        }
    }
}

fn collected_tool_calls(state: BTreeMap<usize, ToolCallState>) -> Vec<ApiToolCall> {
    state
        .into_values()
        .filter(|call| !call.name.trim().is_empty())
        .map(|call| ApiToolCall {
            id: call.id,
            name: call.name,
            arguments: if call.arguments.trim().is_empty() {
                "{}".into()
            } else {
                call.arguments
            },
        })
        .collect()
}

pub async fn stream_chat<F>(
    base_url: &str,
    api_key: &str,
    model: &str,
    messages: &[ApiMessage],
    on_delta: F,
) -> Result<(), String>
where
    F: FnMut(Delta) -> bool,
{
    stream_chat_with_tools(base_url, api_key, model, messages, &[], on_delta)
        .await
        .map(|_| ())
}

pub async fn stream_chat_with_tools<F>(
    base_url: &str,
    api_key: &str,
    model: &str,
    messages: &[ApiMessage],
    tools: &[Value],
    mut on_delta: F,
) -> Result<(String, Vec<ApiToolCall>), String>
where
    F: FnMut(Delta) -> bool,
{
    let url = endpoint(base_url, "chat/completions");
    let payload: Vec<Value> = messages
        .iter()
        .map(|message| {
            let mut object = serde_json::Map::new();
            object.insert("role".into(), Value::String(message.role.clone()));
            if let Some(tool_call_id) = &message.tool_call_id {
                object.insert("tool_call_id".into(), Value::String(tool_call_id.clone()));
            }
            if !message.tool_calls.is_empty() {
                object.insert("content".into(), Value::String(message.content.clone()));
                let calls: Vec<Value> = message
                    .tool_calls
                    .iter()
                    .map(|call| {
                        serde_json::json!({
                            "id": call.id,
                            "type": "function",
                            "function": {
                                "name": call.name,
                                "arguments": call.arguments,
                            }
                        })
                    })
                    .collect();
                object.insert("tool_calls".into(), Value::Array(calls));
            } else if message.images.is_empty() {
                object.insert("content".into(), Value::String(message.content.clone()));
            } else {
                let mut parts = Vec::new();
                if !message.content.trim().is_empty() {
                    parts.push(serde_json::json!({ "type": "text", "text": message.content }));
                }
                for image in &message.images {
                    parts.push(serde_json::json!({
                        "type": "image_url",
                        "image_url": { "url": image }
                    }));
                }
                object.insert("content".into(), Value::Array(parts));
            }
            Value::Object(object)
        })
        .collect();
    let mut body = serde_json::json!({
        "model": model,
        "messages": payload,
        "stream": true,
    });
    if !tools.is_empty() {
        body["tools"] = Value::Array(tools.to_vec());
        body["tool_choice"] = Value::String("auto".into());
    }
    let mut request = client()?.post(&url).json(&body);
    let key = api_key.trim();
    if !key.is_empty() {
        request = request.bearer_auth(key);
    }
    let response = request.send().await.map_err(network_error)?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.map_err(network_error)?;
        return Err(http_error(status, &body));
    }

    let mut parser = SseParser::new();
    let mut raw = String::new();
    let mut saw_event = false;
    let mut stopped = false;
    let mut stream = response.bytes_stream();
    let mut assistant_text = String::new();
    let mut tool_state: BTreeMap<usize, ToolCallState> = BTreeMap::new();

    let mut sink = |delta: Delta| {
        if let Delta::Content(text) = &delta {
            assistant_text.push_str(text);
        }
        on_delta(delta)
    };

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(network_error)?;
        let text = String::from_utf8_lossy(&chunk);
        if !saw_event {
            raw.push_str(&text);
        }
        for payload in parser.feed(&text) {
            saw_event = true;
            raw.clear();
            let value: Value = serde_json::from_str(&payload)
                .map_err(|error| format!("Invalid stream chunk: {error}"))?;
            if !emit(&value, &mut sink)? {
                stopped = true;
                break;
            }
            collect_tool_calls(&value, &mut tool_state);
        }
        if stopped || parser.done {
            break;
        }
    }

    if !stopped && !parser.done {
        for payload in parser.finish() {
            saw_event = true;
            let value: Value = serde_json::from_str(&payload)
                .map_err(|error| format!("Invalid stream chunk: {error}"))?;
            if !emit(&value, &mut sink)? {
                stopped = true;
                break;
            }
            collect_tool_calls(&value, &mut tool_state);
        }
    }

    if !saw_event && !stopped {
        let raw = raw.trim();
        if !raw.is_empty() {
            let value: Value = serde_json::from_str(raw).map_err(|_| {
                format!(
                    "The provider returned an unexpected response: {}",
                    truncate(raw, 300)
                )
            })?;
            emit(&value, &mut sink)?;
            collect_tool_calls(&value, &mut tool_state);
        }
    }
    Ok((assistant_text, collected_tool_calls(tool_state)))
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

pub async fn web_search(
    provider: &str,
    api_key: &str,
    query: &str,
) -> Result<Vec<SearchResult>, String> {
    if api_key.trim().is_empty() {
        return Err("Add a search API key in Settings → Web Search".into());
    }
    let client = client()?;
    match provider {
        "tavily" => {
            let response = client
                .post("https://api.tavily.com/search")
                .json(&serde_json::json!({
                    "api_key": api_key.trim(),
                    "query": query,
                    "max_results": 5,
                    "include_answer": false,
                }))
                .timeout(Duration::from_secs(30))
                .send()
                .await
                .map_err(network_error)?;
            let status = response.status();
            let body = response.text().await.map_err(network_error)?;
            if !status.is_success() {
                return Err(http_error(status, &body));
            }
            let value: Value = serde_json::from_str(&body).map_err(|error| error.to_string())?;
            let mut results = Vec::new();
            if let Some(items) = value.get("results").and_then(Value::as_array) {
                for item in items {
                    results.push(SearchResult {
                        title: item
                            .get("title")
                            .and_then(Value::as_str)
                            .unwrap_or("Result")
                            .to_string(),
                        url: item
                            .get("url")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        snippet: item
                            .get("content")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    });
                }
            }
            Ok(results)
        }
        "exa" => {
            let response = client
                .post("https://api.exa.ai/search")
                .header("x-api-key", api_key.trim())
                .json(&serde_json::json!({
                    "query": query,
                    "numResults": 5,
                    "contents": { "text": { "maxCharacters": 600 } },
                }))
                .timeout(Duration::from_secs(30))
                .send()
                .await
                .map_err(network_error)?;
            let status = response.status();
            let body = response.text().await.map_err(network_error)?;
            if !status.is_success() {
                return Err(http_error(status, &body));
            }
            let value: Value = serde_json::from_str(&body).map_err(|error| error.to_string())?;
            let mut results = Vec::new();
            if let Some(items) = value.get("results").and_then(Value::as_array) {
                for item in items {
                    results.push(SearchResult {
                        title: item
                            .get("title")
                            .and_then(Value::as_str)
                            .unwrap_or("Result")
                            .to_string(),
                        url: item
                            .get("url")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        snippet: item
                            .get("text")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    });
                }
            }
            Ok(results)
        }
        "brave" => {
            let response = client
                .get("https://api.search.brave.com/res/v1/web/search")
                .query(&[("q", query), ("count", "5")])
                .header("X-Subscription-Token", api_key.trim())
                .header("Accept", "application/json")
                .timeout(Duration::from_secs(30))
                .send()
                .await
                .map_err(network_error)?;
            let status = response.status();
            let body = response.text().await.map_err(network_error)?;
            if !status.is_success() {
                return Err(http_error(status, &body));
            }
            let value: Value = serde_json::from_str(&body).map_err(|error| error.to_string())?;
            let mut results = Vec::new();
            if let Some(items) = value
                .get("web")
                .and_then(|web| web.get("results"))
                .and_then(Value::as_array)
            {
                for item in items {
                    results.push(SearchResult {
                        title: item
                            .get("title")
                            .and_then(Value::as_str)
                            .unwrap_or("Result")
                            .to_string(),
                        url: item
                            .get("url")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        snippet: item
                            .get("description")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    });
                }
            }
            Ok(results)
        }
        _ => Err("Choose a search provider first".into()),
    }
}

pub async fn generate_image(
    base_url: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    size: &str,
) -> Result<String, String> {
    let url = endpoint(base_url, "images/generations");
    let mut body = serde_json::json!({
        "prompt": prompt,
        "n": 1,
        "size": size,
    });
    if !model.trim().is_empty() {
        body["model"] = Value::String(model.trim().to_string());
    }
    let mut request = client()?.post(&url).json(&body);
    if !api_key.trim().is_empty() {
        request = request.bearer_auth(api_key.trim());
    }
    let response = request.send().await.map_err(network_error)?;
    let status = response.status();
    let text = response.text().await.map_err(network_error)?;
    if !status.is_success() {
        return Err(http_error(status, &text));
    }
    let value: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    if let Some(error) = value.get("error") {
        if !error.is_null() {
            return Err(error_from_value(&value));
        }
    }
    let item = value
        .get("data")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .ok_or_else(|| "The provider returned no image".to_string())?;
    if let Some(b64) = item.get("b64_json").and_then(Value::as_str) {
        return Ok(format!("data:image/png;base64,{b64}"));
    }
    if let Some(url) = item.get("url").and_then(Value::as_str) {
        return Ok(url.to_string());
    }
    Err("The provider returned an image in an unsupported format".into())
}

fn parse_mcp_body(body: &str, content_type: &str) -> Result<Value, String> {
    if content_type.contains("text/event-stream") || body.trim_start().starts_with("event:") {
        let mut last = None;
        for line in body.lines() {
            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim();
                if let Ok(value) = serde_json::from_str::<Value>(data) {
                    last = Some(value);
                }
            }
        }
        last.ok_or_else(|| "The MCP server returned an empty stream".to_string())
    } else {
        serde_json::from_str(body).map_err(|error| format!("Invalid MCP response: {error}"))
    }
}

async fn mcp_rpc(
    url: &str,
    api_key: &str,
    session: Option<&str>,
    payload: Value,
) -> Result<(Value, Option<String>), String> {
    let mut request = client()?
        .post(url)
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2024-11-05")
        .timeout(Duration::from_secs(60))
        .json(&payload);
    if let Some(session) = session {
        request = request.header("Mcp-Session-Id", session);
    }
    if !api_key.trim().is_empty() {
        request = request.bearer_auth(api_key.trim());
    }
    let response = request.send().await.map_err(network_error)?;
    let status = response.status();
    let new_session = response
        .headers()
        .get("Mcp-Session-Id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = response.text().await.map_err(network_error)?;
    if !status.is_success() {
        return Err(http_error(status, &body));
    }
    if body.trim().is_empty() {
        return Ok((Value::Null, new_session));
    }
    let value = parse_mcp_body(&body, &content_type)?;
    if let Some(error) = value.get("error") {
        if !error.is_null() {
            return Err(error_from_value(&value));
        }
    }
    Ok((value, new_session))
}

pub async fn mcp_list_tools(url: &str, api_key: &str) -> Result<Vec<McpTool>, String> {
    let initialize = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "Oratio", "version": "0.2" }
        }
    });
    let (_, session) = mcp_rpc(url, api_key, None, initialize).await?;
    let _ = mcp_rpc(
        url,
        api_key,
        session.as_deref(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }),
    )
    .await?;
    let (value, _) = mcp_rpc(
        url,
        api_key,
        session.as_deref(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
    )
    .await?;
    let mut tools = Vec::new();
    if let Some(items) = value
        .get("result")
        .and_then(|result| result.get("tools"))
        .and_then(Value::as_array)
    {
        for item in items {
            let Some(name) = item.get("name").and_then(Value::as_str) else {
                continue;
            };
            tools.push(McpTool {
                name: name.to_string(),
                description: item
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                input_schema: item
                    .get("inputSchema")
                    .or_else(|| item.get("input_schema"))
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({ "type": "object", "properties": {} })),
            });
        }
    }
    Ok(tools)
}

pub async fn mcp_call_tool(
    url: &str,
    api_key: &str,
    tool_name: &str,
    arguments: &str,
) -> Result<String, String> {
    let arguments: Value = serde_json::from_str(if arguments.trim().is_empty() {
        "{}"
    } else {
        arguments
    })
    .map_err(|error| format!("Invalid tool arguments: {error}"))?;
    let initialize = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "Oratio", "version": "0.2" }
        }
    });
    let (_, session) = mcp_rpc(url, api_key, None, initialize).await?;
    let _ = mcp_rpc(
        url,
        api_key,
        session.as_deref(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }),
    )
    .await?;
    let (value, _) = mcp_rpc(
        url,
        api_key,
        session.as_deref(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments,
            }
        }),
    )
    .await?;
    let result = value.get("result").cloned().unwrap_or(Value::Null);
    let is_error = result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut text = String::new();
    if let Some(items) = result.get("content").and_then(Value::as_array) {
        for item in items {
            match item.get("type").and_then(Value::as_str) {
                Some("text") => {
                    if let Some(value) = item.get("text").and_then(Value::as_str) {
                        if !text.is_empty() {
                            text.push('\n');
                        }
                        text.push_str(value);
                    }
                }
                _ => {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(&item.to_string());
                }
            }
        }
    }
    if text.trim().is_empty() {
        text = result.to_string();
    }
    if is_error {
        text = format!("Tool error: {text}");
    }
    Ok(truncate(&text, 20000))
}
