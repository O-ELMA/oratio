use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::*, Icon};
use oratio::{
    api, i18n,
    store::{self, Provider},
};

use crate::app::{back, delete_provider, navigate, save_provider, toast, Screen, DATA};
use crate::ui::common::{ConfirmButton, EmptyState, PageHeader};

fn compose_url(fallback_scheme: &str, host: &str) -> String {
    let host = host.trim();
    let (scheme, host) = if let Some(rest) = host.strip_prefix("https://") {
        ("https://", rest)
    } else if let Some(rest) = host.strip_prefix("http://") {
        ("http://", rest)
    } else {
        (fallback_scheme, host)
    };
    let host = host
        .trim_end_matches('/')
        .trim_end_matches("/chat/completions")
        .trim_end_matches('/');
    format!("{scheme}{host}")
}

fn model_list(models: &[String]) -> Vec<String> {
    let mut list = Vec::new();
    for model in models {
        let model = model.trim();
        if !model.is_empty() && !list.iter().any(|item| item == model) {
            list.push(model.to_string());
        }
    }
    list
}

#[component]
pub fn ProvidersPage() -> Element {
    let data = DATA();
    let providers = data.providers.clone();
    let chats_using = |id: &str| {
        data.chats
            .iter()
            .filter(|chat| chat.provider_id == id)
            .count()
    };

    rsx! {
        div { class: "screen",
            PageHeader { title: i18n::t("providers.title"),
                button {
                    class: "icon-btn",
                    onclick: move |_| navigate(Screen::ProviderEdit(None)),
                    Icon { icon: LdPlus, width: 20, height: 20, fill: "currentColor" }
                }
            }
            div { class: "content",
                if providers.is_empty() {
                    EmptyState {
                        title: i18n::t("providers.emptyTitle"),
                        message: i18n::t("providers.emptyMessage"),
                        button {
                            class: "btn full",
                            onclick: move |_| navigate(Screen::ProviderEdit(None)),
                            {i18n::t("common.create")}
                        }
                    }
                } else {
                    for provider in providers {
                        {
                            let id = provider.id.clone();
                            let open_id = id.clone();
                            let edit_id = id.clone();
                            let remove_id = id.clone();
                            let builtin = store::is_builtin_provider(&provider.id);
                            let initial = provider.name.chars().next().unwrap_or('P').to_uppercase().to_string();
                            let count = chats_using(&provider.id);
                            rsx! {
                                div {
                                    key: "{provider.id}",
                                    class: "provider-card",
                                    onclick: move |_| navigate(Screen::ProviderEdit(Some(open_id.clone()))),
                                    div { class: "provider-logo", "{initial}" }
                                    div { class: "provider-card-text",
                                        div { class: "provider-card-title",
                                            span { "{provider.name}" }
                                            if builtin {
                                                span { class: "badge primary", {i18n::t("providers.builtIn")} }
                                            }
                                        }
                                        div { class: "provider-card-url", "{provider.base_url}" }
                                        div { class: "provider-card-meta",
                                            {i18n::tn("common.models", provider.models.len())}
                                            if count > 0 {
                                                " · "
                                                {i18n::tn("common.chats", count)}
                                            }
                                        }
                                    }
                                    div { class: "card-actions",
                                        button {
                                            class: "icon-btn small",
                                            onclick: move |event| {
                                                event.stop_propagation();
                                                navigate(Screen::ProviderEdit(Some(edit_id.clone())));
                                            },
                                            Icon { icon: LdPencil, width: 16, height: 16, fill: "currentColor" }
                                        }
                                        if !builtin {
                                            ConfirmButton {
                                                class: "icon-btn small danger-text confirm-icon",
                                                title: i18n::t("common.delete"),
                                                onconfirm: move |_| {
                                                    delete_provider(&remove_id);
                                                    toast(i18n::t("providers.deleted"));
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
}

#[component]
pub fn ProviderEditPage(provider_id: Option<String>) -> Element {
    let data = DATA();
    let existing = provider_id
        .as_deref()
        .and_then(|id| data.provider(id))
        .cloned();
    let is_new = existing.is_none();
    let base = existing.unwrap_or_default();
    let id = if base.id.is_empty() {
        store::new_id()
    } else {
        base.id.clone()
    };
    let (scheme, host) = if let Some(rest) = base.base_url.strip_prefix("http://") {
        ("http://", rest.to_string())
    } else if let Some(rest) = base.base_url.strip_prefix("https://") {
        ("https://", rest.to_string())
    } else {
        ("https://", base.base_url.clone())
    };
    let host = host
        .trim_end_matches('/')
        .trim_end_matches("/chat/completions")
        .trim_end_matches('/')
        .to_string();
    let name0 = base.name.clone();
    let key0 = base.api_key.clone();
    let models0 = if base.models.is_empty() {
        vec![String::new()]
    } else {
        base.models.clone()
    };
    let mut name = use_signal(move || name0);
    let mut base_url = use_signal(move || host);
    let mut api_key = use_signal(move || key0);
    let mut models = use_signal(move || models0);
    let mut model_revision = use_signal(|| 0u32);
    let mut show_key = use_signal(|| false);
    let mut status = use_signal(String::new);
    let mut busy = use_signal(|| false);

    let id_for_save = id.clone();

    rsx! {
        div { class: "screen",
            PageHeader {
                title: if is_new { i18n::t("providers.addTitle") } else { i18n::t("providers.editTitle") },
                button {
                    class: "icon-btn",
                    onclick: move |_| {
                        let name_value = name().trim().to_string();
                        let base_value = compose_url(scheme, &base_url());
                        if name_value.is_empty() {
                            status.set(i18n::t("providers.needName"));
                            return;
                        }
                        if base_value == scheme {
                            status.set(i18n::t("providers.needHost"));
                            return;
                        }
                        let provider = Provider {
                            id: id_for_save.clone(),
                            name: name_value,
                            base_url: base_value,
                            api_key: api_key().trim().to_string(),
                            models: model_list(&models()),
                        };
                        save_provider(provider);
                        toast(i18n::t("providers.saved"));
                        back();
                    },
                    Icon { icon: LdCheck, width: 20, height: 20, fill: "currentColor" }
                }
            }
            div { class: "content",
                div { class: "card-group",
                    div { class: "field",
                        label { {i18n::t("providers.name")} }
                        input {
                            class: "input",
                            placeholder: i18n::t("providers.namePlaceholder"),
                            initial_value: "{name}",
                            oninput: move |event| name.set(event.value()),
                        }
                    }
                    div { class: "field",
                        label { {i18n::t("providers.baseUrl")} }
                        div { class: "url-wrap",
                            span { class: "url-fixed", "{scheme}" }
                            input {
                                class: "url-input",
                                placeholder: "api.openai.com/v1",
                                autocapitalize: "off",
                                autocorrect: "off",
                                spellcheck: "false",
                                initial_value: "{base_url}",
                                oninput: move |event| base_url.set(event.value()),
                            }
                            span { class: "url-fixed", "/chat/completions" }
                        }
                        p { class: "hint", {i18n::t("providers.urlHint")} }
                    }
                    div { class: "field",
                        label { {i18n::t("providers.apiKey")} }
                        div { class: "key-row",
                            input {
                                class: "input",
                                r#type: if show_key() { "text" } else { "password" },
                                placeholder: i18n::t("providers.keyPlaceholder"),
                                autocapitalize: "off",
                                autocorrect: "off",
                                spellcheck: "false",
                                initial_value: "{api_key}",
                                oninput: move |event| api_key.set(event.value()),
                            }
                            button {
                                class: "icon-btn small",
                                title: if show_key() { i18n::t("providers.hideKey") } else { i18n::t("providers.showKey") },
                                onclick: move |_| show_key.set(!show_key()),
                                if show_key() {
                                    Icon { icon: LdEyeOff, width: 18, height: 18, fill: "currentColor" }
                                } else {
                                    Icon { icon: LdEye, width: 18, height: 18, fill: "currentColor" }
                                }
                            }
                        }
                    }
                    div { class: "field",
                        label { {i18n::t("providers.models")} }
                        div { class: "model-inputs",
                            for (index, value) in models().iter().enumerate() {
                                {
                                    let current = value.clone();
                                    rsx! {
                                        div {
                                            key: "{model_revision()}-{index}",
                                            class: "model-input-row",
                                            input {
                                                class: "input model-input",
                                                placeholder: i18n::t("providers.modelId"),
                                                autocapitalize: "off",
                                                autocorrect: "off",
                                                spellcheck: "false",
                                                initial_value: "{current}",
                                                oninput: move |event| {
                                                    let mut list = models();
                                                    if let Some(slot) = list.get_mut(index) {
                                                        *slot = event.value();
                                                    }
                                                    models.set(list);
                                                },
                                            }
                                            button {
                                                class: "icon-btn small danger-text",
                                                title: i18n::t("providers.deleteModel"),
                                                onclick: move |_| {
                                                    let mut list = models();
                                                    if index < list.len() {
                                                        list.remove(index);
                                                        models.set(list);
                                                        model_revision.set(model_revision() + 1);
                                                    }
                                                },
                                                Icon { icon: LdTrash2, width: 16, height: 16, fill: "currentColor" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        button {
                            class: "btn ghost full",
                            title: i18n::t("providers.addModel"),
                            onclick: move |_| {
                                let mut list = models();
                                list.push(String::new());
                                models.set(list);
                                model_revision.set(model_revision() + 1);
                            },
                            Icon { icon: LdPlus, width: 20, height: 20, fill: "currentColor" }
                        }
                    }
                    div { class: "form-actions",
                        button {
                            class: "btn ghost",
                            disabled: busy(),
                            onclick: move |_| {
                                let url = compose_url(scheme, &base_url());
                                let key = api_key().trim().to_string();
                                if url == scheme {
                                    status.set(i18n::t("providers.enterHost"));
                                    return;
                                }
                                busy.set(true);
                                status.set(i18n::t("providers.fetchingModels"));
                                let mut models_signal = models;
                                let mut revision_signal = model_revision;
                                let mut status_signal = status;
                                let mut busy_signal = busy;
                                dioxus::core::spawn_forever(async move {
                                    match api::fetch_models(&url, &key).await {
                                        Ok(found) if found.is_empty() => {
                                            status_signal.set(i18n::t("providers.noModels"));
                                        }
                                        Ok(found) => {
                                            models_signal.set(found.clone());
                                            revision_signal.set(revision_signal() + 1);
                                            status_signal.set(i18n::ta("providers.fetchedModels", &[("count", &found.len().to_string())]));
                                        }
                                        Err(error) => status_signal.set(i18n::ta("providers.fetchFailed", &[("error", &error)])),
                                    }
                                    busy_signal.set(false);
                                });
                            },
                            if busy() { {i18n::t("providers.fetching")} } else { {i18n::t("providers.fetchModels")} }
                        }
                        button {
                            class: "btn ghost",
                            disabled: busy(),
                            onclick: move |_| {
                                let url = compose_url(scheme, &base_url());
                                let key = api_key().trim().to_string();
                                if url == scheme {
                                    status.set(i18n::t("providers.enterHost"));
                                    return;
                                }
                                busy.set(true);
                                status.set(i18n::t("providers.testingConnection"));
                                let mut status_signal = status;
                                let mut busy_signal = busy;
                                dioxus::core::spawn_forever(async move {
                                    match api::fetch_models(&url, &key).await {
                                        Ok(models) => status_signal.set(i18n::ta(
                                            "providers.connected",
                                            &[("count", &models.len().to_string())],
                                        )),
                                        Err(error) => status_signal.set(i18n::ta("providers.connectionFailed", &[("error", &error)])),
                                    }
                                    busy_signal.set(false);
                                });
                            },
                            {i18n::t("providers.testConnection")}
                        }
                    }
                    if !status().is_empty() {
                        div { class: "hint status-text", "{status}" }
                    }
                }
            }

        }
    }
}
