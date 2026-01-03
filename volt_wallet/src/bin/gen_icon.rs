use std::path::Path;

fn main() {
    let src_path = "C:\\Users\\shando\\coin_project\\app_icon.png";
    let dest_path = "C:\\Users\\shando\\coin_project\\volt_wallet\\app_icon.ico";
    
    println!("Opening image from: {}", src_path);
    let img = image::open(src_path).expect("Failed to open source image");
    
    println!("Resizing to 256x256...");
    let resized = img.resize(256, 256, image::imageops::FilterType::Lanczos3);
    
    println!("Saving to: {}", dest_path);
    resized.save_with_format(dest_path, image::ImageFormat::Ico).expect("Failed to save ICO file");
    
    println!("Icon generated successfully!");
}
