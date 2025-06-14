use itertools::Itertools;
use md5::{Digest, Md5};
use std::collections::HashMap;
use time::OffsetDateTime;
use urlencoding::encode;

const MIXIN_KEY_ENC_TAB: [u8; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42, 19, 29,
    28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4, 22, 25,
    54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
];

pub struct WbiUtils;

impl WbiUtils {
    /// 获取混合密钥
    fn get_mixin_key(img_key: &str, sub_key: &str) -> String {
        let orig = format!("{}{}", img_key, sub_key);
        MIXIN_KEY_ENC_TAB
            .iter()
            .filter_map(|&i| orig.chars().nth(i as usize))
            .take(32)
            .collect()
    }

    /// WBI签名加密
    pub fn enc_wbi(mut params: HashMap<String, String>, img_key: &str, sub_key: &str) -> String {
        let mixin_key = Self::get_mixin_key(img_key, sub_key);

        // 添加时间戳
        let curr_time = OffsetDateTime::now_utc().unix_timestamp();
        params.insert("wts".to_string(), curr_time.to_string());

        // 按键排序并过滤特殊字符
        let filtered_params: HashMap<String, String> = params
            .into_iter()
            .sorted_by(|a, b| a.0.cmp(&b.0))
            .map(|(k, v)| {
                let filtered_v = v.chars().filter(|&c| !"!'()*".contains(c)).collect();
                (k, filtered_v)
            })
            .collect();

        // 构建查询字符串
        let query = filtered_params
            .iter()
            .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
            .join("&");

        // 计算w_rid
        let mut hasher = Md5::new();
        hasher.update(format!("{}{}", query, mixin_key).as_bytes());
        let w_rid = format!("{:x}", hasher.finalize());

        // 添加w_rid并返回最终的查询字符串
        let mut final_params = filtered_params;
        final_params.insert("w_rid".to_string(), w_rid);

        final_params
            .iter()
            .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
            .join("&")
    }
}
