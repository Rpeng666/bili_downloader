use crate::parser::errors::ParseError;

/// 统一的 API 错误处理函数
pub fn handle_api_error(code: i32, message: &str, context: &str) -> ParseError {
    match code {
        -403 => ParseError::ParseError(format!(
            "{}访问被拒绝（-403）: {}。可能原因：权限不足、需要登录或Cookie已过期",
            context, message
        )),
        -404 => ParseError::ParseError(format!(
            "{}不存在（-404）: {}。内容可能已被删除或URL错误",
            context, message
        )),
        -10403 => ParseError::ParseError(format!(
            "{}需要大会员权限（-10403）: {}。请登录大会员账号或选择较低清晰度",
            context, message
        )),
        -500 => ParseError::ParseError(format!(
            "{}访问限制（-500）: {}。可能需要购买或特定权限",
            context, message
        )),
        6001 => ParseError::ParseError(format!(
            "{}地区限制（6001）: {}。此内容在当前地区不可观看",
            context, message
        )),
        62002 => ParseError::ParseError(format!(
            "{}不可见（62002）: {}。内容可能是私密视频或需要特定权限",
            context, message
        )),
        62012 => ParseError::ParseError(format!(
            "{}审核中（62012）: {}。内容正在审核，暂时无法访问",
            context, message
        )),
        _ => {
            if code != 0 {
                ParseError::ParseError(format!(
                    "{}API返回错误（{}）: {}",
                    context, code, message
                ))
            } else {
                ParseError::ParseError(format!("{}API响应异常: {}", context, message))
            }
        }
    }
}