pub fn build_authenticated_rtsp_url(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    raw_rtsp_url: &str,
) -> String {
    let clean_url = raw_rtsp_url.trim();
    if clean_url.is_empty() {
        if username.is_empty() {
            format!("rtsp://{}:{}/Streaming/Channels/101", host, port)
        } else {
            format!("rtsp://{}:{}@{}:{}/Streaming/Channels/101", username, password, host, port)
        }
    } else if clean_url.starts_with("rtsp://") {
        if username.is_empty() {
            clean_url.to_string()
        } else {
            // If already contains credentials, replace or insert
            let after_proto = &clean_url[7..];
            if let Some(at_idx) = after_proto.find('@') {
                format!("rtsp://{}:{}@{}", username, password, &after_proto[at_idx + 1..])
            } else {
                format!("rtsp://{}:{}@{}", username, password, after_proto)
            }
        }
    } else {
        if username.is_empty() {
            format!("rtsp://{}:{}/{}", host, port, clean_url.trim_start_matches('/'))
        } else {
            format!("rtsp://{}:{}@{}:{}/{}", username, password, host, port, clean_url.trim_start_matches('/'))
        }
    }
}
