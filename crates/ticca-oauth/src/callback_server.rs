//! Local HTTP server for OAuth callbacks

use crate::common::{OAuthError, OAuthResult};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

/// HTML response page for successful auth
const SUCCESS_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Authorization Complete</title>
    <style>
        :root {
            color-scheme: light;
        }
        body {
            font-family: "Segoe UI", "SF Pro Text", "Helvetica Neue", Arial, sans-serif;
            margin: 0;
            min-height: 100vh;
            display: grid;
            place-items: center;
            background: radial-gradient(circle at top, #f6f7fb 0%, #e8ebf3 60%, #dde2ee 100%);
            color: #1f2937;
        }
        .card {
            width: min(520px, 92vw);
            background: #ffffff;
            border-radius: 20px;
            padding: 36px 40px;
            box-shadow: 0 20px 50px rgba(31, 41, 55, 0.15);
            border: 1px solid #e5e7eb;
        }
        .badge {
            display: inline-flex;
            align-items: center;
            gap: 8px;
            padding: 6px 12px;
            border-radius: 999px;
            background: #ecfdf3;
            color: #15803d;
            font-size: 12px;
            font-weight: 600;
            letter-spacing: 0.02em;
        }
        h1 {
            margin: 18px 0 8px;
            font-size: 24px;
        }
        p {
            margin: 0;
            color: #4b5563;
            line-height: 1.5;
        }
        .hint {
            margin-top: 18px;
            font-size: 13px;
            color: #6b7280;
        }
        .actions {
            margin-top: 22px;
            display: inline-flex;
            gap: 10px;
        }
        .button {
            background: #111827;
            color: #ffffff;
            border: none;
            padding: 10px 14px;
            border-radius: 10px;
            font-size: 13px;
            text-decoration: none;
            display: inline-block;
        }
        .secondary {
            background: #f3f4f6;
            color: #111827;
        }
    </style>
</head>
<body>
    <div class="card">
        <div class="badge">Authorization Complete</div>
        <h1>You're all set.</h1>
        <p>The account is connected successfully. You can return to Ticca Desktop now.</p>
        <div class="actions">
            <span class="button">Done</span>
            <span class="button secondary">Close this tab</span>
        </div>
        <div class="hint">This window can be closed safely.</div>
    </div>
</body>
</html>"#;

/// HTML response page for errors
const ERROR_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Authorization Failed</title>
    <style>
        :root {
            color-scheme: light;
        }
        body {
            font-family: "Segoe UI", "SF Pro Text", "Helvetica Neue", Arial, sans-serif;
            margin: 0;
            min-height: 100vh;
            display: grid;
            place-items: center;
            background: radial-gradient(circle at top, #fff5f5 0%, #fde8e8 60%, #fbd5d5 100%);
            color: #1f2937;
        }
        .card {
            width: min(520px, 92vw);
            background: #ffffff;
            border-radius: 20px;
            padding: 36px 40px;
            box-shadow: 0 20px 50px rgba(127, 29, 29, 0.12);
            border: 1px solid #fee2e2;
        }
        .badge {
            display: inline-flex;
            align-items: center;
            gap: 8px;
            padding: 6px 12px;
            border-radius: 999px;
            background: #fef2f2;
            color: #b91c1c;
            font-size: 12px;
            font-weight: 600;
            letter-spacing: 0.02em;
        }
        h1 {
            margin: 18px 0 8px;
            font-size: 24px;
        }
        p {
            margin: 0;
            color: #4b5563;
            line-height: 1.5;
        }
        .error {
            margin-top: 14px;
            background: #fef2f2;
            border: 1px solid #fecaca;
            color: #991b1b;
            font-family: ui-monospace, "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
            padding: 10px 12px;
            border-radius: 10px;
            font-size: 12px;
            white-space: pre-wrap;
        }
        .hint {
            margin-top: 18px;
            font-size: 13px;
            color: #6b7280;
        }
    </style>
</head>
<body>
    <div class="card">
        <div class="badge">Authorization Failed</div>
        <h1>We couldn't complete the request.</h1>
        <p>Please return to Ticca Desktop and try again.</p>
        <div class="error">ERROR_MESSAGE</div>
        <div class="hint">If this persists, check your network connection or provider status.</div>
    </div>
</body>
</html>"#;

/// Callback result from OAuth provider
#[derive(Debug, Clone)]
pub struct CallbackResult {
    pub code: String,
    pub state: String,
}

/// Find an available port in the given range
pub fn find_available_port(range: (u16, u16)) -> OAuthResult<u16> {
    for port in range.0..=range.1 {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return Ok(port);
        }
    }
    Err(OAuthError::CallbackServerError(format!(
        "No available port in range {}-{}",
        range.0, range.1
    )))
}

/// Start a callback server and wait for the OAuth redirect
pub fn wait_for_callback(
    port: u16,
    path: &str,
    expected_state: &str,
    timeout: Duration,
) -> OAuthResult<CallbackResult> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| OAuthError::CallbackServerError(format!("Failed to bind: {}", e)))?;

    listener.set_nonblocking(true).map_err(|e| {
        OAuthError::CallbackServerError(format!("Failed to set non-blocking: {}", e))
    })?;

    let start = std::time::Instant::now();
    let expected_path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path)
    };

    tracing::info!(
        "OAuth callback server listening on port {} for path {}",
        port,
        expected_path
    );

    loop {
        // Check timeout
        if start.elapsed() > timeout {
            return Err(OAuthError::Timeout);
        }

        // Try to accept a connection
        match listener.accept() {
            Ok((mut stream, _)) => {
                // Read the request
                let mut buffer = [0u8; 4096];
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .map_err(OAuthError::IoError)?;

                let bytes_read = match stream.read(&mut buffer) {
                    Ok(n) => n,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    Err(e) => return Err(OAuthError::IoError(e)),
                };

                let request = String::from_utf8_lossy(&buffer[..bytes_read]);

                // Parse the request line
                let request_line = request.lines().next().unwrap_or("");

                // Check if this is our callback path
                if let Some(path_and_query) = request_line.split_whitespace().nth(1)
                    && path_and_query.starts_with(&expected_path)
                {
                    // Parse query parameters
                    if let Some(query) = path_and_query.split('?').nth(1) {
                        let params: std::collections::HashMap<_, _> = query
                            .split('&')
                            .filter_map(|pair| {
                                let mut parts = pair.split('=');
                                Some((parts.next()?, parts.next()?))
                            })
                            .collect();

                        // Check for errors
                        if let Some(error) = params.get("error") {
                            let error_msg = params.get("error_description").unwrap_or(error);

                            // URL decode the error message
                            let decoded_error = urlencoding::decode(error_msg)
                                .unwrap_or_else(|_| error_msg.to_string().into());

                            // Send error response
                            let html = ERROR_HTML.replace("ERROR_MESSAGE", &decoded_error);
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                                html.len(),
                                html
                            );
                            let _ = stream.write_all(response.as_bytes());

                            return Err(OAuthError::AuthorizationFailed(decoded_error.to_string()));
                        }

                        // Get code and state
                        let code = params.get("code").ok_or_else(|| {
                            OAuthError::InvalidResponse("Missing code parameter".into())
                        })?;
                        let state = params.get("state").ok_or_else(|| {
                            OAuthError::InvalidResponse("Missing state parameter".into())
                        })?;

                        // Verify state
                        if *state != expected_state {
                            let html = ERROR_HTML
                                .replace("ERROR_MESSAGE", "State mismatch - possible CSRF attack");
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                                html.len(),
                                html
                            );
                            let _ = stream.write_all(response.as_bytes());

                            return Err(OAuthError::AuthorizationFailed("State mismatch".into()));
                        }

                        // Send success response
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                            SUCCESS_HTML.len(),
                            SUCCESS_HTML
                        );
                        let _ = stream.write_all(response.as_bytes());

                        tracing::info!("OAuth callback received successfully");

                        return Ok(CallbackResult {
                            code: code.to_string(),
                            state: state.to_string(),
                        });
                    }
                }

                // Not our callback, send 404
                let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
                let _ = stream.write_all(response.as_bytes());
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No connection yet, wait a bit
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(OAuthError::IoError(e));
            }
        }
    }
}

/// Build a redirect URI for the given port and path
pub fn build_redirect_uri(host: &str, port: u16, path: &str) -> String {
    let clean_path = path.trim_start_matches('/');
    format!("http://{}:{}/{}", host, port, clean_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn test_find_available_port() {
        if TcpListener::bind(("127.0.0.1", 0)).is_err() {
            return;
        }
        // This should find a port in a high range
        let result = find_available_port((49152, 49200));
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_redirect_uri() {
        assert_eq!(
            build_redirect_uri("localhost", 8080, "callback"),
            "http://localhost:8080/callback"
        );
        assert_eq!(
            build_redirect_uri("127.0.0.1", 8080, "/callback"),
            "http://127.0.0.1:8080/callback"
        );
    }
}
