use std::collections::HashMap;
use std::{fmt::Error, io::Read, sync::Arc, time::Duration};

use crate::common::client::error::ApiError;
use crate::common::client::models::user_info::{CommonResponse, UserInfoResponse};
use crate::common::wbi_utils::WbiUtils;
use anyhow::{Result, anyhow};
use cookie::Cookie;
use cookie_store::CookieStore;
use flate2::read::GzDecoder;
use reqwest::{
    Client, ClientBuilder, Response, Url,
    header::{ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, COOKIE, REFERER, USER_AGENT},
};
use reqwest_cookie_store::CookieStoreMutex;
use serde::de::DeserializeOwned;
use serde_json::Value;

use tracing::{debug, error, info};

// 支持自动携带认证状态的客户端
#[derive(Debug, Clone)]
pub struct BiliClient {
    pub inner: Client,
    pub cookie_store: Arc<CookieStoreMutex>,
}

impl BiliClient {
    // 创建基础客户端，未认证
    pub fn new() -> Self {
        let headers = Self::get_default_headers();
        // 创建 CookieStore
        let cookie_store = CookieStore::default();
        let cookie_store = CookieStoreMutex::new(cookie_store);
        let cookie_store = Arc::new(cookie_store);

        Self {
            inner: match ClientBuilder::new()
                .timeout(Duration::from_secs(10))
                .cookie_provider(Arc::clone(&cookie_store))
                .default_headers(headers)
                .build()
            {
                Ok(client) => client,
                Err(e) => {
                    error!("Error creating client: {}", e);
                    panic!("Failed to create client");
                }
            },
            cookie_store,
        }
    }

    pub fn get_default_headers() -> reqwest::header::HeaderMap {
        // 创建默认请求头
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(ACCEPT, reqwest::header::HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"));
        headers.insert(
            ACCEPT_LANGUAGE,
            reqwest::header::HeaderValue::from_static("zh-CN,zh;q=0.9"),
        );
        headers.insert(
            ACCEPT_ENCODING,
            reqwest::header::HeaderValue::from_static("gzip, deflate"),
        );
        headers.insert(
            REFERER,
            reqwest::header::HeaderValue::from_static("https://www.bilibili.com/"),
        );
        headers.insert(USER_AGENT, reqwest::header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36"));

        headers
    }

    // 创建已经认证的客户端
    pub fn authenticated(&self) -> Self {
        // 保存Cookie到本地
        info!("Authenticated method called, but cookie management is not implemented.");

        // 返回自己
        Self {
            inner: self.inner.clone(),
            cookie_store: Arc::clone(&self.cookie_store),
        }
    }

    // 通用请求
    pub async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T, ApiError> {
        let cookie = self.get_all_cookies().await;
        let cookie_str = cookie
            .iter()
            .map(|c| {
                format!(
                    "{}={}",
                    c["name"].as_str().unwrap(),
                    c["value"].as_str().unwrap()
                )
            })
            .collect::<Vec<String>>()
            .join(";");

        debug!("Cookie: {}", cookie_str);

        let resp = self
            .inner
            .get(url)
            .header(COOKIE, cookie_str)
            .headers(Self::get_default_headers())
            .send()
            .await
            .map_err(|e| {
                error!("请求失败: {}", e);
                ApiError::InvalidResponse(format!("请求失败: {}", e))
            })?;

        debug!("Response Head: {:?}", resp);
        // debug!("Response Body: {:?}", resp.text().await);

        // Err(ApiError::Other("xxx".to_string())) // 这里可以根据实际情况决定是否需要重试

        Self::handle_response::<T>(resp).await
    }

    // 处理响应
    pub async fn get_raw_response(&self, url: &str) -> Result<Response, ApiError> {
        let cookie = self.get_all_cookies().await;
        let cookie_str = cookie
            .iter()
            .map(|c| {
                format!(
                    "{}={}",
                    c["name"].as_str().unwrap(),
                    c["value"].as_str().unwrap()
                )
            })
            .collect::<Vec<String>>()
            .join(";");

        let resp = self
            .inner
            .get(url)
            .header(COOKIE, cookie_str)
            .headers(Self::get_default_headers())
            .send()
            .await?;

        Ok(resp)
    }

    fn try_decompress(raw: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        let mut decoder = GzDecoder::new(raw);
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded)?;
        Ok(decoded)
    }

    async fn handle_response<T: DeserializeOwned>(resp: Response) -> Result<T, ApiError> {
        let status = resp.status();
        // 自动重试机制
        if status.is_server_error() {
            return Err(ApiError::RetryLater);
        }

        let raw_body = resp.bytes().await?;

        let decompressed = match Self::try_decompress(&raw_body) {
            Ok(data) => data,
            Err(_) => raw_body.to_vec(), // 解压失败则原样使用
        };

        let text = String::from_utf8_lossy(&decompressed);

        // 首先尝试解析 JSON
        match serde_json::from_str::<Value>(&text) {
            Ok(json_value) => {
                // 如果包含 "code"，按B站格式解析
                if let Some(code) = json_value.get("code").and_then(|v| v.as_i64()) {
                    if code != 0 {
                        let message = json_value
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown error")
                            .to_string();
                        return Err(ApiError::ApiError(code, message));
                    }
                }

                debug!("json_value: {}", json_value);

                // 尝试将 JSON 解析为目标类型
                match serde_json::from_value::<T>(json_value) {
                    Ok(data) => Ok(data),
                    Err(e) => {
                        error!("JSON 结构匹配失败: {}", e);
                        Err(ApiError::InvalidResponse(format!(
                            "结构匹配失败: {}. 响应: {}",
                            e, text
                        )))
                    }
                }
            }
            Err(_) => {
                // 不是 JSON，可能是 HTML 页面
                if text.contains("<!DOCTYPE html>") || text.contains("<html") {
                    return Err(ApiError::HtmlResponse(text.to_string()));
                }

                Err(ApiError::InvalidResponse(text.to_string()))
            }
        }
    }

    pub async fn get_all_cookies(&self) -> Vec<serde_json::Value> {
        // 获取所有 Cookie
        let store = self.cookie_store.lock().unwrap();

        let mut cookies = vec![];

        for cookie in store.iter_any() {
            let cookie_info = serde_json::json!({
                "name": cookie.name(),
                "value": cookie.value(),
                "domain": cookie.domain(),
                "path": cookie.path(),
            });
            cookies.push(cookie_info);
        }

        cookies
    }

    pub async fn save_cookies_to_local(&self, path: &str) -> Result<(), Error> {
        // 保存 Cookie 到本地

        let store = self.cookie_store.lock().unwrap();
        let mut writer = std::fs::File::create(path)
            .map(std::io::BufWriter::new)
            .unwrap();

        if let Ok(_) = store.save(&mut writer, serde_json::to_string).map_err(|e| {
            error!("Error saving cookies: {}", e);
            format!("Cookies保存失败: {}", e)
        }) {
            info!("Cookies saved to {}", path);
        } else {
            error!("Error saving cookies");
        }
        Ok(())
    }

    pub async fn load_cookies_from_local(&mut self, path: &str) {
        print!("从{}: 加载 Cookie", path);
        let mut reader = std::fs::File::open(path)
            .map(std::io::BufReader::new)
            .unwrap();
        let store =
            reqwest_cookie_store::CookieStore::load(&mut reader, |s| serde_json::from_str(s))
                .unwrap();

        let store = CookieStoreMutex::new(store);
        let store = Arc::new(store);
        self.inner = ClientBuilder::new()
            .timeout(Duration::from_secs(10))
            .cookie_provider(Arc::clone(&store))
            .default_headers(Self::get_default_headers())
            .build()
            .unwrap();
        self.cookie_store = store;
    }

    pub async fn check_qr_login_status(&self) -> Result<()> {
        for cookie in self.get_all_cookies().await {
            info!("Cookie: {:?}", cookie);
        }

        // 检查登录状态，通过访问个人中心接口
        let personal_url = "https://api.bilibili.com/x/web-interface/nav";

        let resp = self
            .get::<CommonResponse<UserInfoResponse>>(personal_url)
            .await
            .map_err(|e| {
                error!("请求失败: {}", e);
                anyhow!("请求失败: {}", e)
            })?;

        info!("resp: {:?}", resp);
        info!("登录成功");
        Ok(())
    }

    async fn get_cookies(&self, name: &str) -> Option<String> {
        // 获取 Cookie
        let cookie_store = self.cookie_store.lock().unwrap();

        for cookie in cookie_store.iter_any() {
            if cookie.name() == name {
                return Some(cookie.value().to_string());
            }
        }

        None
    }

    pub async fn set_cookies(&self, cookie_info: &serde_json::Value) {
        // 设置 Cookie
        let mut cookie_store = self.cookie_store.lock().unwrap();

        for cookie in cookie_info.as_array().unwrap() {
            let cookie = Cookie::build((
                cookie.get("name").unwrap().as_str().unwrap(),
                cookie.get("value").unwrap().as_str().unwrap(),
            ))
            .domain("bilibili.com")
            .into();

            cookie_store
                .insert_raw(&cookie, &Url::parse("https://www.bilibili.com/").unwrap())
                .unwrap();
        }
    }

    pub async fn post<T: DeserializeOwned>(&self, url: &str, body: &str) -> Result<T, ApiError> {
        let resp = self.inner.post(url).body(body.to_string()).send().await?;
        Self::handle_response(resp).await
    }

    pub async fn post_form<T: DeserializeOwned>(
        &self,
        url: &str,
        form: &str,
    ) -> Result<T, ApiError> {
        let resp = self
            .inner
            .post(url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form.to_string())
            .send()
            .await?;
        Self::handle_response(resp).await
    }
    pub async fn post_json<T: DeserializeOwned>(
        &self,
        url: &str,
        json: &str,
    ) -> Result<T, ApiError> {
        let resp = self
            .inner
            .post(url)
            .header("Content-Type", "application/json")
            .body(json.to_string())
            .send()
            .await?;
        Self::handle_response(resp).await
    }
    pub async fn post_json_with_auth<T: DeserializeOwned>(
        &self,
        url: &str,
        json: &str,
    ) -> Result<T, ApiError> {
        let resp = self
            .inner
            .post(url)
            .header("Content-Type", "application/json")
            // .header("Authorization", format!("Bearer {}", self.auth_cookie.as_ref().unwrap()))
            .body(json.to_string())
            .send()
            .await?;
        Self::handle_response(resp).await
    }
    pub async fn post_form_with_auth<T: DeserializeOwned>(
        &self,
        url: &str,
        form: &str,
    ) -> Result<T, ApiError> {
        let resp = self
            .inner
            .post(url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            // .header("Authorization", format!("Bearer {}", self.auth_cookie.as_ref().unwrap()))
            .body(form.to_string())
            .send()
            .await?;
        Self::handle_response(resp).await
    }
    pub async fn post_with_auth<T: DeserializeOwned>(
        &self,
        url: &str,
        body: &str,
    ) -> Result<T, ApiError> {
        let resp = self
            .inner
            .post(url)
            // .header("Authorization", format!("Bearer {}", self.auth_cookie.as_ref().unwrap()))
            .body(body.to_string())
            .send()
            .await?;
        Self::handle_response(resp).await
    }
    pub async fn get_with_auth<T: DeserializeOwned>(&self, url: &str) -> Result<T, ApiError> {
        let resp = self
            .inner
            .get(url)
            // .header("Authorization", format!("Bearer {}", self.auth_cookie.as_ref().unwrap()))
            .send()
            .await?;
        Self::handle_response(resp).await
    }
    pub async fn delete<T: DeserializeOwned>(&self, url: &str) -> Result<T, ApiError> {
        let resp = self.inner.delete(url).send().await?;
        Self::handle_response(resp).await
    }
    pub async fn delete_with_auth<T: DeserializeOwned>(&self, url: &str) -> Result<T, ApiError> {
        let resp = self
            .inner
            .delete(url)
            // .header("Authorization", format!("Bearer {}", self.auth_cookie.as_ref().unwrap()))
            .send()
            .await?;
        Self::handle_response(resp).await
    }
    pub async fn put<T: DeserializeOwned>(&self, url: &str, body: &str) -> Result<T, ApiError> {
        let resp = self.inner.put(url).body(body.to_string()).send().await?;
        Self::handle_response(resp).await
    }
    pub async fn put_with_auth<T: DeserializeOwned>(
        &self,
        url: &str,
        body: &str,
    ) -> Result<T, ApiError> {
        let resp = self
            .inner
            .put(url)
            // .header("Authorization", format!("Bearer {}", self.auth_cookie.as_ref().unwrap()))
            .body(body.to_string())
            .send()
            .await?;
        Self::handle_response(resp).await
    }

    // 发送带 WBI 签名的 GET 请求
    pub async fn get_auto<T: DeserializeOwned>(
        &self,
        url: &str,
        params: HashMap<String, String>,
    ) -> Result<T, ApiError> {
        let mut url = url.to_string();

        // 尝试获取 wbi keys
        if let Ok((img_key, sub_key)) = self.get_wbi_keys().await {
            url = format!("{}?{}", url, WbiUtils::enc_wbi(params, &img_key, &sub_key));
        } else {
            // fallback: 加上 params 但不加密
            let query = serde_urlencoded::to_string(&params).unwrap_or_default();
            url = format!("{}?{}", url, query);
        }

        self.get::<T>(url.clone().as_str()).await
    }

    // 发送带 WBI 签名的 POST 请求
    pub async fn post_auto<T: DeserializeOwned>(
        &self,
        url: &str,
        params: HashMap<String, String>,
    ) -> Result<T, ApiError> {
        let mut url = url.to_string();

        // 尝试获取 wbi keys
        if let Ok((img_key, sub_key)) = self.get_wbi_keys().await {
            url = format!("{}?{}", url, WbiUtils::enc_wbi(params, &img_key, &sub_key));
        } else {
            // fallback: 加上 params 但不加密
            let query = serde_urlencoded::to_string(&params).unwrap_or_default();
            url = format!("{}?{}", url, query);
        }

        self.post::<T>(url.clone().as_str(), "").await
    }

    // 获取 wbi keys
    pub async fn get_wbi_keys(&self) -> Result<(String, String), ApiError> {
        let url = "https://api.bilibili.com/x/web-interface/nav";
        let resp = self.get::<serde_json::Value>(url).await?;
        let data = resp["data"]
            .as_object()
            .ok_or_else(|| ApiError::InvalidResponse("Failed to get wbi keys".to_string()))?;

        let img_url = data["wbi_img"]["img_url"]
            .as_str()
            .ok_or_else(|| ApiError::InvalidResponse("Failed to get img_url".to_string()))?;
        let sub_url = data["wbi_img"]["sub_url"]
            .as_str()
            .ok_or_else(|| ApiError::InvalidResponse("Failed to get sub_url".to_string()))?;

        // 从URL中提取key
        let img_key = img_url
            .split("/")
            .last()
            .ok_or_else(|| ApiError::InvalidResponse("Failed to parse img_key".to_string()))?
            .split(".")
            .next()
            .ok_or_else(|| ApiError::InvalidResponse("Failed to parse img_key".to_string()))?;

        let sub_key = sub_url
            .split("/")
            .last()
            .ok_or_else(|| ApiError::InvalidResponse("Failed to parse sub_key".to_string()))?
            .split(".")
            .next()
            .ok_or_else(|| ApiError::InvalidResponse("Failed to parse sub_key".to_string()))?;

        Ok((img_key.to_string(), sub_key.to_string()))
    }
}
