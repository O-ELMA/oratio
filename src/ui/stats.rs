use std::collections::BTreeMap;

use chrono::{Datelike, Duration, Local, TimeZone};
use dioxus::prelude::*;
use dioxus_charts::{charts::pie::LabelPosition, BarChart, PieChart};
use dioxus_free_icons::{icons::ld_icons::*, Icon};
use oratio::i18n;
use oratio::store::UsageEntry;

use crate::app::DATA;
use crate::ui::common::{format_tokens, PageHeader, SectionCard};

const PALETTE: [&str; 6] = [
    "#8E4955", "#C96442", "#9A6C3E", "#116682", "#4C662B", "#6A4C93",
];

#[component]
pub fn StatsPage() -> Element {
    let data = DATA();
    let usage = data.usage.clone();
    let mut range = use_signal(|| 30u64);
    let now = Local::now().timestamp_millis() as u64;
    let cutoff = if range() == 0 {
        0
    } else {
        now.saturating_sub(range() * 24 * 3600 * 1000)
    };
    let entries: Vec<UsageEntry> = usage
        .iter()
        .filter(|entry| entry.at >= cutoff)
        .cloned()
        .collect();

    let requests = entries.len();
    let input_tokens: u32 = entries.iter().map(|entry| entry.input_tokens).sum();
    let output_tokens: u32 = entries.iter().map(|entry| entry.output_tokens).sum();
    let total_tokens = input_tokens + output_tokens;

    let mut by_day: BTreeMap<String, u32> = BTreeMap::new();
    let mut by_model: BTreeMap<String, u32> = BTreeMap::new();
    let mut heat: BTreeMap<String, u32> = BTreeMap::new();
    for entry in &entries {
        let Some(time) = Local.timestamp_millis_opt(entry.at as i64).single() else {
            continue;
        };
        let day = time.format("%Y-%m-%d").to_string();
        *by_day.entry(day.clone()).or_default() += entry.input_tokens + entry.output_tokens;
        *heat.entry(day).or_default() += 1;
        let model = if entry.model.is_empty() {
            i18n::t("common.unknownModel")
        } else {
            entry.model.clone()
        };
        *by_model.entry(model).or_default() += entry.input_tokens + entry.output_tokens;
    }

    let days = if range() == 0 { 30 } else { range() as i64 };
    let label_step = if days <= 7 {
        1
    } else if days <= 30 {
        5
    } else {
        15
    };
    let labels: Vec<String> = (0..days)
        .map(|offset| {
            if offset % label_step != 0 && offset != days - 1 {
                return String::new();
            }
            let date = Local::now() - Duration::days(days - 1 - offset);
            if days <= 10 {
                i18n::weekday_short(date.weekday())
            } else {
                format!("{}/{}", date.month(), date.day())
            }
        })
        .collect();
    let values: Vec<f32> = (0..days)
        .map(|offset| {
            let date = Local::now() - Duration::days(days - 1 - offset);
            let key = date.format("%Y-%m-%d").to_string();
            by_day.get(&key).copied().unwrap_or(0) as f32
        })
        .collect();
    let highest = values.iter().cloned().fold(0.0_f32, f32::max).max(1.0);

    let mut ranked: Vec<(String, u32)> = by_model.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));
    ranked.truncate(8);
    let model_values: Vec<f32> = ranked.iter().map(|(_, tokens)| *tokens as f32).collect();

    let today = Local::now().date_naive();
    let start = today - Duration::days(25 * 7 + today.weekday().num_days_from_sunday() as i64);
    let mut weeks: Vec<Vec<(String, u32)>> = Vec::new();
    for week in 0..26 {
        let mut days_cells = Vec::new();
        for day in 0..7 {
            let date = start + Duration::days(week * 7 + day);
            let key = date.format("%Y-%m-%d").to_string();
            let count = heat.get(&key).copied().unwrap_or(0);
            days_cells.push((key, count));
        }
        weeks.push(days_cells);
    }
    let heat_max = heat.values().copied().max().unwrap_or(1).max(1);

    rsx! {
        div { class: "screen",
            PageHeader { title: i18n::t("stats.title"), subtitle: i18n::t("stats.subtitle") }
            div { class: "content",
                SectionCard { title: i18n::t("stats.activity"), subtitle: i18n::t("stats.activitySub"),
                    div { class: "heatmap",
                        for week in weeks {
                            div { class: "heatmap-week",
                                for (key, count) in week {
                                    div {
                                        key: "{key}",
                                        class: "heat-cell",
                                        class: if count == 0 { "heat-0" } else if count * 4 < heat_max { "heat-1" } else if count * 2 < heat_max { "heat-2" } else if count < heat_max { "heat-3" } else { "heat-4" },
                                        title: i18n::ta("stats.heatTitle", &[("date", &key), ("count", &i18n::tn("common.requests", count as usize))]),
                                    }
                                }
                            }
                        }
                    }
                    div { class: "heat-legend",
                        span { {i18n::t("common.less")} }
                        span { class: "heat-cell heat-0" }
                        span { class: "heat-cell heat-1" }
                        span { class: "heat-cell heat-2" }
                        span { class: "heat-cell heat-3" }
                        span { class: "heat-cell heat-4" }
                        span { {i18n::t("common.more")} }
                    }
                }

                div { class: "stats-grid",
                    div { class: "stat-card",
                        Icon { icon: LdMessageSquare, width: 18, height: 18, fill: "currentColor" }
                        span { class: "stat-value", "{requests}" }
                        span { class: "stat-label", {i18n::t("stats.requests")} }
                    }
                    div { class: "stat-card",
                        Icon { icon: LdArrowUp, width: 18, height: 18, fill: "currentColor" }
                        span { class: "stat-value", "{format_tokens(input_tokens)}" }
                        span { class: "stat-label", {i18n::t("common.tokensIn")} }
                    }
                    div { class: "stat-card",
                        Icon { icon: LdArrowDown, width: 18, height: 18, fill: "currentColor" }
                        span { class: "stat-value", "{format_tokens(output_tokens)}" }
                        span { class: "stat-label", {i18n::t("common.tokensOut")} }
                    }
                    div { class: "stat-card",
                        Icon { icon: LdSparkles, width: 18, height: 18, fill: "currentColor" }
                        span { class: "stat-value", "{format_tokens(total_tokens)}" }
                        span { class: "stat-label", {i18n::t("common.total")} }
                    }
                }

                div { class: "range-row",
                    for (label, value) in [(i18n::t("stats.range7"), 7u64), (i18n::t("stats.range30"), 30), (i18n::t("stats.range90"), 90), (i18n::t("common.all"), 0)] {
                        button {
                            class: if range() == value { "range-chip selected" } else { "range-chip" },
                            onclick: move |_| range.set(value),
                            "{label}"
                        }
                    }
                }

                if total_tokens == 0 {
                    SectionCard { title: i18n::t("stats.usage"), subtitle: "",
                        div { class: "muted-note",
                            {i18n::t("stats.noUsage")}
                        }
                    }
                } else {
                    SectionCard { title: i18n::t("stats.tokenTrend"), subtitle: i18n::t("stats.tokenTrendSub"),
                        div { class: "chart-card",
                            BarChart {
                                series: vec![values],
                                labels: Some(labels),
                                viewbox_width: 640,
                                viewbox_height: 260,
                                padding_left: 46,
                                padding_right: 8,
                                padding_top: 18,
                                padding_bottom: 26,
                                highest: Some(highest),
                                bar_width: "58%",
                                show_series_labels: false,
                                show_grid: true,
                                show_dotted_grid: true,
                                show_grid_ticks: false,
                                max_ticks: 4,
                                label_interpolation: (|value| format_tokens(value as u32)) as fn(f32) -> String,
                            }
                        }
                    }

                    SectionCard { title: i18n::t("stats.byModel"), subtitle: i18n::t("stats.byModelSub"),
                        div { class: "model-chart-row",
                            div { class: "pie-wrap",
                                PieChart {
                                    series: model_values.clone(),
                                    labels: None::<Vec<String>>,
                                    viewbox_width: 240,
                                    viewbox_height: 240,
                                    show_labels: false,
                                    donut: true,
                                    donut_width: 34.0,
                                    label_position: LabelPosition::Inside,
                                }
                            }
                            div { class: "legend",
                                for (index, (name, tokens)) in ranked.iter().enumerate() {
                                    div {
                                        key: "{name}",
                                        class: "legend-row",
                                        span {
                                            class: "legend-dot",
                                            style: "background:{PALETTE[index % PALETTE.len()]};",
                                        }
                                        span { class: "legend-name", "{name}" }
                                        span { class: "legend-value", "{format_tokens(*tokens)}" }
                                    }
                                }
                            }
                        }
                    }

                    SectionCard { title: i18n::t("stats.ranking"), subtitle: i18n::t("stats.rankingSub"),
                        for (index, (name, tokens)) in ranked.iter().enumerate() {
                            {
                                let max_tokens = ranked.first().map(|(_, value)| *value).unwrap_or(1).max(1);
                                let percent = (*tokens as f32 / max_tokens as f32 * 100.0).max(2.0);
                                let color = PALETTE[index % PALETTE.len()];
                                rsx! {
                                    div { class: "ranking-row",
                                        span { class: "ranking-index", "{index + 1}" }
                                        div { class: "ranking-text",
                                            span { "{name}" }
                                            div { class: "ranking-bar",
                                                div {
                                                    class: "ranking-fill",
                                                    style: "width:{percent}%;background:{color};",
                                                }
                                            }
                                        }
                                        span { class: "ranking-value", "{format_tokens(*tokens)}" }
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
