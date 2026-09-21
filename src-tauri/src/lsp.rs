use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

pub async fn start_lsp_server() -> Result<u16, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(handle_connection(stream));
        }
    });
    
    Ok(port)
}

async fn handle_connection(stream: TcpStream) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("Error during websocket handshake: {}", e);
            return;
        }
    };
    
    let (mut ws_write, mut ws_read) = ws_stream.split();
    
    // Resolve texlab path. During dev it's in src-tauri/bin.
    // In prod, Tauri sidecars are placed next to the executable.
    let current_exe = std::env::current_exe().unwrap();
    let dir = current_exe.parent().unwrap();
    
    // Naive resolution for development vs production
    let texlab_path = if dir.join("texlab-x86_64-pc-windows-msvc.exe").exists() {
        dir.join("texlab-x86_64-pc-windows-msvc.exe")
    } else if dir.join("bin/texlab-x86_64-pc-windows-msvc.exe").exists() {
        dir.join("bin/texlab-x86_64-pc-windows-msvc.exe")
    } else if dir.join("../bin/texlab-x86_64-pc-windows-msvc.exe").exists() {
        dir.join("../bin/texlab-x86_64-pc-windows-msvc.exe")
    } else {
        // Fallback to system path
        std::path::PathBuf::from("texlab")
    };

    let mut cmd = Command::new(&texlab_path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = match cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to spawn texlab: {}", e);
            return;
        }
    };
    
    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let mut stdout = child.stdout.take().expect("Failed to open stdout");
    
    // Thread to read from websocket and write to texlab stdin
    let stdin_task = tokio::spawn(async move {
        while let Some(msg) = ws_read.next().await {
            if let Ok(Message::Text(text)) = msg {
                let rpc_msg = format!("Content-Length: {}\r\n\r\n{}", text.len(), text);
                if stdin.write_all(rpc_msg.as_bytes()).await.is_err() {
                    break;
                }
            }
        }
    });
    
    // Thread to read from texlab stdout and write to websocket
    let stdout_task = tokio::spawn(async move {
        loop {
            // Read headers
            let mut headers = String::new();
            let mut content_length = 0;
            loop {
                let mut buf = [0u8; 1];
                let mut line = String::new();
                loop {
                    match stdout.read_exact(&mut buf).await {
                        Ok(_) => {
                            let c = buf[0] as char;
                            line.push(c);
                            if line.ends_with("\r\n") {
                                break;
                            }
                        }
                        Err(_) => return, // EOF or error
                    }
                }
                
                if line == "\r\n" {
                    break;
                }
                
                if line.to_lowercase().starts_with("content-length:") {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() == 2 {
                        if let Ok(len) = parts[1].trim().parse::<usize>() {
                            content_length = len;
                        }
                    }
                }
            }
            
            if content_length > 0 {
                let mut content_buf = vec![0u8; content_length];
                if stdout.read_exact(&mut content_buf).await.is_ok() {
                    if let Ok(text) = String::from_utf8(content_buf) {
                        if ws_write.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
        }
    });
    
    tokio::select! {
        _ = stdin_task => (),
        _ = stdout_task => (),
    }
    
    let _ = child.kill().await;
}
