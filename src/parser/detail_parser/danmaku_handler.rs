pub struct DanmakuHandler;

impl DanmakuHandler {
    pub fn new() -> Self {
        DanmakuHandler
    }
    /// 获取弹幕下载地址
    pub fn get_url(cid: i64) -> Result<String, String> {
        let url = format!("https://comment.bilibili.com/{}.xml", cid);
        Ok(url)
    }
}
