//! macOS / iOS 专用：本机回环 HTTP 服务，承载封面 / m3u8 / 分片代理。
//!
//! 为什么不用 `hg://localhost`：macOS 26.6 起 WKWebView 会拦截「页面在一个自定义协议
//! （tauri://localhost）上、子资源在另一个自定义协议（hg://）上」的跨协议加载，
//! 协议处理器根本收不到请求，封面空白、视频不播（tauri-apps/wry#1778）。
//! 走 http://127.0.0.1 属于普通网络加载，不受影响，媒体也能用 WebKit 原生管线。
//!
//! 安全：只监听 127.0.0.1（不会触发防火墙弹窗），路径带随机令牌，其它进程猜不到。

use crate::media;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const MAX_HEAD: usize = 16 * 1024;

fn token() -> String {
    let mut out = String::new();
    for i in 0..2u64 {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(i);
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

/// 启动服务，返回前端要用的地址前缀，如 `http://127.0.0.1:53124/3f9a…`
pub async fn start() -> std::io::Result<String> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();
    let tok = token();
    let origin = format!("http://127.0.0.1:{port}/{tok}");
    let base = format!("/{tok}");
    let origin_c = origin.clone();
    tauri::async_runtime::spawn(async move {
        let mut listener = Some(listener);
        loop {
            // iOS 挂起较久后系统会回收监听 socket，accept 持续报错：
            // 丢掉旧的，在原端口重新绑定（页面里已注入的地址不变，继续可用）。
            let l = match listener.take() {
                Some(l) => l,
                None => match TcpListener::bind(("127.0.0.1", port)).await {
                    Ok(l) => l,
                    Err(_) => {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        continue;
                    }
                },
            };
            loop {
                let stream = match l.accept().await {
                    Ok((s, _)) => s,
                    Err(_) => {
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                        break;
                    }
                };
                let base = base.clone();
                let origin = origin_c.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = serve_conn(stream, &base, &origin).await;
                });
            }
        }
    });
    Ok(origin)
}

fn find_head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
}

async fn serve_conn(mut stream: TcpStream, base: &str, origin: &str) -> std::io::Result<()> {
    let _ = stream.set_nodelay(true);
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    loop {
        // 读到一个完整请求头（GET/HEAD/OPTIONS 都没有请求体）
        let end = loop {
            if let Some(e) = find_head_end(&buf) {
                break e;
            }
            if buf.len() > MAX_HEAD {
                return Ok(());
            }
            let n = stream.read(&mut chunk).await?;
            if n == 0 {
                return Ok(());
            }
            buf.extend_from_slice(&chunk[..n]);
        };
        let head = String::from_utf8_lossy(&buf[..end]).to_string();
        buf.drain(..end);

        let mut lines = head.split("\r\n");
        let req_line = lines.next().unwrap_or("");
        let mut parts = req_line.split_whitespace();
        let method = parts.next().unwrap_or("").to_ascii_uppercase();
        let target = parts.next().unwrap_or("/").to_string();
        let mut range: Option<String> = None;
        let mut close = false;
        for l in lines {
            if let Some((k, v)) = l.split_once(':') {
                let k = k.trim().to_ascii_lowercase();
                let v = v.trim();
                if k == "range" {
                    range = Some(v.to_string());
                } else if k == "connection" && v.eq_ignore_ascii_case("close") {
                    close = true;
                }
            }
        }

        let (path, query) = target.split_once('?').unwrap_or((target.as_str(), ""));
        let resp = match path.strip_prefix(base) {
            None => media_404(),
            Some(_) if method == "OPTIONS" => media::preflight(),
            Some(p) if method == "GET" || method == "HEAD" => {
                media::handle(p, query, origin, range.as_deref()).await
            }
            Some(_) => media_404(),
        };

        let (parts, body) = resp.into_parts();
        let mut out = format!(
            "HTTP/1.1 {} {}\r\n",
            parts.status.as_u16(),
            parts.status.canonical_reason().unwrap_or("")
        );
        for (k, v) in parts.headers.iter() {
            if let Ok(v) = v.to_str() {
                out.push_str(&format!("{}: {}\r\n", k.as_str(), v));
            }
        }
        out.push_str(&format!("Content-Length: {}\r\n", body.len()));
        out.push_str(if close { "Connection: close\r\n\r\n" } else { "Connection: keep-alive\r\n\r\n" });
        stream.write_all(out.as_bytes()).await?;
        if method != "HEAD" {
            stream.write_all(&body).await?;
        }
        stream.flush().await?;
        if close {
            return Ok(());
        }
    }
}

fn media_404() -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(404)
        .body(b"not found".to_vec())
        .unwrap_or_else(|_| tauri::http::Response::new(Vec::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn serves_with_token_only() {
        let origin = start().await.expect("start");
        let rest = origin.strip_prefix("http://").unwrap();
        let (hostport, tok) = rest.split_once('/').unwrap();
        let mut s = TcpStream::connect(hostport).await.unwrap();
        s.write_all(format!("OPTIONS /{tok}/img?u=x HTTP/1.1\r\nHost: a\r\n\r\n").as_bytes()).await.unwrap();
        let mut b = vec![0u8; 512];
        let n = s.read(&mut b).await.unwrap();
        assert!(String::from_utf8_lossy(&b[..n]).starts_with("HTTP/1.1 204"));
        s.write_all(b"GET /wrong/img?u=x HTTP/1.1\r\nConnection: close\r\n\r\n").await.unwrap();
        let n = s.read(&mut b).await.unwrap();
        assert!(String::from_utf8_lossy(&b[..n]).starts_with("HTTP/1.1 404"));
    }
}
