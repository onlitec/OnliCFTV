pub fn encode_userinfo(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

pub fn build_authenticated_rtsp_url(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    raw_rtsp_url: &str,
) -> String {
    let clean_url = raw_rtsp_url.trim();
    let enc_user = encode_userinfo(username.trim());
    let enc_pass = encode_userinfo(password.trim());

    if clean_url.is_empty() {
        if username.is_empty() {
            format!("rtsp://{}:{}/Streaming/Channels/101", host, port)
        } else {
            format!("rtsp://{}:{}@{}:{}/Streaming/Channels/101", enc_user, enc_pass, host, port)
        }
    } else if clean_url.starts_with("rtsp://") {
        if username.is_empty() {
            clean_url.to_string()
        } else {
            let after_proto = &clean_url[7..];
            if let Some(at_idx) = after_proto.find('@') {
                format!("rtsp://{}:{}@{}", enc_user, enc_pass, &after_proto[at_idx + 1..])
            } else {
                format!("rtsp://{}:{}@{}", enc_user, enc_pass, after_proto)
            }
        }
    } else {
        if username.is_empty() {
            format!("rtsp://{}:{}/{}", host, port, clean_url.trim_start_matches('/'))
        } else {
            format!("rtsp://{}:{}@{}:{}/{}", enc_user, enc_pass, host, port, clean_url.trim_start_matches('/'))
        }
    }
}
