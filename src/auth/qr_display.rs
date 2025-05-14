use colored::Colorize;
use qrcode::{QrCode};
use qrcode::types::Color;
use super::errors::Result;


pub fn display_qr(url: &str) -> Result<()> {
    println !("{}: ", "请扫描二维码".green());

    let code = QrCode::new(url).unwrap();
    let image = code.render::<qrcode::render::unicode::Dense1x2>()
        .quiet_zone(false)  // 要不要边缘空白
        .module_dimensions(1, 1) 
        .build();

    print!("\n{}", image);
    Ok(())
    
}