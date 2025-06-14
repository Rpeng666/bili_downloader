use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::Mutex;

use serde_derive::Deserialize;
use tokio::time::interval;
use tracing::error;

use super::BiliClient;

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: SystemTime, // 精确到秒的过期时间
}

impl TokenInfo {
    // 判断是否需要刷新
    pub fn needs_refresh(&self) -> bool {
        SystemTime::now()
            .duration_since(self.expires_at)
            .map(|d| d.as_secs() > 300)
            .unwrap_or(true)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenInfoResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64, // 过期时间，单位秒
}

// 自动刷新的令牌管家
#[derive(Debug, Clone)]
pub struct TokenKeeper {
    current_token: Arc<Mutex<TokenInfo>>,
    client: BiliClient, // 使用未认证的客户端进行刷新
}

impl TokenKeeper {
    pub fn new(initial_token: TokenInfo) -> Self {
        let client = BiliClient::new();
        Self {
            current_token: Arc::new(Mutex::new(initial_token)),
            client: client,
        }
    }

    // 启动后台刷新任务
    pub fn start_refresh_daemon(&self) -> tokio::task::JoinHandle<()> {
        let token_ref = self.current_token.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60));

            loop {
                interval.tick().await;

                // 检查是否需要刷新
                let needs_refresh = {
                    let token = token_ref.lock().await;
                    token.needs_refresh()
                };

                if needs_refresh {
                    let refresh_token = {
                        let token = token_ref.lock().await;
                        token.refresh_token.clone()
                    };

                    // 调用刷新 API
                    if let Ok(Some(new_token)) = client
                        .post::<Option<RefreshResponse>>("/auth/refresh", "")
                        .await
                    {
                        // 更新 token
                        let mut token = token_ref.lock().await;
                        *token = TokenInfo {
                            access_token: new_token.access_token,
                            refresh_token: new_token.refresh_token,
                            expires_at: SystemTime::now()
                                + Duration::from_secs(new_token.expires_in),
                        };
                    } else {
                        // 处理刷新失败的情况
                        error!("Failed to refresh token");
                        continue;
                    }
                }
            }
        })
    }

    // 获取当前的有效令牌
    pub async fn get_current_token(&self) -> TokenInfo {
        // 这里使用锁来获取当前令牌
        // 由于是异步锁，所以需要使用await
        // 这里的unwrap()是为了处理锁的错误情况
        // 你可以根据你的需求来处理这个错误
        // 例如：返回一个默认值或者抛出一个错误
        self.current_token.lock().await.clone()
    }
}

// 刷新接口响应模型
#[derive(Deserialize)]
struct RefreshResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64, // 过期时间，单位秒
}
