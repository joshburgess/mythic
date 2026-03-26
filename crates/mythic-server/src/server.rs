//! Development HTTP server with WebSocket-based live reload.

use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use mythic_core::config::SiteConfig;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Live reload message types sent over WebSocket.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum ReloadMessage {
    /// Full page reload.
    #[serde(rename = "reload")]
    Reload,
    /// Hot CSS reload — update stylesheet without full refresh.
    #[serde(rename = "css-reload")]
    CssReload { path: String },
    /// Hot HTML content update — replace <main> content.
    #[serde(rename = "html-update")]
    HtmlUpdate { html: String },
    /// Build error — display overlay in browser.
    #[serde(rename = "error")]
    Error { message: String },
}

struct AppState {
    output_dir: PathBuf,
    reload_tx: broadcast::Sender<ReloadMessage>,
}

/// The JavaScript client injected into HTML responses for live reload.
const LIVE_RELOAD_SCRIPT: &str = r#"<script>
(function(){
  var proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  var url = proto + '//' + location.host + '/__mythic/ws';
  var retryMs = 1000;
  function connect() {
    var ws = new WebSocket(url);
    ws.onopen = function() { retryMs = 1000; };
    ws.onmessage = function(e) {
      var msg;
      try { msg = JSON.parse(e.data); } catch(_) { location.reload(); return; }
      if (msg.type === 'reload') {
        hideError();
        location.reload();
      } else if (msg.type === 'css-reload') {
        hideError();
        var links = document.querySelectorAll('link[rel="stylesheet"]');
        links.forEach(function(l) {
          var href = l.getAttribute('href').split('?')[0];
          l.setAttribute('href', href + '?v=' + Date.now());
        });
      } else if (msg.type === 'html-update') {
        hideError();
        var main = document.querySelector('main') || document.querySelector('article');
        if (main && msg.html) {
          morphContent(main, msg.html);
        } else {
          location.reload();
        }
      } else if (msg.type === 'error') {
        showError(msg.message);
      }
    };
    ws.onclose = function() {
      setTimeout(function() {
        retryMs = Math.min(retryMs * 2, 10000);
        connect();
      }, retryMs);
    };
  }
  function morphContent(target, newHtml) {
    var tmp = document.createElement('div');
    tmp.innerHTML = newHtml;
    var src = tmp.firstElementChild || tmp;
    reconcile(target, src);
  }
  function reconcile(existing, incoming) {
    var ec = Array.from(existing.childNodes);
    var ic = Array.from(incoming.childNodes);
    var max = Math.max(ec.length, ic.length);
    for (var i = 0; i < max; i++) {
      if (i >= ic.length) { existing.removeChild(ec[i]); continue; }
      if (i >= ec.length) { existing.appendChild(ic[i].cloneNode(true)); continue; }
      if (ec[i].nodeType !== ic[i].nodeType || ec[i].nodeName !== ic[i].nodeName) {
        existing.replaceChild(ic[i].cloneNode(true), ec[i]);
      } else if (ec[i].nodeType === 3) {
        if (ec[i].textContent !== ic[i].textContent) ec[i].textContent = ic[i].textContent;
      } else if (ec[i].nodeType === 1) {
        var ea = ec[i].attributes, ia = ic[i].attributes;
        for (var j = ea.length - 1; j >= 0; j--) {
          if (!ic[i].hasAttribute(ea[j].name)) ec[i].removeAttribute(ea[j].name);
        }
        for (var j = 0; j < ia.length; j++) {
          if (ec[i].getAttribute(ia[j].name) !== ia[j].value)
            ec[i].setAttribute(ia[j].name, ia[j].value);
        }
        reconcile(ec[i], ic[i]);
      }
    }
  }
  function showError(msg) {
    var el = document.getElementById('__mythic-error');
    if (!el) {
      el = document.createElement('div');
      el.id = '__mythic-error';
      el.style.cssText = 'position:fixed;top:0;left:0;right:0;z-index:99999;background:#1a1a2e;color:#ff6b6b;font-family:monospace;font-size:14px;padding:20px 24px;white-space:pre-wrap;border-bottom:3px solid #ff6b6b;max-height:40vh;overflow:auto;';
      document.body.prepend(el);
    }
    el.textContent = msg;
    el.style.display = 'block';
  }
  function hideError() {
    var el = document.getElementById('__mythic-error');
    if (el) el.style.display = 'none';
  }
  connect();
})();
</script>"#;

/// Start the development server.
pub async fn serve(
    config: &SiteConfig,
    root: &Path,
    port: u16,
    reload_tx: broadcast::Sender<ReloadMessage>,
) -> Result<()> {
    let output_dir = root.join(&config.output_dir);

    let state = Arc::new(AppState {
        output_dir: output_dir.clone(),
        reload_tx,
    });

    let app = Router::new()
        .route("/__mythic/ws", get(ws_handler))
        .fallback(get(file_handler))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("  Server running at http://localhost:{port}");
    println!("  Press Ctrl+C to stop\n");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rx = state.reload_tx.subscribe();
    ws.on_upgrade(move |socket| handle_ws(socket, rx))
}

async fn handle_ws(mut socket: WebSocket, mut rx: broadcast::Receiver<ReloadMessage>) {
    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(reload_msg) => {
                        let json = serde_json::to_string(&reload_msg).unwrap_or_default();
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            return;
                        }
                    }
                    Err(_) => return,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => return,
                    _ => {}
                }
            }
        }
    }
}

async fn file_handler(State(state): State<Arc<AppState>>, req: axum::extract::Request) -> Response {
    let path = req.uri().path();

    // Reject any path containing ".." components to prevent traversal before any filesystem ops
    let path_str = path.trim_start_matches('/');
    if path_str.contains("..") {
        return (axum::http::StatusCode::FORBIDDEN, "Forbidden").into_response();
    }

    let mut file_path = state.output_dir.join(path_str);

    // Clean URL: /about/ → /about/index.html
    if file_path.is_dir() || !file_path.exists() {
        let with_index = if file_path.is_dir() {
            file_path.join("index.html")
        } else {
            file_path.with_extension("").join("index.html")
        };
        if with_index.exists() {
            file_path = with_index;
        }
    }

    if !file_path.exists() {
        return (axum::http::StatusCode::NOT_FOUND, "404 Not Found").into_response();
    }

    let content = match std::fs::read(&file_path) {
        Ok(c) => c,
        Err(_) => {
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Read error").into_response()
        }
    };

    let mime = mime_from_path(&file_path);

    // Inject live reload script into HTML responses
    if mime == "text/html" {
        let html = String::from_utf8_lossy(&content);
        let injected = inject_live_reload(&html);
        return Html(injected).into_response();
    }

    ([(axum::http::header::CONTENT_TYPE, mime)], content).into_response()
}

fn inject_live_reload(html: &str) -> String {
    if let Some(pos) = html.rfind("</body>") {
        let mut result = String::with_capacity(html.len() + LIVE_RELOAD_SCRIPT.len());
        result.push_str(&html[..pos]);
        result.push_str(LIVE_RELOAD_SCRIPT);
        result.push('\n');
        result.push_str(&html[pos..]);
        result
    } else {
        format!("{html}{LIVE_RELOAD_SCRIPT}")
    }
}

fn mime_from_path(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html" | "htm") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("xml") => "application/xml",
        _ => "application/octet-stream",
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C handler");
    println!("\nShutting down...");
}

/// Send a reload message to all connected WebSocket clients.
pub fn notify_reload(tx: &broadcast::Sender<ReloadMessage>, msg: ReloadMessage) {
    let _ = tx.send(msg);
}

/// Create a new broadcast channel for reload messages.
pub fn reload_channel() -> (
    broadcast::Sender<ReloadMessage>,
    broadcast::Receiver<ReloadMessage>,
) {
    broadcast::channel(64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injects_script_before_body() {
        let html = "<!DOCTYPE html><html><body><p>Hello</p></body></html>";
        let result = inject_live_reload(html);
        assert!(result.contains("__mythic/ws"));
        assert!(result.contains("<script>"));
        // Script should appear before </body>
        let script_pos = result.find("<script>").unwrap();
        let body_pos = result.find("</body>").unwrap();
        assert!(script_pos < body_pos);
    }

    #[test]
    fn inject_handles_no_body_tag() {
        let html = "<p>Fragment</p>";
        let result = inject_live_reload(html);
        assert!(result.contains("__mythic/ws"));
    }

    #[test]
    fn mime_detection() {
        assert_eq!(mime_from_path(Path::new("style.css")), "text/css");
        assert_eq!(
            mime_from_path(Path::new("app.js")),
            "application/javascript"
        );
        assert_eq!(mime_from_path(Path::new("page.html")), "text/html");
        assert_eq!(mime_from_path(Path::new("photo.webp")), "image/webp");
        assert_eq!(
            mime_from_path(Path::new("unknown.xyz")),
            "application/octet-stream"
        );
    }

    #[test]
    fn live_reload_script_under_5kb() {
        assert!(
            LIVE_RELOAD_SCRIPT.len() < 5120,
            "Script is {}B, must be <5KB",
            LIVE_RELOAD_SCRIPT.len()
        );
    }
}
