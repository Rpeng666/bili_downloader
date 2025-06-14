use colored::Colorize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::client::error::ApiError;

use super::BiliClient;
// use super::token::TokenInfo;

#[derive(Debug)]
pub struct SessionManager {
    pub sessions: Arc<Mutex<HashMap<Uuid, BiliClient>>>, // 使用UUID作为会话ID
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // 登录成功之后，创建新的会话
    pub async fn create_session(
        &self,
        session_id: Uuid,
        client: &BiliClient,
    ) -> Result<Uuid, ApiError> {
        // let _ = client.check_qr_login_status().await;

        let mut sessions = self.sessions.lock().unwrap();
        if sessions.contains_key(&session_id) {
            return Err(ApiError::InvalidResponse("会话已存在".to_string()));
        }
        // 创建新的会话
        info!("{}: {}", "创建会话".green(), session_id);
        sessions.insert(session_id, client.clone());

        // 保存会话到文件
        self.save_session_to_file(session_id, client).await?;
        info!("{}: {}", "会话已保存".green(), session_id);

        Ok(session_id)
    }

    pub async fn save_session_to_file(
        &self,
        session_id: Uuid,
        client: &BiliClient,
    ) -> Result<(), ApiError> {
        // 将会话信息保存到文件
        let folder_path = format!("./sessions/{}", session_id);
        std::fs::create_dir_all(&folder_path)
            .map_err(|e| ApiError::InvalidResponse(format!("创建目录失败: {}", e)))?;
        let file_path = format!("{}/cookies.jsonl", folder_path);
        client
            .save_cookies_to_local(file_path.as_str())
            .await
            .map_err(|e| ApiError::InvalidResponse(format!("保存会话失败: {}", e)))?;
        info!("{}: {}", "会话已保存".green(), session_id);
        Ok(())
    }

    // 获取会话中的客户端（自动携带最新令牌）
    pub async fn get_authed_client(&self, session_id: Uuid) -> Result<BiliClient, ApiError> {
        let sessions = match self.sessions.lock() {
            Ok(sessions) => sessions,
            Err(_) => return Err(ApiError::InvalidResponse("会话锁定失败".to_string())),
        };

        if let Some(client) = sessions.get(&session_id) {
            Ok(client.clone())
        } else {
            warn!("会话不存在, 使用默认客户端，部分功能可能不可用");
            // 返回一个未登录的客户端
            Ok(BiliClient::new())
        }
    }

    // 销毁会话
    pub fn destory_session(&self, session_id: Uuid) -> Result<(), ApiError> {
        let mut sessions = self.sessions.lock().unwrap();
        if sessions.remove(&session_id).is_some() {
            Ok(())
        } else {
            Err(ApiError::InvalidResponse("会话不存在".to_string()))
        }
    }

    // 验证cookie的有效性
    async fn validate_cookie(&self, cookie: &str) -> Result<u64, ApiError> {
        // let client = BiliClient::authenticated(cookie);
        // let profile: UserProfile = client
        //     .get("/user/profile")
        //     .await?;
        // Ok(profile.mid)
        Ok(0)
    }

    // // 获取初始令牌
    // async fn fetch_initial_token(&self, cookie: &str) -> Result<TokenInfo, ApiError> {
    //     // let client = BiliClient::authenticated(cookie);
    //     // let token_info: TokenInfoResponse = client
    //     //     .post("/auth/token", "").await?;

    //     // Ok(TokenInfo {
    //     //     access_token: token_info.access_token,
    //     //     refresh_token: token_info.refresh_token,
    //     //     expires_at: SystemTime::now() + Duration::from_secs(token_info.expires_in),
    //     // })

    //     Err(ApiError::InvalidResponse("获取初始令牌失败".to_string()))
    // }
}
