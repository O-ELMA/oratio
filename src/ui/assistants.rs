use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::*, Icon};
use oratio::i18n;
use oratio::store::{self, Assistant};

use crate::app::{self, back, delete_assistant, navigate, save_assistant, toast, Screen, DATA};
use crate::ui::common::{
    assistant_color, Avatar, ConfirmButton, EmptyState, ModelPickerSheet, PageHeader, Sheet, Toggle,
};

#[component]
pub fn AssistantsPage() -> Element {
    let data = DATA();
    let assistants = data.assistants.clone();
    let current_id = data.settings.assistant_id.clone();
    let mut query = use_signal(String::new);
    let needle = query().to_lowercase();
    let filtered: Vec<Assistant> = assistants
        .iter()
        .filter(|assistant| {
            needle.is_empty() || assistant.display_name().to_lowercase().contains(&needle)
        })
        .cloned()
        .collect();

    rsx! {
        div { class: "screen",
            PageHeader { title: i18n::t("assistants.title"), subtitle: i18n::t("assistants.subtitle"),
                button {
                    class: "icon-btn",
                    onclick: move |_| navigate(Screen::AssistantEdit(None)),
                    Icon { icon: LdPlus, width: 20, height: 20, fill: "currentColor" }
                }
            }
            div { class: "content",
                input {
                    class: "input search-input",
                    placeholder: i18n::t("assistants.search"),
                    initial_value: "{query}",
                    oninput: move |event| query.set(event.value()),
                }
                if filtered.is_empty() {
                    EmptyState {
                        title: i18n::t("assistants.emptyTitle"),
                        message: i18n::t("assistants.emptyMessage"),
                        button {
                            class: "btn full",
                            onclick: move |_| navigate(Screen::AssistantEdit(None)),
                            {i18n::t("common.create")}
                        }
                    }
                } else {
                    for assistant in filtered {
                        {
                            let is_current = assistant.id == current_id;
                            let provider = data.provider(&assistant.provider_id);
                            let model_label = if assistant.model.is_empty() {
                                i18n::t("assistants.defaultModel")
                            } else {
                                assistant.model.clone()
                            };
                            let provider_label = provider
                                .map(|provider| provider.name.clone())
                                .unwrap_or_else(|| i18n::t("common.noProvider"));
                            let open_id = assistant.id.clone();
                            let edit_id = assistant.id.clone();
                            let remove_id = assistant.id.clone();
                            rsx! {
                                div {
                                    key: "{assistant.id}",
                                    class: "assistant-card",
                                    onclick: move |_| navigate(Screen::AssistantEdit(Some(open_id.clone()))),
                                    Avatar { name: assistant.display_name(), color: assistant.color(), size: 48 }
                                    div { class: "assistant-card-text",
                                        div { class: "assistant-card-title",
                                            span { "{assistant.display_name()}" }
                                            if is_current {
                                                span { class: "badge primary", {i18n::t("common.current")} }
                                            }
                                        }
                                        div { class: "assistant-card-sub", "{provider_label} · {model_label}" }
                                        div { class: "assistant-card-prompt",
                                            if assistant.system_prompt.is_empty() {
                                                {i18n::t("chat.noSystemPrompt")}
                                            } else {
                                                "{assistant.system_prompt}"
                                            }
                                        }
                                    }
                                    div { class: "card-actions",
                                        button {
                                            class: "icon-btn small",
                                            onclick: move |event| {
                                                event.stop_propagation();
                                                navigate(Screen::AssistantEdit(Some(edit_id.clone())));
                                            },
                                            Icon { icon: LdPencil, width: 16, height: 16, fill: "currentColor" }
                                        }
                                        ConfirmButton {
                                            class: "icon-btn small danger-text confirm-icon",
                                            title: i18n::t("common.delete"),
                                            onconfirm: move |_| {
                                                delete_assistant(&remove_id);
                                                toast(i18n::t("assistants.deleted"));
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

        }
    }
}

#[component]
pub fn AssistantEditPage(assistant_id: Option<String>) -> Element {
    let data = DATA();
    let existing = assistant_id
        .as_deref()
        .and_then(|id| data.assistant(id))
        .cloned();
    let is_new = existing.is_none();
    let base = existing.clone().unwrap_or_else(|| Assistant {
        id: store::new_id(),
        name: String::new(),
        system_prompt: String::new(),
        provider_id: String::new(),
        model: String::new(),
        avatar_color: assistant_color("assistant").to_string(),
        use_assistant_avatar: true,
        use_gradient_background: false,
        enable_web_search: false,
        skill_ids: Vec::new(),
    });
    let (initial_provider, initial_model) = if base.provider_id.is_empty() && base.model.is_empty()
    {
        data.model_for_assistant(&base)
    } else {
        (base.provider_id.clone(), base.model.clone())
    };

    let name0 = base.name.clone();
    let prompt0 = base.system_prompt.clone();
    let color0 = base.color();
    let use_avatar0 = base.use_assistant_avatar;
    let gradient0 = base.use_gradient_background;
    let web_search0 = base.enable_web_search;
    let skill_ids0 = base.skill_ids.clone();

    let mut name = use_signal(move || name0);
    let mut prompt = use_signal(move || prompt0);
    let mut color = use_signal(move || color0);
    let mut use_avatar = use_signal(move || use_avatar0);
    let mut gradient = use_signal(move || gradient0);
    let mut web_search = use_signal(move || web_search0);
    let mut skill_ids = use_signal(move || skill_ids0);
    let mut provider_id = use_signal(move || initial_provider);
    let mut model = use_signal(move || initial_model);
    let mut model_open = use_signal(|| false);

    let base_for_save = base.clone();
    let base_for_delete = base.clone();
    let can_delete = !is_new && data.assistants.len() > 1;
    let provider_name = data
        .provider(&provider_id())
        .map(|provider| provider.name.clone())
        .unwrap_or_else(|| i18n::t("common.noProvider"));

    rsx! {
        div { class: "screen",
            PageHeader {
                title: if is_new { i18n::t("assistants.new") } else { i18n::t("assistants.edit") },
                button {
                    class: "icon-btn",
                    onclick: move |_| {
                        let value = name().trim().to_string();
                        let mut assistant = base_for_save.clone();
                        assistant.name = value;
                        assistant.system_prompt = prompt().trim().to_string();
                        assistant.avatar_color = color();
                        assistant.provider_id = provider_id();
                        assistant.model = model();
                        assistant.use_assistant_avatar = use_avatar();
                        assistant.use_gradient_background = gradient();
                        assistant.enable_web_search = web_search();
                        assistant.skill_ids = skill_ids();
                        save_assistant(assistant.clone());
                        if is_new {
                            let mut data = DATA.signal();
                            data.write().settings.assistant_id = assistant.id.clone();
                            app::persist();
                            app::sync_draft(&assistant.id);
                        }
                        toast(i18n::t("assistants.saved"));
                        back();
                    },
                    Icon { icon: LdCheck, width: 20, height: 20, fill: "currentColor" }
                }
            }
            div { class: "content",
                div { class: "card-group",
                    div { class: "assistant-compact",
                        Avatar {
                            name: if name().is_empty() { i18n::t("common.defaultAssistant") } else { name() },
                            color: color(),
                            size: 64
                        }
                        div { class: "assistant-compact-name",
                            label { {i18n::t("assistants.name")} }
                            input {
                                class: "input",
                                placeholder: i18n::t("common.defaultAssistant"),
                                initial_value: "{name}",
                                oninput: move |event| name.set(event.value()),
                            }
                        }
                    }
                    div { class: "color-row compact-colors",
                        for option in ["#8E4955", "#116682", "#4C662B", "#735C0C", "#6A4C93", "#A64B2A", "#3D6B5C", "#7A3E65"] {
                            button {
                                class: if color() == option { "color-dot selected" } else { "color-dot" },
                                style: "background:{option};",
                                onclick: move |_| color.set(option.to_string()),
                            }
                        }
                    }
                    button {
                        class: "setting-row",
                        onclick: move |_| model_open.set(true),
                        div { class: "row-icon secondary",
                            Icon { icon: LdSparkles, width: 18, height: 18, fill: "currentColor" }
                        }
                        div { class: "setting-row-text",
                            span { {i18n::t("assistants.defaultModel")} }
                            small { "{provider_name} · {model}" }
                        }
                        Icon { icon: LdChevronRight, width: 18, height: 18, fill: "currentColor" }
                    }
                    div { class: "field",
                        label { {i18n::t("chat.systemPrompt")} }
                        textarea {
                            class: "input prompt-input assistant-prompt",
                            rows: "3",
                            placeholder: i18n::t("assistants.promptPlaceholder"),
                            initial_value: "{prompt}",
                            oninput: move |event| prompt.set(event.value()),
                        }
                    }
                    div { class: "setting-row static compact",
                        div { class: "row-icon tertiary",
                            Icon { icon: LdBrain, width: 18, height: 18, fill: "currentColor" }
                        }
                        div { class: "setting-row-text",
                            span { {i18n::t("assistants.useAvatar")} }
                            small { {i18n::t("assistants.useAvatarNote")} }
                        }
                        Toggle { checked: use_avatar(), onchange: move |value| use_avatar.set(value) }
                    }
                    div { class: "setting-row static compact",
                        div { class: "row-icon",
                            Icon { icon: LdPalette, width: 18, height: 18, fill: "currentColor" }
                        }
                        div { class: "setting-row-text",
                            span { {i18n::t("assistants.gradient")} }
                            small { {i18n::t("assistants.gradientNote")} }
                        }
                        Toggle { checked: gradient(), onchange: move |value| gradient.set(value) }
                    }
                    div { class: "setting-row static compact",
                        div { class: "row-icon secondary",
                            Icon { icon: LdGlobe, width: 18, height: 18, fill: "currentColor" }
                        }
                        div { class: "setting-row-text",
                            span { {i18n::t("chat.webSearch")} }
                            small { {i18n::t("assistants.webSearchNote")} }
                        }
                        Toggle { checked: web_search(), onchange: move |value| web_search.set(value) }
                    }
                }

                div { class: "card-group",
                    div { class: "card-group-head",
                        h2 { {i18n::t("extensions.skills")} }
                        p { {i18n::t("assistants.skillsNote")} }
                    }
                    if data.skills.is_empty() {
                        div { class: "muted-note", {i18n::t("assistants.noSkills")} }
                        button {
                            class: "btn ghost full",
                            onclick: move |_| navigate(Screen::Skills),
                            {i18n::t("assistants.openSkills")}
                        }
                    } else {
                        div { class: "skills-scroll",
                        for skill in data.skills.iter() {
                            {
                                let skill = skill.clone();
                                let checked = skill_ids().contains(&skill.id);
                                rsx! {
                                    div { key: "{skill.id}", class: "setting-row static compact",
                                        div { class: "row-icon tertiary",
                                            Icon { icon: LdWrench, width: 18, height: 18, fill: "currentColor" }
                                        }
                                        div { class: "setting-row-text",
                                            span { "{skill.name}" }
                                            small { "{skill.description}" }
                                        }
                                        Toggle {
                                            checked,
                                            onchange: move |value| {
                                                let mut ids = skill_ids();
                                                if value {
                                                    if !ids.contains(&skill.id) { ids.push(skill.id.clone()); }
                                                } else {
                                                    ids.retain(|id| id != &skill.id);
                                                }
                                                skill_ids.set(ids);
                                            },
                                        }
                                    }
                                }
                            }
                        }
                        }
                    }
                }

                if can_delete {
                    ConfirmButton {
                        class: "btn ghost danger-text full",
                        text: i18n::t("common.delete"),
                        onconfirm: move |_| {
                            delete_assistant(&base_for_delete.id);
                            toast(i18n::t("assistants.deleted"));
                            navigate(Screen::Assistants);
                        },
                    }
                }
            }

            if model_open() {
                ModelPickerSheet {
                    current_provider: provider_id(),
                    current_model: model(),
                    on_close: move |_| model_open.set(false),
                    on_select: move |value: (String, String)| {
                        provider_id.set(value.0);
                        model.set(value.1);
                    },
                }
            }

        }
    }
}

#[component]
pub fn AssistantPickerSheet(
    current_id: String,
    on_close: EventHandler<()>,
    on_select: EventHandler<String>,
) -> Element {
    let data = DATA();
    let assistants = data.assistants.clone();
    rsx! {
        Sheet { title: i18n::t("drawer.assistants"), on_close,
            div { class: "sheet-list",
                for assistant in assistants {
                    {
                        let selected = assistant.id == current_id;
                        let id = assistant.id.clone();
                        rsx! {
                            button {
                                key: "{assistant.id}",
                                class: if selected { "assistant-row selected" } else { "assistant-row" },
                                onclick: move |_| on_select.call(id.clone()),
                                Avatar { name: assistant.display_name(), color: assistant.color(), size: 40 }
                                div { class: "assistant-row-text",
                                    span { "{assistant.display_name()}" }
                                    small {
                                        if assistant.system_prompt.is_empty() {
                                            if assistant.model.is_empty() { {i18n::t("assistants.noPromptDefaultModel")} } else { "{assistant.model}" }
                                        } else {
                                            {i18n::t("assistants.customPrompt")}
                                        }
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
                        on_close.call(());
                        navigate(Screen::AssistantEdit(None));
                    },
                    div { class: "row-icon", Icon { icon: LdPlus, width: 18, height: 18, fill: "currentColor" } }
                    div { class: "sheet-row-text", span { {i18n::t("common.create")} } }
                }
            }
        }
    }
}
