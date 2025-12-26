//! Database tests.

use super::ConfigDatabase;
use crate::config::models::{
    ApiKeyAccount, DiscoveredModel, McpServer, McpTransport, ModelConfig, OAuthAccount, OAuthToken,
};
use chrono::{Duration, Utc};
use rusqlite::Connection;
use tempfile::TempDir;

struct TestDb {
    db: ConfigDatabase,
    _dir: TempDir,
}

fn test_db() -> TestDb {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test_config.db");
    let conn = Connection::open(&db_path).unwrap();
    let db = ConfigDatabase::new_with_connection(conn);
    db.initialize().unwrap();
    TestDb { db, _dir: dir }
}

#[test]
fn test_mcp_servers_crud_and_agent_mapping() {
    let test = test_db();
    let db = &test.db;

    let server = McpServer::new(uuid::Uuid::new_v4().to_string(), "filesystem")
        .with_stdio_command("mcp-filesystem")
        .with_args(vec!["--root".to_string(), "/tmp".to_string()]);

    db.upsert_mcp_server(&server).unwrap();

    let listed = db.list_mcp_servers().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "filesystem");
    assert_eq!(listed[0].transport, McpTransport::Stdio);
    assert_eq!(listed[0].command.as_deref(), Some("mcp-filesystem"));

    db.set_agent_mcp_server_ids("coding", std::slice::from_ref(&server.id))
        .unwrap();
    let ids = db.get_agent_mcp_server_ids("coding").unwrap();
    assert_eq!(ids, vec![server.id.clone()]);

    db.set_mcp_server_enabled(&server.id, false).unwrap();
    let listed = db.list_mcp_servers().unwrap();
    assert!(!listed[0].is_enabled);

    db.delete_mcp_server(&server.id).unwrap();
    assert!(db.list_mcp_servers().unwrap().is_empty());
    assert!(db.get_agent_mcp_server_ids("coding").unwrap().is_empty());
}

#[test]
fn test_settings_crud() {
    let test = test_db();
    let db = &test.db;

    // Get default settings (theme is set by migrations)
    let theme = db.get_setting("theme").unwrap();
    assert!(theme.is_some());
    assert_eq!(theme.unwrap().value, "dark");

    // Set custom setting
    db.set_setting("test_key", "test_value").unwrap();
    let setting = db.get_setting("test_key").unwrap().unwrap();
    assert_eq!(setting.value, "test_value");

    // Update setting
    db.set_setting("test_key", "updated_value").unwrap();
    let setting = db.get_setting("test_key").unwrap().unwrap();
    assert_eq!(setting.value, "updated_value");

    // Delete setting
    assert!(db.delete_setting("test_key").unwrap());
    assert!(db.get_setting("test_key").unwrap().is_none());
}

#[test]
fn test_models_crud() {
    let test = test_db();
    let db = &test.db;

    // Create model
    let model = ModelConfig::new("claude-3", "Claude 3 Sonnet", "anthropic")
        .with_context_length(200000)
        .as_default();
    db.upsert_model(&model).unwrap();

    // Read model
    let retrieved = db.get_model("claude-3").unwrap().unwrap();
    assert_eq!(retrieved.name, "Claude 3 Sonnet");
    assert_eq!(retrieved.context_length, 200000);
    assert!(retrieved.is_default);

    // Get default model
    let default = db.get_default_model().unwrap().unwrap();
    assert_eq!(default.id, "claude-3");

    // Delete model
    assert!(db.delete_model("claude-3").unwrap());
    assert!(db.get_model("claude-3").unwrap().is_none());
}

#[test]
fn test_oauth_tokens_crud() {
    let test = test_db();
    let db = &test.db;

    // Create token
    let token = OAuthToken::new("claude", "access_token_123").with_refresh_token("refresh_456");
    db.upsert_oauth_token(&token).unwrap();

    // Read token
    let retrieved = db.get_oauth_token("claude").unwrap().unwrap();
    assert_eq!(retrieved.access_token, "access_token_123");
    assert_eq!(retrieved.refresh_token, Some("refresh_456".to_string()));

    // Delete token
    assert!(db.delete_oauth_token("claude").unwrap());
    assert!(db.get_oauth_token("claude").unwrap().is_none());
}

#[test]
fn test_oauth_accounts_prune_expired_without_refresh_token() {
    let test = test_db();
    let db = &test.db;

    let mut expired = OAuthAccount::new("expired", "claude", "access_token");
    expired.expires_at = Some((Utc::now() - Duration::seconds(60)).to_rfc3339());
    expired.refresh_token = None;
    db.upsert_oauth_account(&expired).unwrap();

    let mut expired_empty_refresh = OAuthAccount::new("expired_empty", "claude", "access_token");
    expired_empty_refresh.expires_at = Some((Utc::now() - Duration::seconds(60)).to_rfc3339());
    expired_empty_refresh.refresh_token = Some("".to_string());
    db.upsert_oauth_account(&expired_empty_refresh).unwrap();

    let mut expired_with_refresh =
        OAuthAccount::new("expired_with_refresh", "claude", "access_token");
    expired_with_refresh.expires_at = Some((Utc::now() - Duration::seconds(60)).to_rfc3339());
    expired_with_refresh.refresh_token = Some("refresh_token".to_string());
    db.upsert_oauth_account(&expired_with_refresh).unwrap();

    let listed = db.list_oauth_accounts_pruned(Some("claude")).unwrap();
    let ids: Vec<String> = listed.into_iter().map(|a| a.id).collect();
    assert_eq!(ids, vec!["expired_with_refresh".to_string()]);

    assert!(db.get_oauth_account("expired").unwrap().is_none());
    assert!(db.get_oauth_account("expired_empty").unwrap().is_none());
    assert!(
        db.get_oauth_account("expired_with_refresh")
            .unwrap()
            .is_some()
    );
}

#[test]
fn test_api_key_accounts_crud() {
    let test = test_db();
    let db = &test.db;

    // Create account
    let account = ApiKeyAccount::new("acc1", "openai", "sk-test-123")
        .with_label("Test Key")
        .with_priority(10);
    db.upsert_api_key_account(&account).unwrap();

    // Read account
    let retrieved = db.get_api_key_account("acc1").unwrap().unwrap();
    assert_eq!(retrieved.provider, "openai");
    assert_eq!(retrieved.api_key, "sk-test-123");
    assert_eq!(retrieved.label, Some("Test Key".to_string()));
    assert_eq!(retrieved.priority, 10);
    assert!(retrieved.is_active);

    // List by provider
    let listed = db.list_api_key_accounts(Some("openai")).unwrap();
    assert_eq!(listed.len(), 1);

    // Create another account for same provider
    let account2 = ApiKeyAccount::new("acc2", "openai", "sk-test-456").with_priority(5);
    db.upsert_api_key_account(&account2).unwrap();

    // List should be ordered by priority DESC
    let listed = db.list_api_key_accounts(Some("openai")).unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, "acc1"); // Higher priority first
    assert_eq!(listed[1].id, "acc2");

    // Toggle active
    db.set_api_key_account_active("acc1", false).unwrap();
    let retrieved = db.get_api_key_account("acc1").unwrap().unwrap();
    assert!(!retrieved.is_active);

    // Set cooldown
    let cooldown = (Utc::now() + Duration::seconds(60)).to_rfc3339();
    db.set_api_key_account_cooldown(
        "acc1",
        Some(cooldown.clone()),
        Some("429 error".to_string()),
        Some(Utc::now().to_rfc3339()),
    )
    .unwrap();
    let retrieved = db.get_api_key_account("acc1").unwrap().unwrap();
    assert!(retrieved.cooldown_until.is_some());
    assert_eq!(retrieved.last_error, Some("429 error".to_string()));

    // Clear cooldown
    db.clear_api_key_account_cooldown("acc1").unwrap();
    let retrieved = db.get_api_key_account("acc1").unwrap().unwrap();
    assert!(retrieved.cooldown_until.is_none());
    assert!(retrieved.last_error.is_none());

    // Update priority
    db.set_api_key_account_priority("acc2", 20).unwrap();
    let listed = db.list_api_key_accounts(Some("openai")).unwrap();
    assert_eq!(listed[0].id, "acc2"); // Now higher priority

    // Delete account
    assert!(db.delete_api_key_account("acc1").unwrap());
    assert!(db.get_api_key_account("acc1").unwrap().is_none());

    // List all providers
    let account3 = ApiKeyAccount::new("acc3", "anthropic", "sk-ant-test");
    db.upsert_api_key_account(&account3).unwrap();
    let all = db.list_api_key_accounts(None).unwrap();
    assert_eq!(all.len(), 2);
}

#[test]
fn test_discovered_models_crud() {
    let test = test_db();
    let db = &test.db;

    // Create models
    let model1 = DiscoveredModel::new("claude", "claude-sonnet-4-20250514")
        .with_context_length(200_000)
        .with_display_name("Claude Sonnet 4");
    let model2 = DiscoveredModel::new("openai", "gpt-4o").with_context_length(128_000);
    let model3 =
        DiscoveredModel::new("claude", "claude-3-5-haiku-20241022").with_context_length(200_000);

    // Upsert models
    db.upsert_discovered_model(&model1).unwrap();
    db.upsert_discovered_model(&model2).unwrap();
    db.upsert_discovered_model(&model3).unwrap();

    // Read by canonical ID
    let retrieved = db
        .get_discovered_model("claude:claude-sonnet-4-20250514")
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.canonical_id, "claude:claude-sonnet-4-20250514");
    assert_eq!(retrieved.provider, "claude");
    assert_eq!(retrieved.model_id, "claude-sonnet-4-20250514");
    assert_eq!(retrieved.context_length, 200_000);
    assert_eq!(retrieved.display_name, Some("Claude Sonnet 4".to_string()));

    // List all models
    let all = db.list_discovered_models(None).unwrap();
    assert_eq!(all.len(), 3);

    // List by provider
    let claude_models = db.list_discovered_models(Some("claude")).unwrap();
    assert_eq!(claude_models.len(), 2);

    let openai_models = db.list_discovered_models(Some("openai")).unwrap();
    assert_eq!(openai_models.len(), 1);
    assert_eq!(openai_models[0].model_id, "gpt-4o");

    // Get context length
    let ctx = db
        .get_model_context_length("claude:claude-sonnet-4-20250514")
        .unwrap();
    assert_eq!(ctx, Some(200_000));

    let ctx = db.get_model_context_length("nonexistent:model").unwrap();
    assert_eq!(ctx, None);

    // Update existing model
    let model1_updated =
        DiscoveredModel::new("claude", "claude-sonnet-4-20250514").with_context_length(250_000);
    db.upsert_discovered_model(&model1_updated).unwrap();
    let retrieved = db
        .get_discovered_model("claude:claude-sonnet-4-20250514")
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.context_length, 250_000);

    // Delete specific model
    assert!(db.delete_discovered_model("openai:gpt-4o").unwrap());
    assert!(db.get_discovered_model("openai:gpt-4o").unwrap().is_none());

    // Delete by provider
    let deleted = db.delete_discovered_models_for_provider("claude").unwrap();
    assert_eq!(deleted, 2);
    let all = db.list_discovered_models(None).unwrap();
    assert!(all.is_empty());
}

#[test]
fn test_discovered_models_batch_upsert() {
    let test = test_db();
    let db = &test.db;

    let models = vec![
        DiscoveredModel::new("groq", "llama-3.1-70b").with_context_length(128_000),
        DiscoveredModel::new("groq", "llama-3.1-8b").with_context_length(128_000),
        DiscoveredModel::new("groq", "mixtral-8x7b").with_context_length(32_000),
    ];

    db.upsert_discovered_models_batch(&models).unwrap();

    let groq_models = db.list_discovered_models(Some("groq")).unwrap();
    assert_eq!(groq_models.len(), 3);
}
