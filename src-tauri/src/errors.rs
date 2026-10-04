//! Turns provider failures into short sentences a person can act on.

use reqwest::StatusCode;

use crate::settings::host_of;

pub fn provider_name(base_url: &str) -> &'static str {
    let host = host_of(base_url);
    [
        ("anthropic.com", "Anthropic"),
        ("openai.com", "OpenAI"),
        ("openrouter.ai", "OpenRouter"),
        ("googleapis.com", "Google"),
        ("groq.com", "Groq"),
        ("x.ai", "xAI"),
        ("mistral.ai", "Mistral"),
    ]
    .iter()
    .find(|(domain, _)| host.ends_with(domain))
    .map(|(_, name)| *name)
    .unwrap_or("your provider's")
}

/// Marker phrase used to decide whether opening Settings would help.
const CHECK_SETTINGS: &str = "check it in Settings";

pub fn http(service: &str, base_url: &str, status: StatusCode, body: &str) -> String {
    let provider = provider_name(base_url);
    match status.as_u16() {
        401 | 403 => format!("{provider} rejected the {service} key, {CHECK_SETTINGS}"),
        404 => format!("{provider} doesn't know that {service} model or address, {CHECK_SETTINGS}"),
        429 => format!("{provider} says you're out of credits or sending too fast; try again in a moment"),
        500..=599 => format!("{provider} is having trouble right now; try again in a moment"),
        _ => {
            eprintln!("[flitty] {service} error {status}: {}", body.chars().take(300).collect::<String>());
            format!("the {service} request failed ({status})")
        }
    }
}

pub fn network(service: &str, base_url: &str, error: &reqwest::Error) -> String {
    eprintln!("[flitty] {service} network error: {error}");
    format!("I can't reach {} for {service}; check your internet connection or the address in Settings", host_of(base_url))
}

pub fn points_to_settings(message: &str) -> bool {
    message.contains(CHECK_SETTINGS) || message.contains("Settings")
}
