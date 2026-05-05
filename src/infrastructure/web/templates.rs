pub trait AppVersion {
    fn app_version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
}
