use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use itertools::Itertools;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

#[derive(Debug, PartialEq)]
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
#[derive(Debug)]
struct HttpRequest<'a> {
    method: HttpMethod,
    path: &'a str,
    _version: &'a str,
    headers: HashMap<&'a str, &'a str>,
    body: Option<&'a [u8]>,
}
impl HttpRequest<'_> {
    fn form_req_str(buffer: &str) -> Result<HttpRequest> {
        println!("buffer: {}", buffer);
        let mut lines = buffer.split("\r\n");

        let (method, path, version) = &lines
            .next()
            .ok_or(anyhow!("Invalid frame"))?
            .split(' ')
            .collect_tuple()
            .ok_or(anyhow!("Invalid frame"))?;
        println!("method: {}", method);
        println!("path: {}", path);
        println!("version: {}", version);
        let headers: HashMap<_, _> = lines
            .by_ref()
            .map_while(|l| {
                if let Some((k, v)) = l.split_once(": ") {
                    Some((k.trim(), v.trim()))
                } else {
                    None
                }
            })
            .collect();
        println!("headers: {:?}", headers);
        let body = match lines.next() {
            Some(body_data) if !body_data.is_empty() => {
                let data_len = headers
                    .get("Content-Length")
                    .ok_or(anyhow!("No content length"))?
                    .parse()?;
                Some(&body_data.as_bytes()[0..data_len])
            }
            _ => None,
        };

        println!("body: {:?}", body);
        Ok(HttpRequest {
            method: HttpMethod::from(*method),
            path,
            _version: version,
            headers,
            body,
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
    println!("Incoming connection from {}", stream.peer_addr()?);
    let mut buffer = [0; 512];

    let size = stream.read(&mut buffer)?;
    println!("size: {}", size);
    let request = std::str::from_utf8(&buffer[0..size])?;
    println!("req: {}", request);
    let frame = HttpRequest::form_req_str(request)?;
    println!("frame: {:?}", frame);
    match frame.path {
        "/" => index_route(&stream),
        _ if frame.path.starts_with("/echo") => echo_route(&stream, &frame),
        _ if frame.path.starts_with("/user-agent") => uesr_angent(&stream, &frame),
        _ if frame.path.starts_with("/files/") && frame.method == HttpMethod::GET => {
            files_route(&stream, &frame, &config.dir)
        }

        _ if frame.path.starts_with("/files/") && frame.method == HttpMethod::POST => {
            files_upload(&stream, &frame, &config.dir)
        }
        _ => not_found_route(&stream),
    }?;

    Ok(())
}

fn files_upload(stream: &TcpStream, frame: &HttpRequest, dir: &Option<String>) -> Result<usize> {
    let path = frame.path.replace("/files/", "");
    println!("path: {}", path);
    let file_path = format!("{}/{}", dir.as_ref().unwrap(), path);
    let mut file = match File::create(file_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Failed to create file: {}", e);
            return not_found_route(stream);
        }
    };
    let body = frame.body.ok_or(anyhow!("No body"))?;
    let _ = file.write_all(body);
    send_created(stream)?;
    Ok(1)
}

fn send_created(mut stream: &TcpStream) -> Result<usize> {
    stream
        .write(b"HTTP/1.1 201 CREATED\r\n\r\n")
        .context("Failed to write to stream")
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
