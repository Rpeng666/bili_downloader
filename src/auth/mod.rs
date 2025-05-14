mod qr_display;
mod errors;
mod token;
mod session;

use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use anyhow::Error;
use colored::Colorize;
pub use qr_display::display_qr;
use session::SessionManager; 
use uuid::Uuid;
use crate::common::api::client::BiliClient;
use crate::common::api::error::ApiError;
use crate::common::api::models::auth::{LoginStatusResponse, QrLoginData, QrStatus};
use crate::common::api::models::common::CommonResponse;

const QR_API: &str = "https://passport.bilibili.com/x/passport-login/web/qrcode/generate";

// 认证状态管理核心组件
#[derive(Debug)]
pub struct AuthManager {

    session_manager: Arc<Mutex<SessionManager>>,

    qr_status : Arc<Mutex<QrStatus>>,
}

// 登录状态机
#[derive(Debug, Clone, Copy, PartialEq)]
enum LoginStatus {
    LoggedOut,
    PendingQrConfirm,
    LoggedIn(u64),
}

impl AuthManager {
    // 创建认证管理器
    pub fn new() ->Self {
        Self { 
            session_manager: Arc::new(Mutex::new(SessionManager::new())),
            qr_status: Arc::new(Mutex::new(QrStatus { status: String::new(), message: String::new(), data: None })),
        }
    }

    // 二维码登录流程
    pub async fn qr_login_flow(&self) -> Result<Uuid, ApiError> {
        println!("{}: \n", "正在生成二维码".green());
        // 生成二维码
        let qr_data = self.generate_qr_session().await?;
        
        // 显示二维码
        if let Err(e) = display_qr(&qr_data.url) {
            eprintln!("Error displaying QR code: {}", e);
            return Err(ApiError::DisplayError(e.to_string()));
        }

        // 轮询登录状态
        let session_id = self.check_qr_login_status(&qr_data.qrcode_key, 60).await?;

        // 返回会话ID
        Ok(session_id)
        
    }

    pub async fn login_by_cookies_file(&self, path: & String) -> Result<Option<Uuid>, Error> {
        // 通过读取文件来检查登录状态
        let mut new_client = BiliClient::new();
        new_client.load_cookies_from_local(path.as_str()).await;
        // 检查登录状态
        if let Err(_) = new_client.check_qr_login_status().await{
            // 登录状态无效
            println!("{}: {}", "登录状态无效".red(), path);
            return Ok(None);
        } else {
            // 登录状态有效
            println!("{}: {}", "登录状态有效".green(), path);
            // 创建新的会话
            let session_manager = self.session_manager.lock().unwrap();
            let session_id = Uuid::new_v4();
            session_manager.create_session(session_id, &new_client).await?;
            return Ok(Some(session_id));
        }
    }

    // 获取当前所有会话
    pub fn  list_sessions(&self) -> Vec<Uuid> {
        let sessions = match self.session_manager.lock() {
            Ok(guard) => guard,
            Err(_) => return vec![],
            
        };

        // 获取所有会话ID
        let session_ids: Vec<Uuid> = match sessions.sessions.lock() {
            Ok(guard) => guard.keys().cloned().collect(),
            Err(_) => return vec![],
        };
        // 返回会话ID列表
        session_ids
    }

    // 执行需要认证的操作
    pub async fn with_session<F, T>(&self, session_id: Uuid, func: F) -> Result<T, ApiError>
    where
        F: FnOnce(&BiliClient) -> Result<T, ApiError>,
    {
        let session_manager = match self.session_manager.lock() {
            Ok(guard) => guard,
            Err(_) => return Err(ApiError::LockError),
            
        };

        let sessions = match session_manager.sessions.lock() {
            Ok(guard) => guard,
            Err(_) => return Err(ApiError::LockError),
        };

        // 获取会话
        if let Some(session) = sessions.get(&session_id) {
            func(&session.client)
        } else {
            Err(ApiError::InvalidSession)
        }
        
    }

    // 获取二维码登录数据（带自动刷新）
    pub async fn generate_qr_session(&self) -> Result<QrLoginData, ApiError> {
        let client = BiliClient::new();
        let qr_data: CommonResponse<QrLoginData> = client
                .get(QR_API)
                .await?;

        println!("qr_data: {:?}", qr_data);
        if qr_data.code != 0 {  
            return Err(ApiError::InvalidResponse(qr_data.message));
        }
        println!("{}: {}", "二维码生成成功".red(), qr_data.message);
        // 显示二维码
        Ok(QrLoginData { url: match qr_data.data {
            Some(ref data) => data.url.clone(),
            None => return Err(ApiError::InvalidResponse("二维码数据URL缺失".to_string())),
            
        }, qrcode_key: match qr_data.data {
            Some(ref data) => data.qrcode_key.clone(),
            None => return Err(ApiError::InvalidResponse("二维码数据Qrcode_key缺失".to_string())),
        } })
    }

    // 轮询登录状态（带超时控制）
    pub async fn check_qr_login_status(&self, qrcode_key: &str, timeout_secs: u64) -> Result<Uuid, ApiError> {
        let client = BiliClient::new();
        let endpoint = format!(
            "https://passport.bilibili.com/x/passport-login/web/qrcode/poll?qrcode_key={}",
            qrcode_key
        );

        // 生成新的会话 ID
        let session_id = Uuid::new_v4();

        // 轮询逻辑
        for _ in 0..timeout_secs {
            let status: LoginStatusResponse = client.get(&endpoint).await?;
            
            // 检查登录状态
            if let Some(ref data) = status.data {
                // 更新状态机
                let mut qr_status = match self.qr_status.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                
                // 根据不同状态进行处理
                match data.code {
                    0 => {
                        // 扫码成功且确认登录
                        println!("{}", "登录成功！".green());
                        qr_status.status = "LoggedIn".to_string();
                        qr_status.message = "登录成功".to_string();

                        // 创建新会话
                        let session_manager = match self.session_manager.lock() {
                            Ok(manager) => manager,
                            Err(_) => return Err(ApiError::LockError),
                        };
                        println!("login success, {:?}", data);
                        // 使用登录成功后的 cookies 创建会话
                        session_manager.create_session(session_id, &client).await?;
                        return Ok(session_id);
                    },
                    86038 => {
                        // 二维码已过期
                        println!("{}", "二维码已过期，请重新获取".red());
                        return Err(ApiError::QrCodeExpired);
                    },
                    86090 => {
                        // 二维码已扫描，等待确认
                        println!("{}", "二维码已扫描，请在手机上确认".yellow());
                        qr_status.status = "PendingConfirm".to_string();
                        qr_status.message = "等待确认".to_string();
                    },
                    86101 => {
                        // 未扫码
                        println!("{}", "等待扫码...".blue());
                        qr_status.status = "WaitingForScan".to_string();
                        qr_status.message = "等待扫描".to_string();
                    },
                    _ => {
                        // 其他未知状态
                        return Err(ApiError::InvalidResponse(format!(
                            "未知的状态码: {}, 消息: {}", 
                            data.code, 
                            data.message
                        )));
                    }
                }
            } else {
                return Err(ApiError::InvalidResponse("登录状态数据缺失".to_string()));
            }

            // 等待1秒后继续轮询
            sleep(Duration::from_secs(1));
        }

        // 超时处理
        Err(ApiError::OperationTimeout)
    }

    pub async fn check_login_status(&self, session_id: Uuid) -> Result<(), ApiError> {
        // 通过读取文件来检查登录状态
        let client = self.session_manager.lock().unwrap().get_authed_client(session_id).await?;
        // 检查登录状态
        if let Err(_) = client.check_qr_login_status().await{
            // 登录状态无效
            println!("{}: {}", "登录状态无效".red(), session_id);
            return Ok(());
        } else {
            // 登录状态有效
            println!("{}: {}", "登录状态有效".green(), session_id);
            return Ok(());
        }
    }

    pub async fn get_authed_client(&self, session_id: Uuid) -> Result<BiliClient, ApiError> {
        let client = self.session_manager.lock().unwrap().get_authed_client(session_id).await?;
        Ok(client)
    }
}

