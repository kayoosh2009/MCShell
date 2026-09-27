use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use base64::Engine;

use crate::paths;
use crate::versions::download_to;

const INJECTOR_URL: &str = "https://github.com/yushijinhun/authlib-injector/releases/download/v1.2.5/authlib-injector-1.2.5.jar";

pub fn ensure_authlib_injector() -> Result<()> {
    download_to(INJECTOR_URL, &paths::authlib_injector_jar())?;
    ensure_keypair()
}

fn ensure_keypair() -> Result<()> {
    let priv_path = paths::private_key_file();
    let pub_path = paths::public_key_der_file();
    if priv_path.is_file() && pub_path.is_file() {
        return Ok(());
    }
    std::fs::create_dir_all(paths::data_dir())?;
    std::process::Command::new("openssl")
        .arg("genrsa").arg("-out").arg(&priv_path).arg("2048")
        .output()?;
    std::process::Command::new("openssl")
        .arg("rsa").arg("-in").arg(&priv_path)
        .arg("-pubout").arg("-outform").arg("DER").arg("-out").arg(&pub_path)
        .output()?;
    Ok(())
}

fn public_key_base64() -> Result<String> {
    let bytes = std::fs::read(paths::public_key_der_file())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

fn sign(data: &[u8]) -> Result<String> {
    use std::io::Write;
    let mut child = std::process::Command::new("openssl")
        .arg("dgst").arg("-sha1").arg("-sign").arg(paths::private_key_file())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    child.stdin.take().unwrap().write_all(data)?;
    let output = child.wait_with_output()?;
    Ok(base64::engine::general_purpose::STANDARD.encode(output.stdout))
}

pub struct SkinServer {
    pub port: u16,
    stop: Arc<AtomicBool>,
}

impl SkinServer {
    pub fn start(username: String, uuid: String) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;

        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();

        std::thread::spawn(move || {
            while !stop_clone.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = handle_connection(stream, &username, &uuid, port);
                    }
                    Err(_) => std::thread::sleep(std::time::Duration::from_millis(50)),
                }
            }
        });

        Ok(Self { port, stop })
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn handle_connection(mut stream: TcpStream, username: &str, uuid: &str, port: u16) -> Result<()> {
    let mut buf = [0u8; 2048];
    let n = stream.read(&mut buf)?;
    let request = String::from_utf8_lossy(&buf[..n]);
    let path = request.lines().next().unwrap_or("").split_whitespace().nth(1).unwrap_or("/");

    if path.starts_with("/skin.png") {
        let bytes = std::fs::read(paths::skin_file()).unwrap_or_default();
        write_response(&mut stream, "image/png", &bytes)?;
    } else if path.contains("/session/minecraft/hasJoined") || path.contains("/profile/") {
        let body = profile_json(username, uuid, port);
        write_response(&mut stream, "application/json", body.as_bytes())?;
    } else {
        let pubkey = public_key_base64().unwrap_or_default();
        let body = format!(
            r#"{{"meta":{{"serverName":"MCShell","implementationName":"mcshell","implementationVersion":"0.1.0"}},"skinDomains":["127.0.0.1"],"signaturePublickey":"{pubkey}"}}"#
        );
        write_response(&mut stream, "application/json", body.as_bytes())?;
    }
    Ok(())
}

fn profile_json(username: &str, uuid: &str, port: u16) -> String {
    let skin_url = format!("http://127.0.0.1:{port}/skin.png");
    let texture = format!(
        r#"{{"timestamp":0,"profileId":"{uuid}","profileName":"{username}","textures":{{"SKIN":{{"url":"{skin_url}"}}}}}}"#
    );
    let texture_b64 = base64::engine::general_purpose::STANDARD.encode(&texture);
    let signature = sign(texture_b64.as_bytes()).unwrap_or_default();
    format!(
        r#"{{"id":"{uuid}","name":"{username}","properties":[{{"name":"textures","value":"{texture_b64}","signature":"{signature}"}}]}}"#
    )
}

fn write_response(stream: &mut TcpStream, content_type: &str, body: &[u8]) -> Result<()> {
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    Ok(())
}