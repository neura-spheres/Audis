//! Chat completions, for the assistant.

use std::time::Duration;

use audis_common::{ChatApi, ProviderId};

use crate::error::{AsrError, Result};

const TIMEOUT: Duration = Duration::from_secs(45);
const MAX_TOKENS: u32 = 1024;

/// Ask a provider's chat model for a reply.
pub fn chat(
    provider: ProviderId,
    endpoint: Option<String>,
    api_key: String,
    model: &str,
    system: &str,
    user: &str,
) -> Result<String> {
    let name = provider.info().name;
    let Some(support) = provider.chat() else {
        return Err(AsrError::Recognition {
            detail: format!("{name} transcribes speech but cannot answer questions"),
        });
    };

    if api_key.trim().is_empty() {
        return Err(AsrError::ProviderKeyMissing { provider: name });
    }

    let base_url = match (&support.base_url, endpoint) {
        (Some(fixed), _) => fixed.clone(),
        (None, Some(user)) if !user.trim().is_empty() => user.trim().to_owned(),
        (None, _) => {
            return Err(AsrError::Recognition {
                detail: format!("{name} needs an endpoint, and none is set"),
            });
        }
    };
    let base_url = base_url.trim_end_matches('/').to_owned();

    let client = reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .map_err(|error| AsrError::ProviderUnreachable {
            provider: name.clone(),
            detail: error.to_string(),
        })?;

    match support.api {
        ChatApi::OpenAiChat => {
            let body = serde_json::json!({
                "model": model,
                "messages": [
                    { "role": "system", "content": system },
                    { "role": "user", "content": user },
                ],
            });
            let value = send(
                client
                    .post(format!("{base_url}/chat/completions"))
                    .bearer_auth(&api_key)
                    .json(&body),
                &name,
            )?;
            Ok(value
                .pointer("/choices/0/message/content")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_owned())
        }
        ChatApi::GeminiGenerate => {
            let body = serde_json::json!({
                "systemInstruction": { "parts": [{ "text": system }] },
                "contents": [{ "role": "user", "parts": [{ "text": user }] }],
                "generationConfig": { "temperature": 0.4 },
            });
            let value = send(
                client
                    .post(format!("{base_url}/models/{model}:generateContent"))
                    .header("x-goog-api-key", &api_key)
                    .json(&body),
                &name,
            )?;
            Ok(value
                .pointer("/candidates/0/content/parts/0/text")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_owned())
        }
        ChatApi::AnthropicMessages => {
            let body = serde_json::json!({
                "model": model,
                "max_tokens": MAX_TOKENS,
                "system": system,
                "messages": [{ "role": "user", "content": user }],
            });
            let value = send(
                client
                    .post(format!("{base_url}/messages"))
                    .header("x-api-key", &api_key)
                    .header("anthropic-version", "2023-06-01")
                    .json(&body),
                &name,
            )?;
            Ok(value
                .pointer("/content/0/text")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_owned())
        }
    }
}

fn send(request: reqwest::blocking::RequestBuilder, provider: &str) -> Result<serde_json::Value> {
    let response = request
        .send()
        .map_err(|error| AsrError::ProviderUnreachable {
            provider: provider.to_owned(),
            detail: error.to_string(),
        })?;

    let status = response.status();
    let text = response.text().unwrap_or_default();

    if !status.is_success() {
        return Err(AsrError::ProviderRejected {
            provider: provider.to_owned(),
            status: status.as_u16(),
            detail: text.chars().take(300).collect(),
        });
    }

    serde_json::from_str(&text).map_err(|error| AsrError::Recognition {
        detail: format!("could not read the reply: {error}"),
    })
}
