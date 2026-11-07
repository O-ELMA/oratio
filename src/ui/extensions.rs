use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::*, Icon};
use oratio::{
    api, i18n,
    store::{self, McpServer, Prompt, Skill},
};

use crate::app::{mutate, persist, toast, DATA};
use crate::ui::common::{copy_text, ConfirmButton, PageHeader, SectionCard, Sheet, Toggle};

#[component]
pub fn SkillsPage() -> Element {
    let data = DATA();
    let skills = data.skills.clone();
    let assistants = data.assistants.clone();
    let mut editing = use_signal(|| None::<Skill>);
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut content = use_signal(String::new);

    let mut open_editor = move |skill: Option<Skill>| {
        let skill = skill.unwrap_or_else(|| Skill {
            id: store::new_id(),
            name: String::new(),
            description: String::new(),
            content: String::new(),
        });
        name.set(skill.name.clone());
        description.set(skill.description.clone());
        content.set(skill.content.clone());
        editing.set(Some(skill));
    };

    rsx! {
        div { class: "screen",
            PageHeader { title: i18n::t("skills.title"), subtitle: i18n::t("skills.subtitle"),
                button {
                    class: "icon-btn",
                    onclick: move |_| open_editor(None),
                    Icon { icon: LdPlus, width: 20, height: 20, fill: "currentColor" }
                }
            }
            div { class: "content",
                if skills.is_empty() {
                    div { class: "muted-note",
                        {i18n::t("skills.empty")}
                    }
                    button { class: "btn full", onclick: move |_| open_editor(None), {i18n::t("common.create")} }
                } else {
                    for skill in skills {
                        {
                            let users = assistants
                                .iter()
                                .filter(|assistant| assistant.skill_ids.contains(&skill.id))
                                .count();
                            let id = skill.id.clone();
                            rsx! {
                                div { key: "{skill.id}", class: "extension-card",
                                    div { class: "row-icon tertiary",
                                        Icon { icon: LdWrench, width: 20, height: 20, fill: "currentColor" }
                                    }
                                    div { class: "extension-card-text",
                                        div { class: "extension-card-title", "{skill.name}" }
                                        div { class: "extension-card-sub", "{skill.description}" }
                                        div { class: "extension-card-meta", {i18n::tn("common.assistantsUsed", users)} }
                                    }
                                    div { class: "card-actions",
                                        button {
                                            class: "icon-btn small",
                                            onclick: move |_| open_editor(Some(skill.clone())),
                                            Icon { icon: LdPencil, width: 16, height: 16, fill: "currentColor" }
                                        }
                                        ConfirmButton {
                                            class: "icon-btn small danger-text confirm-icon",
                                            title: i18n::t("common.delete"),
                                            onconfirm: move |_| {
                                                let remove_id = id.clone();
                                                mutate(|data| {
                                                    data.skills.retain(|skill| skill.id != remove_id);
                                                    for assistant in &mut data.assistants {
                                                        assistant.skill_ids.retain(|skill_id| skill_id != &remove_id);
                                                    }
                                                });
                                                persist();
                                                toast(i18n::t("skills.deleted"));
                                            },
                                            Icon { icon: LdTrash2, width: 16, height: 16, fill: "currentColor" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(skill) = editing() {
                Sheet { title: String::new(), on_close: move |_| editing.set(None),
                    div { class: "field",
                        label { {i18n::t("skills.name")} }
                        input {
                            class: "input",
                            initial_value: "{name}",
                            oninput: move |event| name.set(event.value()),
                        }
                    }
                    div { class: "field",
                        label { {i18n::t("skills.description")} }
                        input {
                            class: "input",
                            placeholder: i18n::t("skills.descriptionPlaceholder"),
                            initial_value: "{description}",
                            oninput: move |event| description.set(event.value()),
                        }
                    }
                    div { class: "field",
                        label { {i18n::t("skills.instructions")} }
                        textarea {
                            class: "input skill-input",
                            rows: "8",
                            placeholder: i18n::t("skills.instructionsPlaceholder"),
                            initial_value: "{content}",
                            oninput: move |event| content.set(event.value()),
                        }
                    }
                    div { class: "sheet-actions",
                        button { class: "btn ghost", onclick: move |_| editing.set(None), {i18n::t("common.cancel")} }
                        button {
                            class: "btn",
                            onclick: move |_| {
                                let name_value = name().trim().to_string();
                                if name_value.is_empty() {
                                    toast(i18n::t("skills.needName"));
                                    return;
                                }
                                let mut value = skill.clone();
                                value.name = name_value;
                                value.description = description().trim().to_string();
                                value.content = content();
                                let id = value.id.clone();
                                mutate(|data| {
                                    if let Some(existing) = data.skills.iter_mut().find(|item| item.id == id) {
                                        *existing = value.clone();
                                    } else {
                                        data.skills.push(value.clone());
                                    }
                                });
                                persist();
                                editing.set(None);
                                toast(i18n::t("skills.saved"));
                            },
                            {i18n::t("common.save")}
                        }
                    }
                }
            }

        }
    }
}

#[component]
pub fn PromptsPage() -> Element {
    let data = DATA();
    let prompts = data.prompts.clone();
    let mut editing = use_signal(|| None::<Prompt>);
    let mut name = use_signal(String::new);
    let mut content = use_signal(String::new);

    let mut open_editor = move |prompt: Option<Prompt>| {
        let prompt = prompt.unwrap_or_else(|| Prompt {
            id: store::new_id(),
            name: String::new(),
            content: String::new(),
        });
        name.set(prompt.name.clone());
        content.set(prompt.content.clone());
        editing.set(Some(prompt));
    };

    rsx! {
        div { class: "screen",
            PageHeader { title: i18n::t("prompts.title"), subtitle: i18n::t("prompts.subtitle"),
                button {
                    class: "icon-btn",
                    onclick: move |_| open_editor(None),
                    Icon { icon: LdPlus, width: 20, height: 20, fill: "currentColor" }
                }
            }
            div { class: "content",
                if prompts.is_empty() {
                    div { class: "muted-note",
                        {i18n::t("prompts.empty")}
                    }
                    button { class: "btn full", onclick: move |_| open_editor(None), {i18n::t("common.create")} }
                } else {
                    for prompt in prompts {
                        {
                            let id = prompt.id.clone();
                            rsx! {
                                div { key: "{prompt.id}", class: "extension-card",
                                    div { class: "row-icon primary",
                                        Icon { icon: LdBookOpen, width: 20, height: 20, fill: "currentColor" }
                                    }
                                    div { class: "extension-card-text",
                                        div { class: "extension-card-title", "{prompt.name}" }
                                        div { class: "extension-card-preview", "{prompt.content}" }
                                    }
                                    div { class: "card-actions",
                                        button {
                                            class: "icon-btn small",
                                            onclick: move |_| open_editor(Some(prompt.clone())),
                                            Icon { icon: LdPencil, width: 16, height: 16, fill: "currentColor" }
                                        }
                                        ConfirmButton {
                                            class: "icon-btn small danger-text confirm-icon",
                                            title: i18n::t("common.delete"),
                                            onconfirm: move |_| {
                                                let remove_id = id.clone();
                                                mutate(|data| {
                                                    data.skills.retain(|skill| skill.id != remove_id);
                                                    for assistant in &mut data.assistants {
                                                        assistant.skill_ids.retain(|skill_id| skill_id != &remove_id);
                                                    }
                                                });
                                                persist();
                                                toast(i18n::t("prompts.deleted"));
                                            },
                                            Icon { icon: LdTrash2, width: 16, height: 16, fill: "currentColor" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(prompt) = editing() {
                Sheet { title: String::new(), on_close: move |_| editing.set(None),
                    div { class: "field",
                        label { {i18n::t("prompts.name")} }
                        input {
                            class: "input",
                            initial_value: "{name}",
                            oninput: move |event| name.set(event.value()),
                        }
                    }
                    div { class: "field",
                        label { {i18n::t("prompts.prompt")} }
                        textarea {
                            class: "input skill-input",
                            rows: "7",
                            initial_value: "{content}",
                            oninput: move |event| content.set(event.value()),
                        }
                    }
                    div { class: "sheet-actions",
                        button { class: "btn ghost", onclick: move |_| editing.set(None), {i18n::t("common.cancel")} }
                        button {
                            class: "btn",
                            onclick: move |_| {
                                let name_value = name().trim().to_string();
                                if name_value.is_empty() {
                                    toast(i18n::t("prompts.needName"));
                                    return;
                                }
                                let mut value = prompt.clone();
                                value.name = name_value;
                                value.content = content();
                                let id = value.id.clone();
                                mutate(|data| {
                                    if let Some(existing) = data.prompts.iter_mut().find(|item| item.id == id) {
                                        *existing = value.clone();
                                    } else {
                                        data.prompts.push(value.clone());
                                    }
                                });
                                persist();
                                editing.set(None);
                                toast(i18n::t("prompts.saved"));
                            },
                            {i18n::t("common.save")}
                        }
                    }
                }
            }

        }
    }
}

#[component]
pub fn WebSearchPage() -> Element {
    let data = DATA();
    let settings = data.settings.clone();
    let mut provider = use_signal(|| settings.search_provider.clone());
    let mut api_key = use_signal(|| settings.search_api_key.clone());
    let mut query = use_signal(|| i18n::t("websearch.defaultQuery"));
    let mut results = use_signal(Vec::<api::SearchResult>::new);
    let mut testing = use_signal(|| false);
    let mut error = use_signal(String::new);

    rsx! {
        div { class: "screen",
            PageHeader { title: i18n::t("websearch.title"), subtitle: i18n::t("websearch.subtitle") }
            div { class: "content",
                SectionCard { title: i18n::t("websearch.provider"), subtitle: "",
                    div { class: "chip-row",
                        for (id, label) in [("none", i18n::t("common.off")), ("tavily", "Tavily".to_string()), ("exa", "Exa".to_string()), ("brave", "Brave".to_string())] {
                            button {
                                class: if provider() == id { "chip selected" } else { "chip" },
                                onclick: move |_| provider.set(id.to_string()),
                                "{label}"
                            }
                        }
                    }
                    div { class: "field",
                        label { {i18n::t("websearch.apiKey")} }
                        input {
                            class: "input",
                            r#type: "password",
                            placeholder: i18n::t("websearch.apiKeyPlaceholder"),
                            autocapitalize: "off",
                            autocorrect: "off",
                            spellcheck: "false",
                            initial_value: "{api_key}",
                            oninput: move |event| api_key.set(event.value()),
                        }
                    }
                    button {
                        class: "btn full",
                        onclick: move |_| {
                            let provider_value = provider();
                            let key_value = api_key().trim().to_string();
                            mutate(|data| {
                                data.settings.search_provider = provider_value;
                                data.settings.search_api_key = key_value;
                            });
                            persist();
                            toast(i18n::t("websearch.saved"));
                        },
                        {i18n::t("common.save")}
                    }
                }

                SectionCard { title: i18n::t("websearch.test"), subtitle: i18n::t("websearch.testSub"),
                    div { class: "field",
                        label { {i18n::t("websearch.testQuery")} }
                        input {
                            class: "input",
                            initial_value: "{query}",
                            oninput: move |event| query.set(event.value()),
                        }
                    }
                    button {
                        class: "btn ghost full",
                        disabled: testing(),
                        onclick: move |_| {
                            let provider_value = provider();
                            let key_value = api_key().trim().to_string();
                            let query_value = query().trim().to_string();
                            if provider_value == "none" {
                                toast(i18n::t("websearch.chooseProvider"));
                                return;
                            }
                            if key_value.is_empty() || query_value.is_empty() {
                                toast(i18n::t("websearch.addKeyAndQuery"));
                                return;
                            }
                            testing.set(true);
                            error.set(String::new());
                            results.set(Vec::new());
                            let mut results_signal = results;
                            let mut testing_signal = testing;
                            let mut error_signal = error;
                            dioxus::prelude::spawn(async move {
                                match api::web_search(&provider_value, &key_value, &query_value).await {
                                    Ok(found) => results_signal.set(found),
                                    Err(message) => error_signal.set(message),
                                }
                                testing_signal.set(false);
                            });
                        },
                        if testing() { {i18n::t("websearch.searching")} } else { {i18n::t("websearch.testAction")} }
                    }
                    if !error().is_empty() {
                        div { class: "errorbar", span { class: "error-text", "{error}" } }
                    }
                    for result in results() {
                        button {
                            key: "{result.url}",
                            class: "search-result",
                            onclick: move |_| copy_text(&result.url),
                            div { class: "search-result-text",
                                div { class: "search-result-title", "{result.title}" }
                                div { class: "search-result-snippet", "{result.snippet}" }
                                div { class: "search-result-meta", "{result.url}" }
                            }
                        }
                    }
                }

                SectionCard { title: i18n::t("websearch.howUsed"), subtitle: "",
                    div { class: "about-line",
                        {i18n::t("websearch.howUsedBody")}
                    }
                    div { class: "about-line muted",
                        {i18n::t("websearch.supported")}
                    }
                }
            }
        }
    }
}

#[component]
pub fn McpPage() -> Element {
    let data = DATA();
    let servers = data.mcp_servers.clone();
    let mut editing = use_signal(|| None::<McpServer>);
    let mut name = use_signal(String::new);
    let mut url = use_signal(String::new);
    let mut key = use_signal(String::new);
    let mut enabled = use_signal(|| true);
    let mut testing = use_signal(|| false);
    let mut status = use_signal(String::new);
    let mut tested_tools = use_signal(Vec::<oratio::store::McpTool>::new);

    let mut open_editor = move |server: Option<McpServer>| {
        let server = server.unwrap_or_else(|| McpServer {
            id: store::new_id(),
            name: String::new(),
            url: String::new(),
            api_key: String::new(),
            enabled: true,
            tools: Vec::new(),
        });
        name.set(server.name.clone());
        url.set(server.url.clone());
        key.set(server.api_key.clone());
        enabled.set(server.enabled);
        tested_tools.set(server.tools.clone());
        status.set(String::new());
        editing.set(Some(server));
    };

    rsx! {
        div { class: "screen",
            PageHeader { title: i18n::t("mcp.title"), subtitle: i18n::t("mcp.subtitle"),
                button {
                    class: "icon-btn",
                    onclick: move |_| open_editor(None),
                    Icon { icon: LdPlus, width: 20, height: 20, fill: "currentColor" }
                }
            }
            div { class: "content",
                if servers.is_empty() {
                    div { class: "muted-note",
                        {i18n::t("mcp.empty")}
                    }
                    button { class: "btn full", onclick: move |_| open_editor(None), {i18n::t("common.create")} }
                } else {
                    for server in servers {
                        {
                            let id = server.id.clone();
                            let server_id = server.id.clone();
                            rsx! {
                                div { key: "{server.id}", class: "extension-card",
                                    div { class: "row-icon secondary",
                                        Icon { icon: LdServer, width: 20, height: 20, fill: "currentColor" }
                                    }
                                    div { class: "extension-card-text",
                                        div { class: "extension-card-title", "{server.name}" }
                                        div { class: "extension-card-sub", "{server.url}" }
                                        div { class: "extension-card-meta",
                                            if server.tools.is_empty() {
                                                {i18n::t("mcp.noTools")}
                                            } else {
                                                {i18n::tn("common.tools", server.tools.len())}
                                            }
                                        }
                                        if !server.tools.is_empty() {
                                            div { class: "tag-row",
                                                for tool in server.tools.iter().take(8) {
                                                    span { key: "{tool.name}", class: "tag", "{tool.name}" }
                                                }
                                            }
                                        }
                                    }
                                    div { class: "card-actions",
                                        Toggle {
                                            checked: server.enabled,
                                            onchange: move |value| {
                                                mutate(|data| {
                                                    if let Some(item) = data.mcp_servers.iter_mut().find(|item| item.id == server_id) {
                                                        item.enabled = value;
                                                    }
                                                });
                                                persist();
                                            },
                                        }
                                        button {
                                            class: "icon-btn small",
                                            onclick: move |_| open_editor(Some(server.clone())),
                                            Icon { icon: LdPencil, width: 16, height: 16, fill: "currentColor" }
                                        }
                                        ConfirmButton {
                                            class: "icon-btn small danger-text confirm-icon",
                                            title: i18n::t("common.delete"),
                                            onconfirm: move |_| {
                                                let remove_id = id.clone();
                                                mutate(|data| {
                                                    data.skills.retain(|skill| skill.id != remove_id);
                                                    for assistant in &mut data.assistants {
                                                        assistant.skill_ids.retain(|skill_id| skill_id != &remove_id);
                                                    }
                                                });
                                                persist();
                                                toast(i18n::t("mcp.deleted"));
                                            },
                                            Icon { icon: LdTrash2, width: 16, height: 16, fill: "currentColor" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "muted-note",
                        {i18n::t("mcp.note")}
                    }
                }
            }

            if let Some(server) = editing() {
                {
                let server_save = server.clone();
                let server_test_id = server.id.clone();
                rsx! {
                Sheet { title: if server.name.is_empty() { i18n::t("mcp.new") } else { i18n::t("mcp.edit") }, on_close: move |_| editing.set(None),
                    div { class: "field",
                        label { {i18n::t("mcp.name")} }
                        input {
                            class: "input",
                            initial_value: "{name}",
                            oninput: move |event| name.set(event.value()),
                        }
                    }
                    div { class: "field",
                        label { {i18n::t("mcp.endpoint")} }
                        input {
                            class: "input",
                            placeholder: i18n::t("mcp.endpointPlaceholder"),
                            autocapitalize: "off",
                            autocorrect: "off",
                            spellcheck: "false",
                            initial_value: "{url}",
                            oninput: move |event| url.set(event.value()),
                        }
                    }
                    div { class: "field",
                        label { {i18n::t("mcp.token")} }
                        input {
                            class: "input",
                            r#type: "password",
                            initial_value: "{key}",
                            oninput: move |event| key.set(event.value()),
                        }
                    }
                    div { class: "setting-row static",
                        div { class: "row-icon secondary",
                            Icon { icon: LdServer, width: 18, height: 18, fill: "currentColor" }
                        }
                        div { class: "setting-row-text", span { {i18n::t("mcp.enabled")} } }
                        Toggle { checked: enabled(), onchange: move |value| enabled.set(value) }
                    }
                    button {
                            class: "btn ghost full",
                            disabled: testing(),
                            onclick: move |_| {
                                let url_value = url().trim().to_string();
                                let key_value = key().trim().to_string();
                                if url_value.is_empty() {
                                    status.set(i18n::t("mcp.enterEndpoint"));
                                    return;
                                }
                                testing.set(true);
                                status.set(i18n::t("mcp.contacting"));
                                let mut status_signal = status;
                                let mut testing_signal = testing;
                                let mut tools_signal = tested_tools;
                                let edited_id = server_test_id.clone();
                                dioxus::prelude::spawn(async move {
                                    match api::mcp_list_tools(&url_value, &key_value).await {
                                        Ok(tools) => {
                                            let mut data = DATA.signal();
                                            let mut guard = data.write();
                                            if let Some(item) = guard.mcp_servers.iter_mut().find(|item| item.id == edited_id) {
                                                item.tools = tools.clone();
                                            }
                                            drop(guard);
                                            crate::app::persist();
                                            tools_signal.set(tools.clone());
                                            status_signal.set(i18n::ta("mcp.connected", &[("count", &tools.len().to_string())]));
                                        }
                                        Err(message) => status_signal.set(i18n::ta("mcp.connectionFailed", &[("error", &message)])),
                                    }
                                    testing_signal.set(false);
                                });
                            },
                            if testing() { {i18n::t("mcp.testing")} } else { {i18n::t("mcp.testConnection")} }
                    }
                    if !status().is_empty() {
                        div { class: "hint status-text", "{status}" }
                    }
                    div { class: "sheet-actions",
                        button { class: "btn ghost", onclick: move |_| editing.set(None), {i18n::t("common.cancel")} }
                        button {
                            class: "btn",
                            onclick: move |_| {
                                let name_value = name().trim().to_string();
                                let url_value = url().trim().to_string();
                                if name_value.is_empty() || url_value.is_empty() {
                                    toast(i18n::t("mcp.required"));
                                    return;
                                }
                                let mut value = server_save.clone();
                                value.name = name_value;
                                value.url = url_value;
                                value.api_key = key().trim().to_string();
                                value.enabled = enabled();
                                value.tools = tested_tools();
                                let id = value.id.clone();
                                mutate(|data| {
                                    if let Some(existing) = data.mcp_servers.iter_mut().find(|item| item.id == id) {
                                        existing.name = value.name.clone();
                                        existing.url = value.url.clone();
                                        existing.api_key = value.api_key.clone();
                                        existing.enabled = value.enabled;
                                        existing.tools = value.tools.clone();
                                    } else {
                                        data.mcp_servers.push(value.clone());
                                    }
                                });
                                persist();
                                editing.set(None);
                                toast(i18n::t("mcp.saved"));
                            },
                            {i18n::t("common.save")}
                        }
                    }
                }
                }
                }
            }

        }
    }
}
