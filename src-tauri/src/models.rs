use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountItem {
    pub id: String,
    pub name: String,
    pub email: String,
    pub user_scope: String,
    pub credential_blob: String,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountVault {
    pub accounts: Vec<AccountItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStateDto {
    pub current_email: Option<String>,
    pub current_user_scope: Option<String>,
    pub is_antigravity_running: bool,
    pub accounts: Vec<AccountItem>,
}
