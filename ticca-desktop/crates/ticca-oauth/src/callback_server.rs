//! Local HTTP server for OAuth callbacks

use std::net::TcpListener;
use std::io::{Read, Write};
use std::time::Duration;
use crate::common::{OAuthError, OAuthResult};

/// HTML response page for successful auth
const SUCCESS_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
    <title>Authorization Complete</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
            color: #e0e0e0;
        }
        .container {
            text-align: center;
            padding: 40px;
            background: rgba(255, 255, 255, 0.05);
            border-radius: 16px;
            backdrop-filter: blur(10px);
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
        }
        h1 { color: #4ade80; margin-bottom: 16px; }
        p { color: #a0a0a0; }
        .icon { font-size: 64px; margin-bottom: 20px; }
    </style>
</head>
<body>
    <div class="container">
        <div class="icon">✅</div>
        <h1>Authorization Complete!</h1>
        <p>You can close this window and return to Ticca Desktop.</p>
    </div>
</body>
</html>"#;

/// HTML response page for errors
const ERROR_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
    <title>Authorization Failed</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
            color: #e0e0e0;
        }
        .container {
            text-align: center;
            padding: 40px;
            background: rgba(255, 255, 255, 0.05);
            border-radius: 16px;
            backdrop-filter: blur(10px);
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
        }
        h1 { color: #f87171; margin-bottom: 16px; }
        p { color: #a0a0a0; }
        .icon { font-size: 64px; margin-bottom: 20px; }
        .error { color: #ef4444; font-family: monospace; margin-top: 16px; }
    </style>
</head>
<body>
    <div class="container">
        <div class="icon">❌</div>
        <h1>Authorization Failed</h1>
        <p>Something went wrong during authorization.</p>
        <p class="error">ERROR_MESSAGE</p>
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
        "No available port in range {}-{}", range.0, range.1
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
    
    listener.set_nonblocking(true)
        .map_err(|e| OAuthError::CallbackServerError(format!("Failed to set non-blocking: {}", e)))?;
    
    let start = std::time::Instant::now();
    let expected_path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path)
    };
    
    tracing::info!("OAuth callback server listening on port {} for path {}", port, expected_path);
    
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
                stream.set_read_timeout(Some(Duration::from_secs(5)))
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
                if let Some(path_and_query) = request_line.split_whitespace().nth(1) {
                    if path_and_query.starts_with(&expected_path) {
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
                                let error_msg = params.get("error_description")
                                    .unwrap_or(error);
                                
                                // URL decode the error message
                                let decoded_error = urlencoding::decode(error_msg)
                                    .unwrap_or_else(|_| error_msg.to_string().into());
                                
                                // Send error response
                                let html = ERROR_HTML.replace("ERROR_MESSAGE", &decoded_error);
                                let response = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                                    html.len(), html
                                );
                                let _ = stream.write_all(response.as_bytes());
                                
                                return Err(OAuthError::AuthorizationFailed(decoded_error.to_string()));
                            }
                            
                            // Get code and state
                            let code = params.get("code")
                                .ok_or_else(|| OAuthError::InvalidResponse("Missing code parameter".into()))?;
                            let state = params.get("state")
                                .ok_or_else(|| OAuthError::InvalidResponse("Missing state parameter".into()))?;
                            
                            // Verify state
                            if *state != expected_state {
                                let html = ERROR_HTML.replace("ERROR_MESSAGE", "State mismatch - possible CSRF attack");
                                let response = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                                    html.len(), html
                                );
                                let _ = stream.write_all(response.as_bytes());
                                
                                return Err(OAuthError::AuthorizationFailed("State mismatch".into()));
                            }
                            
                            // Send success response
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                                SUCCESS_HTML.len(), SUCCESS_HTML
                            );
                            let _ = stream.write_all(response.as_bytes());
                            
                            tracing::info!("OAuth callback received successfully");
                            
                            return Ok(CallbackResult {
                                code: code.to_string(),
                                state: state.to_string(),
                            });
                        }
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
    
    #[test]
    fn test_find_available_port() {
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
