use std::env;
use std::fs;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::thread;
fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => {
                // println!("new connection: {}", _stream.peer_addr().unwrap());
                thread::spawn(move || {
                    handle_connection(_stream);
                });
                ()
                // handle_connection(_stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
        // multi-threaded
        // thread::spawn(|| handle_connection(stream.unwrap()));
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];

    stream.read(&mut buffer).unwrap();

    let req_string = String::from_utf8_lossy(&buffer[..]);
    let req_lines: Vec<&str> = req_string.split('\n').collect();
    eprintln!("req_lines: {:?}", req_lines);
    let first_line: Vec<&str> = match req_lines.first() {
        Some(line) => line.split_whitespace().collect(),
        None => vec![""],
    };

    let req_path = first_line[1];
    let path_parts = req_path.split('/').collect::<Vec<&str>>();
    eprintln!("path_parts: {:?}", path_parts);
    let mut res = String::new();

    match path_parts[1] {
        "echo" => {
            res.push_str("HTTP/1.1 200 OK\r\n");
            res.push_str("Content-Type: text/plain\r\n");
            let echo_idx = req_path.find("/echo").unwrap();
            let echo_str = req_path.split_at(echo_idx).1;
            // Remove "/echo/"
            let rest = echo_str.split_at(6).1;
            res.push_str(&format!("Content-Length: {}\r\n\r\n", rest.len()));
            res.push_str(rest);
        }
        "user-agent" => {
            res.push_str("HTTP/1.1 200 OK\r\n");
            res.push_str("Content-Type: text/plain\r\n");

            let user_agent = req_lines
                .iter()
                .find(|line| line.starts_with("User-Agent:"))
                .unwrap_or(&"");
            println!("user_agent: {}", user_agent);
            let user_content = user_agent.split_at(12).1;
            res.push_str(&format!(
                "Content-Length: {}\r\n\r\n",
                user_content.len() - 1
            ));
            res.push_str(user_content);
        }
        "files" => {
            let args = env::args().collect::<Vec<String>>();
            let dir_arg_idx = args.iter().position(|arg| arg == "--directory");
            match dir_arg_idx {
                Some(idx) => {
                    let dir = &args[idx + 1];
                    let file_path_idx = req_path.find("/files").unwrap();
                    let param = req_path.split_at(file_path_idx).1;
                    let file_name = param.split_at(7).1;
                    let file_path = format!("{}/{}", dir, file_name);

                    match fs::metadata(&file_path) {
                        Ok(_) => {
                            res.push_str("HTTP/1.1 200 OK\r\n");
                            res.push_str("Content-Type: application/octet-stream\r\n");

                            let mut len = 0;
                            let mut contents = String::new();

                            for line in fs::read_to_string(file_path).unwrap().lines() {
                                len += line.len();
                                contents.push_str(format!("{}\r\n", line).as_str());
                            }
                            res.push_str(&format!("Content-Length: {}\r\n\r\n", len));
                            res.push_str(contents.as_str());
                        }
                        Err(_) => res.push_str(stringify!("HTTP/1.1 404 Not Found\r\n\r\n")),
                    }
                }
                None => res.push_str(stringify!("HTTP/1.1 404 Not Found\r\n\r\n")),
            };
        }
        "" => res.push_str("HTTP/1.1 200 OK\r\n\r\n"),
        _ => res.push_str("HTTP/1.1 404 Not Found\r\n\r\n"),
    }
    let _ = stream.write_all(res.as_bytes());
}
