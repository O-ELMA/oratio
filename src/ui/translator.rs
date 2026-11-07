use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::*, Icon};
use oratio::{
    api::{self, ApiMessage, Delta},
    i18n, markdown,
};

use crate::app::{toast, DATA};
use crate::ui::common::{copy_text, ModelPickerSheet, SectionCard};

const LANGUAGES: &[&str] = &[
    "Afrikaans",
    "Albanian",
    "Amharic",
    "Arabic",
    "Armenian",
    "Azerbaijani",
    "Basque",
    "Belarusian",
    "Bengali",
    "Bosnian",
    "Bulgarian",
    "Burmese",
    "Catalan",
    "Cebuano",
    "Chinese (Simplified)",
    "Chinese (Traditional)",
    "Croatian",
    "Czech",
    "Danish",
    "Dutch",
    "English",
    "Esperanto",
    "Estonian",
    "Filipino",
    "Finnish",
    "French",
    "Galician",
    "Georgian",
    "German",
    "Greek",
    "Gujarati",
    "Haitian Creole",
    "Hausa",
    "Hebrew",
    "Hindi",
    "Hmong",
    "Hungarian",
    "Icelandic",
    "Igbo",
    "Indonesian",
    "Irish",
    "Italian",
    "Japanese",
    "Javanese",
    "Kannada",
    "Kazakh",
    "Khmer",
    "Korean",
    "Kurdish",
    "Kyrgyz",
    "Lao",
    "Latin",
    "Latvian",
    "Lithuanian",
    "Luxembourgish",
    "Macedonian",
    "Malagasy",
    "Malay",
    "Malayalam",
    "Maltese",
    "Maori",
    "Marathi",
    "Mongolian",
    "Nepali",
    "Norwegian",
    "Pashto",
    "Persian",
    "Polish",
    "Portuguese",
    "Punjabi",
    "Romanian",
    "Russian",
    "Samoan",
    "Serbian",
    "Sesotho",
    "Shona",
    "Sindhi",
    "Sinhala",
    "Slovak",
    "Slovenian",
    "Somali",
    "Spanish",
    "Sundanese",
    "Swahili",
    "Swedish",
    "Tajik",
    "Tamil",
    "Tatar",
    "Telugu",
    "Thai",
    "Turkish",
    "Turkmen",
    "Ukrainian",
    "Urdu",
    "Uyghur",
    "Uzbek",
    "Vietnamese",
    "Welsh",
    "Xhosa",
    "Yiddish",
    "Yoruba",
    "Zulu",
];

fn extract_translations(raw: &str, requested: &[String]) -> Vec<(String, String)> {
    let Some(start) = raw.find('{') else {
        return Vec::new();
    };
    let Some(end) = raw.rfind('}') else {
        return Vec::new();
    };
    if end <= start {
        return Vec::new();
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw[start..=end]) else {
        return Vec::new();
    };
    let Some(items) = value.get("translations").and_then(|value| value.as_array()) else {
        return Vec::new();
    };
    let mut found: Vec<(String, String)> = Vec::new();
    for item in items {
        let language = item
            .get("language")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let text = item
            .get("text")
            .or_else(|| item.get("translation"))
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if language.trim().is_empty() || text.trim().is_empty() {
            continue;
        }
        found.push((language.to_string(), text.to_string()));
    }
    if found.is_empty() {
        return Vec::new();
    }
    let mut ordered: Vec<(String, String)> = Vec::new();
    for target in requested {
        match found
            .iter()
            .find(|(language, _)| language.eq_ignore_ascii_case(target))
        {
            Some((_, text)) => ordered.push((target.clone(), text.clone())),
            None => ordered.push((target.clone(), i18n::t("translator.noTranslation"))),
        }
    }
    for (language, text) in found {
        if !ordered
            .iter()
            .any(|(existing, _)| existing.eq_ignore_ascii_case(&language))
        {
            ordered.push((language, text));
        }
    }
    ordered
}

#[component]
pub fn TranslatorPanel() -> Element {
    let data = DATA();
    let default_provider = if !data.settings.default_provider_id.is_empty() {
        data.settings.default_provider_id.clone()
    } else {
        data.providers
            .first()
            .map(|provider| provider.id.clone())
            .unwrap_or_default()
    };
    let default_model = if !data.settings.default_model.is_empty() {
        data.settings.default_model.clone()
    } else {
        data.providers
            .first()
            .and_then(|provider| provider.models.first().cloned())
            .unwrap_or_default()
    };
    let mut input = use_signal(String::new);
    let mut selected = use_signal(Vec::<String>::new);
    let mut search = use_signal(String::new);
    let mut picker_open = use_signal(|| false);
    let results = use_signal(Vec::<(String, String)>::new);
    let error = use_signal(String::new);
    let mut provider_id = use_signal(move || default_provider);
    let mut model = use_signal(move || default_model);
    let mut model_open = use_signal(|| false);
    let loading = use_signal(|| false);
    let provider_name = data
        .provider(&provider_id())
        .map(|provider| provider.name.clone())
        .unwrap_or_else(|| i18n::t("common.noProvider"));
    let needle = search().trim().to_lowercase();
    let filtered: Vec<&str> = LANGUAGES
        .iter()
        .copied()
        .filter(|language| needle.is_empty() || language.to_lowercase().contains(&needle))
        .collect();

    rsx! {
        SectionCard { title: i18n::t("translator.text"), subtitle: i18n::t("translator.textSub"),
            textarea {
                class: "input prompt-input",
                rows: "5",
                placeholder: i18n::t("translator.placeholder"),
                initial_value: "{input}",
                oninput: move |event| input.set(event.value()),
            }
            div { class: "field",
                label { {i18n::t("translator.translateTo")} }
                if !selected().is_empty() {
                    div { class: "selected-pills",
                        for language in selected() {
                            {
                                let value = language.clone();
                                rsx! {
                                    button {
                                        key: "{language}",
                                        class: "selected-pill",
                                        onclick: move |_| {
                                            let mut list = selected();
                                            list.retain(|item| item != &value);
                                            selected.set(list);
                                        },
                                        "{language}"
                                        Icon { icon: LdX, width: 12, height: 12, fill: "currentColor" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "language-combo",
                button {
                    class: "setting-row",
                    onclick: move |_| picker_open.set(!picker_open()),
                    div { class: "row-icon primary",
                        Icon { icon: LdLanguages, width: 18, height: 18, fill: "currentColor" }
                    }
                    div { class: "setting-row-text",
                        span { {i18n::t("translator.languages")} }
                        small {
                            if selected().is_empty() {
                                {i18n::t("translator.chooseLanguages")}
                            } else {
                                {i18n::ta("common.selected", &[("count", &selected().len().to_string())])}
                            }
                        }
                    }
                    Icon { icon: LdChevronDown, width: 18, height: 18, fill: "currentColor" }
                }
                if picker_open() {
                    div { class: "language-picker",
                        div { class: "language-search",
                            Icon { icon: LdSearch, width: 16, height: 16, fill: "currentColor" }
                            input {
                                class: "input",
                                placeholder: i18n::t("translator.searchLanguages"),
                                autocapitalize: "off",
                                autocorrect: "off",
                                spellcheck: "false",
                                initial_value: "{search}",
                                oninput: move |event| search.set(event.value()),
                            }
                        }
                        div { class: "language-list",
                            if filtered.is_empty() {
                                div { class: "muted-note", {i18n::t("translator.noLanguageMatch")} }
                            }
                            for language in filtered {
                                {
                                    let value = language.to_string();
                                    let active = selected().iter().any(|item| item == &value);
                                    let click_value = value.clone();
                                    rsx! {
                                        button {
                                            key: "{value}",
                                            class: if active { "language-option selected" } else { "language-option" },
                                            onclick: move |_| {
                                                let mut list = selected();
                                                match list.iter().position(|item| item == &click_value) {
                                                    Some(index) => {
                                                        list.remove(index);
                                                    }
                                                    None => list.push(click_value.clone()),
                                                }
                                                selected.set(list);
                                            },
                                            span { "{value}" }
                                            if active {
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
            button {
                class: "setting-row",
                onclick: move |_| model_open.set(true),
                div { class: "row-icon primary",
                    Icon { icon: LdSparkles, width: 18, height: 18, fill: "currentColor" }
                }
                div { class: "setting-row-text",
                    span { {i18n::t("chat.model")} }
                    small { "{provider_name} · {model}" }
                }
                Icon { icon: LdChevronRight, width: 18, height: 18, fill: "currentColor" }
            }
            button {
                class: "btn full",
                disabled: loading(),
                onclick: move |_| {
                    let text = input().trim().to_string();
                    if text.is_empty() {
                        toast(i18n::t("translator.needText"));
                        return;
                    }
                    let targets = selected();
                    if targets.is_empty() {
                        toast(i18n::t("translator.needLanguage"));
                        return;
                    }
                    if provider_id().is_empty() || model().is_empty() {
                        toast(i18n::t("translator.needModel"));
                        return;
                    }
                    let provider = DATA
                        .peek()
                        .provider(&provider_id())
                        .cloned();
                    let Some(provider) = provider else {
                        toast(i18n::t("chat.providerNotFound"));
                        return;
                    };
                    let model_value = model();
                    let target_list = targets.join(", ");
                    let system = format!(
                        "You are a precise translator. Translate the user's text into every language in this list: {target_list}. \
    Return only a JSON object, with no prose and no code fences, in exactly this shape: \
    {{\"translations\":[{{\"language\":\"<language>\",\"text\":\"<translation>\"}}]}}. \
    Use the language names exactly as given. Keep Markdown formatting inside each text value."
                    );
                    let mut results_signal = results;
                    let mut error_signal = error;
                    let mut loading_signal = loading;
                    results_signal.set(Vec::new());
                    error_signal.set(String::new());
                    loading_signal.set(true);
                    dioxus::prelude::spawn(async move {
                        let mut buffer = String::new();
                        let messages = vec![
                            ApiMessage {
                                role: "system".into(),
                                content: system,
                                ..Default::default()
                            },
                            ApiMessage {
                                role: "user".into(),
                                content: text,
                                ..Default::default()
                            },
                        ];
                        let outcome = api::stream_chat(
                            &provider.base_url,
                            &provider.api_key,
                            &model_value,
                            &messages,
                            |delta| {
                                if let Delta::Content(text) = delta {
                                    buffer.push_str(&text);
                                }
                                true
                            },
                        )
                        .await;
                        match outcome {
                            Ok(()) => {
                                let parsed = extract_translations(&buffer, &targets);
                                if parsed.is_empty() {
                                    error_signal.set(
                                        i18n::t("translator.badFormat"),
                                    );
                                } else {
                                    results_signal.set(parsed);
                                }
                            }
                            Err(message) => {
                                error_signal.set(i18n::ta("translator.failed", &[("error", &message)]))
                            }
                        }
                        loading_signal.set(false);
                    });
                },
                if loading() { {i18n::t("translator.translating")} } else { {i18n::t("translator.translate")} }
            }
            if !error().is_empty() {
                div { class: "errorbar", span { class: "error-text", "{error}" } }
            }
        }

        if !results().is_empty() {
            for (language, text) in results() {
                {
                    let copy_value = text.clone();
                    let html = markdown::to_html(&text);
                    rsx! {
                        div { key: "{language}", class: "card-group translation-result",
                            div { class: "card-group-head translation-head",
                                h2 { "{language}" }
                                button {
                                    class: "icon-btn small",
                                    title: i18n::t("translator.copyTranslation"),
                                    onclick: move |_| copy_text(&copy_value),
                                    Icon { icon: LdCopy, width: 16, height: 16, fill: "currentColor" }
                                }
                            }
                            div { class: "md", dangerous_inner_html: html }
                        }
                    }
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
