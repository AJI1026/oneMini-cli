fn main() {
    #[cfg(windows)]
    {
        let icon = std::path::Path::new("assets/onemini.ico");
        if icon.is_file() {
            let mut res = winres::WindowsResource::new();
            res.set_icon(icon.to_str().expect("icon path utf-8"));
            if let Err(err) = res.compile() {
                eprintln!("cargo:warning=无法嵌入 Windows 图标: {err}");
            }
        } else {
            println!("cargo:warning=assets/onemini.ico 不存在，跳过 Windows 图标嵌入");
        }
    }
}
