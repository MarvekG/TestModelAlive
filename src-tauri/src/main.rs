fn main() {
    set_default_env("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    set_default_env("LIBGL_ALWAYS_SOFTWARE", "1");
    tsa_lib::run();
}

fn set_default_env(key: &str, value: &str) {
    if std::env::var_os(key).is_none() {
        std::env::set_var(key, value);
    }
}
