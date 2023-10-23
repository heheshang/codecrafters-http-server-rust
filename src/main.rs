use std::thread;
use std::{
    io::{BufRead, Write},
    net::{TcpListener, TcpStream},
};
fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        // match stream {
        //     Ok(stream) => {
        //         println!("new connection: {}", stream.peer_addr().unwrap());
        //         handle_connection(stream);
        //     }
        //     Err(e) => {
        //         println!("error: {}", e);
        //     }
        // }
        // multi-threaded
        thread::spawn(|| handle_connection(stream.unwrap()));
    }
}
fn handle_connection(mut stream: TcpStream) {
    let mut reader = std::io::BufReader::new(&stream);
    let mut lines: Vec<String> = Vec::new();

    loop {
        let mut buffer = String::new();
        reader.read_line(&mut buffer).unwrap();
        let buffer = buffer.trim().to_owned();
        if buffer.is_empty() {
            break;
        }
        lines.push(buffer);
    }
    let first_line = lines.first().unwrap();
    let binding = "".to_string();
    let ua = lines.get(2).unwrap_or(&binding);
    let ua = parse_ua_line(ua);
    let (_method, uri, _version) = parse_request_line(first_line);
    match route(uri, ua) {
        Some(c) => {
            let response = generate_response(&c);
            stream.write_all(response.as_bytes()).unwrap();
        }
        None => {
            let response = "HTTP/1.1 404 Not Found\r\n\r\n";
            stream.write_all(response.as_bytes()).unwrap();
        }
    }
}
fn parse_ua_line(ua: &str) -> String {
    if ua.len() < 12 {
        return String::from("");
    }
    String::from(&ua[12..])
}
fn parse_request_line(first_line: &str) -> (String, String, String) {
    let mut method = String::new();
    let mut uri = String::new();
    let mut version = String::new();
    for (i, val) in first_line.split_whitespace().enumerate() {
        match i {
            0 => method = val.to_string(),
            1 => uri = val.to_string(),
            2 => version = val.to_string(),
            _ => {}
        }
    }
    (method, uri, version)
}

fn route(uri: String, ua: String) -> Option<String> {
    let sections = uri.split('/').collect::<Vec<&str>>();
    println!("{:?}", sections);
    if sections.len() < 2 {
        return None;
    }
    let v = sections.get(1).unwrap();
    match *v {
        "" => Some(String::from("")),
        "echo" => Some(sections[2..].join("/")),
        "user-agent" if sections.len() == 2 => Some(ua),
        _ => None,
    }
}

fn generate_response(c: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        c.len(),
        c
    )
}
