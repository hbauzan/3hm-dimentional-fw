//! Environment variable configuration and `.env` loader for upstream LLM providers.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderEnvInfo {
    pub provider: String,
    pub configured: bool,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub env_key_found: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpstreamEnvCatalog {
    pub use_env_available: bool,
    pub providers: HashMap<String, ProviderEnvInfo>,
}

/// Find the `.env` file path looking up from current directory.
pub fn find_env_file() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("ENV_FILE") {
        let p = PathBuf::from(path);
        if p.is_file() {
            return Some(p);
        }
    }

    let candidates = [
        PathBuf::from(".env"),
        PathBuf::from("../.env"),
        PathBuf::from("../../.env"),
    ];

    for candidate in &candidates {
        if candidate.is_file() {
            return candidate
                .canonicalize()
                .ok()
                .or_else(|| Some(candidate.clone()));
        }
    }

    // Check directory of executable or current working directory
    if let Ok(cwd) = std::env::current_dir() {
        let mut curr = cwd.as_path();
        for _ in 0..5 {
            let candidate = curr.join(".env");
            if candidate.is_file() {
                return Some(candidate);
            }
            match curr.parent() {
                Some(parent) => curr = parent,
                None => break,
            }
        }
    }

    None
}

/// Parse `.env` content into key-value pairs.
pub fn parse_env_content(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let line_to_parse = if let Some(stripped) = trimmed.strip_prefix("export ") {
            stripped.trim()
        } else {
            trimmed
        };
        if let Some((k, v)) = line_to_parse.split_once('=') {
            let key = k.trim();
            if key.is_empty() {
                continue;
            }
            let raw_val = v.trim();
            let cleaned_val = strip_quotes(raw_val);
            map.insert(key.to_string(), cleaned_val);
        }
    }
    map
}

fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
    {
        return s[1..s.len() - 1].to_string();
    }
    s.to_string()
}

/// Read all key-value pairs from `.env` file if found.
pub fn read_env_file_map() -> HashMap<String, String> {
    if let Some(path) = find_env_file() {
        if let Ok(content) = fs::read_to_string(path) {
            return parse_env_content(&content);
        }
    }
    HashMap::new()
}

/// Lookup an environment variable: checks `std::env::var` first, then parsed `.env` map.
pub fn lookup_env_var(key: &str, file_map: &HashMap<String, String>) -> Option<String> {
    if let Ok(val) = std::env::var(key) {
        let trimmed = val.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    file_map.get(key).cloned().filter(|v| !v.trim().is_empty())
}

/// Find the first non-empty value among a list of keys.
pub fn lookup_first_nonempty(
    keys: &[&str],
    file_map: &HashMap<String, String>,
) -> Option<(String, String)> {
    for &key in keys {
        if let Some(val) = lookup_env_var(key, file_map) {
            return Some((key.to_string(), val));
        }
    }
    None
}

/// Populate process environment variables from `.env` if not already set.
pub fn load_dotenv() {
    static LOADED: OnceLock<()> = OnceLock::new();
    LOADED.get_or_init(|| {
        let map = read_env_file_map();
        for (k, v) in map {
            if std::env::var(&k).is_err() {
                std::env::set_var(&k, &v);
            }
        }
    });
}

/// Resolve provider info from environment and `.env`.
pub fn get_provider_env(provider: &str) -> ProviderEnvInfo {
    let file_map = read_env_file_map();
    resolve_provider_info(provider, &file_map)
}

fn resolve_provider_info(provider: &str, file_map: &HashMap<String, String>) -> ProviderEnvInfo {
    match provider {
        "mock" => ProviderEnvInfo {
            provider: "mock".to_string(),
            configured: true,
            api_key: String::new(),
            base_url: String::new(),
            model: "mock".to_string(),
            env_key_found: None,
        },
        "gemini" => {
            let key_lookup = lookup_first_nonempty(&["GEMINI_API_KEY", "GOOGLE_API_KEY"], file_map);
            let base_url = lookup_first_nonempty(&["GEMINI_BASE_URL"], file_map)
                .map(|(_, v)| v)
                .unwrap_or_else(|| {
                    "https://generativelanguage.googleapis.com/v1beta/openai".to_string()
                });
            let model = lookup_first_nonempty(&["GEMINI_MODEL"], file_map)
                .map(|(_, v)| v)
                .unwrap_or_else(|| "gemini-3.8-flash".to_string());

            let (env_key_found, api_key, configured) = match key_lookup {
                Some((name, val)) => (Some(name), val, true),
                None => (None, String::new(), false),
            };

            ProviderEnvInfo {
                provider: "gemini".to_string(),
                configured,
                api_key,
                base_url,
                model,
                env_key_found,
            }
        }
        "deepseek" => {
            let key_lookup = lookup_first_nonempty(&["DEEPSEEK_API_KEY"], file_map);
            let base_url = lookup_first_nonempty(&["DEEPSEEK_BASE_URL"], file_map)
                .map(|(_, v)| v)
                .unwrap_or_else(|| "https://api.deepseek.com/v1".to_string());
            let model = lookup_first_nonempty(&["DEEPSEEK_MODEL"], file_map)
                .map(|(_, v)| v)
                .unwrap_or_else(|| "deepseek-chat".to_string());

            let (env_key_found, api_key, configured) = match key_lookup {
                Some((name, val)) => (Some(name), val, true),
                None => (None, String::new(), false),
            };

            ProviderEnvInfo {
                provider: "deepseek".to_string(),
                configured,
                api_key,
                base_url,
                model,
                env_key_found,
            }
        }
        "openai" => {
            let key_lookup = lookup_first_nonempty(&["OPENAI_API_KEY"], file_map);
            let base_url = lookup_first_nonempty(&["OPENAI_BASE_URL"], file_map)
                .map(|(_, v)| v)
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            let model = lookup_first_nonempty(&["OPENAI_MODEL"], file_map)
                .map(|(_, v)| v)
                .unwrap_or_else(|| "gpt-4o-mini".to_string());

            let (env_key_found, api_key, configured) = match key_lookup {
                Some((name, val)) => (Some(name), val, true),
                None => (None, String::new(), false),
            };

            ProviderEnvInfo {
                provider: "openai".to_string(),
                configured,
                api_key,
                base_url,
                model,
                env_key_found,
            }
        }
        "groq" => {
            let key_lookup = lookup_first_nonempty(&["GROQ_API_KEY"], file_map);
            let base_url = lookup_first_nonempty(&["GROQ_BASE_URL"], file_map)
                .map(|(_, v)| v)
                .unwrap_or_else(|| "https://api.groq.com/openai/v1".to_string());
            let model = lookup_first_nonempty(&["GROQ_MODEL"], file_map)
                .map(|(_, v)| v)
                .unwrap_or_else(|| "llama-3.3-70b-versatile".to_string());

            let (env_key_found, api_key, configured) = match key_lookup {
                Some((name, val)) => (Some(name), val, true),
                None => (None, String::new(), false),
            };

            ProviderEnvInfo {
                provider: "groq".to_string(),
                configured,
                api_key,
                base_url,
                model,
                env_key_found,
            }
        }
        "ollama" => {
            let key_lookup =
                lookup_first_nonempty(&["OLLAMA_API_KEY", "UPSTREAM_LLM_API_KEY"], file_map);
            let base_url =
                lookup_first_nonempty(&["OLLAMA_BASE_URL", "UPSTREAM_LLM_BASE_URL"], file_map)
                    .map(|(_, v)| v)
                    .unwrap_or_else(|| "http://127.0.0.1:11434/v1".to_string());
            let model = lookup_first_nonempty(&["OLLAMA_MODEL"], file_map)
                .map(|(_, v)| v)
                .unwrap_or_else(|| "llama3".to_string());

            let (env_key_found, api_key) = match key_lookup {
                Some((name, val)) => (Some(name), val),
                None => (None, String::new()),
            };

            // Ollama is considered configured as long as an endpoint exists
            ProviderEnvInfo {
                provider: "ollama".to_string(),
                configured: true,
                api_key,
                base_url,
                model,
                env_key_found,
            }
        }
        other => ProviderEnvInfo {
            provider: other.to_string(),
            configured: false,
            api_key: String::new(),
            base_url: String::new(),
            model: String::new(),
            env_key_found: None,
        },
    }
}

/// Return catalog of all supported providers and their environment status.
pub fn get_all_providers_env() -> UpstreamEnvCatalog {
    let file_map = read_env_file_map();
    let providers_list = ["mock", "gemini", "openai", "deepseek", "groq", "ollama"];
    let mut providers = HashMap::new();

    for &name in &providers_list {
        providers.insert(name.to_string(), resolve_provider_info(name, &file_map));
    }

    UpstreamEnvCatalog {
        use_env_available: true,
        providers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_env_content_handles_quotes_and_comments() {
        let sample = r#"
# Comment line
SF_HOST=127.0.0.1
GEMINI_API_KEY="my-secret-key-123"
GEMINI_PROJECT_NAME="projects/123456"
OPENAI_API_KEY='sk-another-token'
EMPTY_VAL=
export DEEPSEEK_API_KEY="ds-key"
"#;
        let map = parse_env_content(sample);
        assert_eq!(map.get("SF_HOST").unwrap(), "127.0.0.1");
        assert_eq!(map.get("GEMINI_API_KEY").unwrap(), "my-secret-key-123");
        assert_eq!(map.get("GEMINI_PROJECT_NAME").unwrap(), "projects/123456");
        assert_eq!(map.get("OPENAI_API_KEY").unwrap(), "sk-another-token");
        assert_eq!(map.get("DEEPSEEK_API_KEY").unwrap(), "ds-key");
        assert_eq!(map.get("EMPTY_VAL").unwrap(), "");
    }

    #[test]
    fn resolves_gemini_defaults_and_env_keys() {
        let mut map = HashMap::new();
        map.insert(
            "GEMINI_API_KEY".to_string(),
            "AQ.test_gemini_token".to_string(),
        );

        let info = resolve_provider_info("gemini", &map);
        assert_eq!(info.provider, "gemini");
        assert!(info.configured);
        assert_eq!(info.api_key, "AQ.test_gemini_token");
        assert_eq!(
            info.base_url,
            "https://generativelanguage.googleapis.com/v1beta/openai"
        );
        assert_eq!(info.model, "gemini-3.8-flash");
        assert_eq!(info.env_key_found, Some("GEMINI_API_KEY".to_string()));
    }

    #[test]
    fn resolves_groq_and_ollama() {
        let mut map = HashMap::new();
        map.insert("GROQ_API_KEY".to_string(), "gsk_test".to_string());
        map.insert(
            "UPSTREAM_LLM_BASE_URL".to_string(),
            "http://localhost:11434".to_string(),
        );

        let groq = resolve_provider_info("groq", &map);
        assert!(groq.configured);
        assert_eq!(groq.api_key, "gsk_test");
        assert_eq!(groq.base_url, "https://api.groq.com/openai/v1");
        assert_eq!(groq.model, "llama-3.3-70b-versatile");

        let ollama = resolve_provider_info("ollama", &map);
        assert!(ollama.configured);
        assert_eq!(ollama.base_url, "http://localhost:11434");
        assert_eq!(ollama.model, "llama3");
    }
}
