use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use itertools::Itertools;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
    CONNECT,
    TRACE,
    PATCH,
}

impl From<&str> for HttpMethod {
    fn from(value: &str) -> Self {
        match value {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "DELETE" => HttpMethod::DELETE,
            "HEAD" => HttpMethod::HEAD,
            "OPTIONS" => HttpMethod::OPTIONS,
            "CONNECT" => HttpMethod::CONNECT,
            "TRACE" => HttpMethod::TRACE,
            "PATCH" => HttpMethod::PATCH,
            _ => HttpMethod::GET,
        }
    }
}
struct HttpRequest {
    method: HttpMethod,
    path: String,
    version: String,
    headers: HashMap<String, String>,
}
impl HttpRequest {
    fn form_req_str(buffer: &str) -> Result<HttpRequest> {
        let mut lines = buffer.split("\r\n");
        let (method, path, version) = lines
            .next()
            .ok_or(anyhow!("Invalid frame"))?
            .split(' ')
            .collect_tuple()
            .ok_or(anyhow!("Invalid frame"))?;
        let headers: HashMap<_, _> = lines
            .filter_map(|l| {
                if let Some((k, v)) = l.split_once(": ") {
                    Some((k.trim().to_string(), v.trim().to_string()))
                } else {
                    None
                }
            })
            .collect();

        Ok(HttpRequest {
            method: HttpMethod::from(method),
            path: path.to_string(),
            version: version.to_string(),
            headers,
        })
    }
}

struct Config {
    dir: Option<String>,
}

fn main() -> Result<()> {
    let linster = TcpListener::bind("127.0.0.1:4221")?;
    let args = env::args().collect::<Vec<String>>();
    let dir = if args.len() > 2 && args[1] == "--directory" {
        Some(args[2].clone())
    } else {
        None
    };
    println!("dir: {:?}", &dir);
    let config = Arc::new(Config { dir });
    for stream in linster.incoming() {
        match stream {
            Ok(stream) => {
                let c_config = config.clone();
                thread::spawn(move || handle_connection(stream, c_config));
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream, config: Arc<Config>) -> Result<()> {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer)?;
    let req = std::str::from_utf8(&buffer)?;
    let frame = HttpRequest::form_req_str(req)?;
    match frame.path.as_str() {
        "/" => index_route(&stream),
        _ if frame.path.starts_with("/echo") => echo_route(&stream, &frame),
        _ if frame.path.starts_with("/user-agent") => uesr_angent(&stream, &frame),
        _ if frame.path.starts_with("/files/") => files_route(&stream, &frame, &config.dir),

        _ => not_found_route(&stream),
    }?;

    Ok(())
}

fn uesr_angent(
    stream: &TcpStream,
    frame: &HttpRequest,
) -> std::result::Result<usize, anyhow::Error> {
    let user_agent = frame
        .headers
        .get("User-Agent")
        .ok_or(anyhow!("User-Agent not found"))?;

    send_text_plain(stream, user_agent)
}

fn files_route(stream: &TcpStream, frame: &HttpRequest, dir: &Option<String>) -> Result<usize> {
    let dir = dir.as_ref().ok_or(anyhow!("No directory provided"))?;
    let mut path = PathBuf::from(dir);
    path.push(&frame.path[7..]);
    if let Ok(data) = fs::read(path).context("Failed to read file") {
        send_binary(stream, &data)
    } else {
        not_found_route(stream)
    }
}

fn send_binary(mut stream: &TcpStream, data: &[u8]) -> std::result::Result<usize, anyhow::Error> {
    let mut resp = "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\n".to_string();
    resp.push_str(&format!("Content-Length: {}\r\n\r\n", data.len()));
    let mut res_bytes = Vec::from(resp.as_bytes());
    res_bytes.extend(data);
    stream
        .write(&res_bytes)
        .context("Failed to write to stream")
}

fn echo_route(stream: &TcpStream, frame: &HttpRequest) -> Result<usize> {
    send_text_plain(stream, &frame.path[6..])
}

fn send_text_plain(mut stream: &TcpStream, text: &str) -> Result<usize> {
    let mut data = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n".to_string();
    data.push_str(&format!("Content-Length: {}\r\n\r\n", text.len()));
    data.push_str(text);
    let bytes = stream
        .write(data.as_bytes())
        .context("Failed to write to stream")?;
    Ok(bytes)
}

fn not_found_route(mut stream: &TcpStream) -> Result<usize> {
    stream
        .write(b"HTTP/1.1 404 NOT FOUND\r\n\r\n")
        .context("Failed to write to stream")
}

fn index_route(mut stream: &TcpStream) -> Result<usize> {
    stream
        .write(b"HTTP/1.1 200 OK\r\n\r\n")
        .context("Failed to write to stream")
}
