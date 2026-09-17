use base64::Engine;
use serde_json::json;
use vcu_core::{ErrorCode, ModelConfig, UserConfig, VcuError, VcuResult, VisionPolicy};

pub struct VisionService;

impl VisionService {
    pub fn resolve_model(cfg: &UserConfig) -> VcuResult<&ModelConfig> {
        let name = cfg
            .default_vision_model
            .as_deref()
            .or_else(|| cfg.models.keys().next().map(|s| s.as_str()))
            .ok_or_else(|| {
                VcuError::coded(
                    ErrorCode::VisionProviderRequired,
                    "no vision model configured",
                )
            })?;
        cfg.models.get(name).ok_or_else(|| {
            VcuError::coded(ErrorCode::ModelNotFound, format!("model '{name}' not found"))
        })
    }

    pub fn api_key(model: &ModelConfig) -> VcuResult<String> {
        let key = std::env::var(&model.api_key_env).map_err(|_| {
            VcuError::with_detail(
                ErrorCode::VisionProviderRequired,
                format!("env {} not set", model.api_key_env),
                format!("export {}=... then retry", model.api_key_env),
            )
        })?;
        if key.trim().is_empty() {
            return Err(VcuError::coded(
                ErrorCode::VisionProviderRequired,
                format!("{} is empty", model.api_key_env),
            ));
        }
        Ok(key)
    }

    pub fn should_use_vision(policy: VisionPolicy, force: bool, dom_empty: bool) -> VcuResult<bool> {
        if force {
            return Ok(true);
        }
        match policy {
            VisionPolicy::DomOnly => Ok(false),
            VisionPolicy::VisionAlways | VisionPolicy::VisionFirst => Ok(true),
            VisionPolicy::DomFirst => Ok(dom_empty),
        }
    }

    pub async fn describe_image(
        model: &ModelConfig,
        png_bytes: &[u8],
        prompt: &str,
    ) -> VcuResult<String> {
        if model.provider == "mock" {
            let _ = (png_bytes, prompt);
            return Ok(format!(
                "mock-vision: model={} bytes={} prompt_chars={}",
                model.model,
                png_bytes.len(),
                prompt.chars().count()
            ));
        }
        let key = Self::api_key(model)?;
        let b64 = Engine::encode(&base64::engine::general_purpose::STANDARD, png_bytes);
        let url = format!(
            "{}/chat/completions",
            model.base_url.trim_end_matches('/')
        );
        let body = json!({
            "model": model.model,
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": prompt},
                    {"type": "image_url", "image_url": {"url": format!("data:image/png;base64,{b64}")}}
                ]
            }],
            "max_tokens": 400
        });
        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .bearer_auth(key)
            .json(&body)
            .send()
            .await
            .map_err(|e| VcuError::with_detail(ErrorCode::VisionCallFailed, "http", e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(VcuError::with_detail(
                ErrorCode::VisionCallFailed,
                format!("status {status}"),
                text.chars().take(500).collect::<String>(),
            ));
        }
        let v: serde_json::Value = resp.json().await.map_err(|e| {
            VcuError::with_detail(ErrorCode::VisionCallFailed, "json", e.to_string())
        })?;
        let content = v
            .pointer("/choices/0/message/content")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if content.is_empty() {
            return Err(VcuError::coded(
                ErrorCode::VisionCallFailed,
                "empty vision response",
            ));
        }
        Ok(content)
    }

    pub async fn test_model(model: &ModelConfig) -> VcuResult<String> {
        // 1x1 png
        let png = Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
        )
        .unwrap_or_default();
        Self::describe_image(model, &png, "Reply with the single word: pong").await
    }
}
