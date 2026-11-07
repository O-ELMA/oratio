use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::*, Icon};
use oratio::{api, i18n};

use crate::app::{toast, DATA};
use crate::ui::common::{copy_text, ModelPickerSheet, SectionCard};

#[component]
pub fn MediaPanel() -> Element {
    let data = DATA();
    let default_provider = if !data.settings.media_provider_id.is_empty() {
        data.settings.media_provider_id.clone()
    } else if !data.settings.default_provider_id.is_empty() {
        data.settings.default_provider_id.clone()
    } else {
        data.providers
            .first()
            .map(|provider| provider.id.clone())
            .unwrap_or_default()
    };
    let default_model = if !data.settings.media_model.is_empty() {
        data.settings.media_model.clone()
    } else {
        data.providers
            .iter()
            .find(|provider| provider.id == default_provider)
            .and_then(|provider| {
                provider
                    .models
                    .iter()
                    .find(|model| {
                        let lower = model.to_lowercase();
                        lower.contains("image") || lower.contains("dall") || lower.contains("flux")
                    })
                    .or_else(|| provider.models.first())
                    .cloned()
            })
            .unwrap_or_default()
    };

    let mut prompt = use_signal(String::new);
    let mut provider_id = use_signal(move || default_provider);
    let mut model = use_signal(move || default_model);
    let mut size = use_signal(|| "1024x1024".to_string());
    let mut model_open = use_signal(|| false);
    let loading = use_signal(|| false);
    let error = use_signal(String::new);
    let images = use_signal(Vec::<String>::new);
    let provider_name = data
        .provider(&provider_id())
        .map(|provider| provider.name.clone())
        .unwrap_or_else(|| i18n::t("common.noProvider"));

    rsx! {
        SectionCard { title: i18n::t("media.prompt"), subtitle: i18n::t("media.promptSub"),
            textarea {
                class: "input prompt-input",
                rows: "5",
                placeholder: i18n::t("media.promptPlaceholder"),
                initial_value: "{prompt}",
                oninput: move |event| prompt.set(event.value()),
            }
            button {
                class: "setting-row",
                onclick: move |_| model_open.set(true),
                div { class: "row-icon tertiary",
                    Icon { icon: LdImage, width: 18, height: 18, fill: "currentColor" }
                }
                div { class: "setting-row-text",
                    span { {i18n::t("media.imageModel")} }
                    small { if model().is_empty() { {i18n::t("media.chooseModel")} } else { "{provider_name} · {model}" } }
                }
                Icon { icon: LdChevronRight, width: 18, height: 18, fill: "currentColor" }
            }
            div { class: "field",
                label { {i18n::t("media.size")} }
                div { class: "chip-row",
                    for option in ["1024x1024", "1024x1536", "1536x1024"] {
                        button {
                            class: if size() == option { "chip selected" } else { "chip" },
                            onclick: move |_| size.set(option.to_string()),
                            "{option}"
                        }
                    }
                }
            }
            button {
                class: "btn full",
                disabled: loading(),
                onclick: move |_| {
                    let value = prompt().trim().to_string();
                    if value.is_empty() {
                        toast(i18n::t("media.needPrompt"));
                        return;
                    }
                    let provider = DATA
                        .peek()
                        .provider(&provider_id())
                        .cloned();
                    let Some(provider) = provider else {
                        toast(i18n::t("chat.addProviderFirst"));
                        return;
                    };
                    let model_value = model();
                    let size_value = size();
                    let mut loading_signal = loading;
                    let mut error_signal = error;
                    let mut images_signal = images;
                    loading_signal.set(true);
                    error_signal.set(String::new());
                    dioxus::prelude::spawn(async move {
                        match api::generate_image(
                            &provider.base_url,
                            &provider.api_key,
                            &model_value,
                            &value,
                            &size_value,
                        )
                        .await
                        {
                            Ok(image) => {
                                let mut list = images_signal();
                                list.insert(0, image);
                                list.truncate(8);
                                images_signal.set(list);
                            }
                            Err(message) => error_signal.set(message),
                        }
                        loading_signal.set(false);
                    });
                },
                if loading() { {i18n::t("media.generating")} } else { {i18n::t("media.generate")} }
            }
            if !error().is_empty() {
                div { class: "errorbar", span { class: "error-text", "{error}" } }
            }
        }

        if !images().is_empty() {
            SectionCard { title: i18n::t("media.results"), subtitle: i18n::t("media.newestFirst"),
                div { class: "image-grid",
                    for (index, image) in images().iter().cloned().enumerate() {
                        div { key: "{index}", class: "image-card",
                            img { src: "{image}", alt: i18n::t("media.generatedAlt") }
                            div { class: "image-actions",
                                button {
                                    class: "icon-btn small",
                                    title: i18n::t("media.copyUrl"),
                                    onclick: move |_| copy_text(&image),
                                    Icon { icon: LdCopy, width: 16, height: 16, fill: "currentColor" }
                                }
                            }
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
