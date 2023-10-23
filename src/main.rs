// Uncomment this block to pass the first stage
use std::{
    io::{BufRead, Write},
    net::{TcpListener, TcpStream},
};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("new connection: {}", stream.peer_addr().unwrap());
                handle_connection(stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
fn handle_connection(mut stream: TcpStream) {
    // let response = "HTTP/1.1 200 OK\r\n\r\n";
    // stream.write_all(response.as_bytes()).unwrap();
    // let mut buffer = [0; 1024];
    // let bytes_read = stream
    //     .read(&mut buffer)
    //     .expect("Failed to read from connection");

    // println!(
    //     "Received : {}",
    //     String::from_utf8_lossy(&buffer[..bytes_read])
    // );
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

    let path = get_path(first_line);

    match path.as_str() {
        "/" => {
            let response = "HTTP/1.1 200 OK\r\n\r\n";
            stream.write_all(response.as_bytes()).unwrap();
        }
        "/echo" => {
            let length = path.rsplit_once('/').unwrap().0.len();
            let content = path.rsplit_once('/').unwrap().0;
            let response = "HTTP/1.1 200 OK\r\n";
            stream.write_all(response.as_bytes()).unwrap();
            let content_type = "Content-Type: text/plain\r\n";
            let content_length = format!("Content-Length: {}\r\n\r\n", length);
            stream.write_all(content_type.as_bytes()).unwrap();
            stream.write_all(content_length.as_bytes()).unwrap();
            stream.write_all(content.as_bytes()).unwrap();
        }
        _ => {
            let response = "HTTP/1.1 404 Not Found\r\n\r\n";
            stream.write_all(response.as_bytes()).unwrap();
        }
    }
}

fn get_path(first_line: &String) -> String {
    for (i, val) in first_line.split_whitespace().enumerate() {
        if i == 1 {
            return val.to_string();
        }
    }
    "".to_string()
}
