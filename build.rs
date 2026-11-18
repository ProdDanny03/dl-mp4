const ICON: &str = "./assets/app_icon.ico";

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon(ICON)
            .compile()
            .unwrap_or_else(|err| panic!("failed to embed {ICON}: {err}"));
    }
}
