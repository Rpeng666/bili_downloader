#[async_trait]
impl DownloadTaskTrait for BangumiInfoVec {
    async fn to_download_task(&self) -> Result<DownloadTask, ParseError> {
        let mut tasks = Vec::new();
        for ep in & self.0 {
            if let Some(url) = &ep.video_url {
                tasks.push(DownloadTask:: new(url.clone(), "video".to_string()))
            }
        }
        Ok(DownloadTask::Group(tasks))
    }
}