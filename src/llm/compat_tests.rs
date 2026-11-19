use crate::config::LlmConfig;
use crate::llm::{CompatClient, Llm};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn compat_client_against_local_server() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf);
        let body = r#"{"choices":[{"message":{"content":"pong"}}]}"#;
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(resp.as_bytes());
    });

    let cfg = LlmConfig {
        base_url: format!("http://{addr}/v1"),
        model: "demo".into(),
        api_key: None,
        max_tokens: 64,
        timeout_secs: 5,
    };
    let client = CompatClient::new(&cfg, None).unwrap();
    let out = client.complete("sys", "ping").unwrap();
    assert_eq!(out, "pong");
}
