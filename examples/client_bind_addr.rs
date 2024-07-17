extern crate fstack_mio as mio;
use std::env;
use std::io::Read;
use std::net::SocketAddr;
use bytes::{Buf, Bytes};
use http::Request;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::net::TcpStream;
use http_body_util::{BodyExt, Empty};

#[path = "../benches/support/mod.rs"]
mod support;
use support::TokioIo;

// ./ target/debug/client_bind_addr ifconfig.me 34.160.111.145:80 192.168.8.107:0
fn main() {
    let mut args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("Specify ip and port, example: ./a 127.0.0.1:8070");
    };
    let url = args[1].clone();
    let remote_ip = args[2].clone();
    let local_ip = args[3].clone();
    args.remove(1);
    args.remove(1);
    args.remove(1);
    tokio::fstack_init(args.len(), args);

    pretty_env_logger::init();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let url = url.parse::<hyper::Uri>().expect("invalid url");
    let remote_addr: SocketAddr = remote_ip.parse().expect("invalid remote ip");
    let local_addr: SocketAddr = local_ip.parse().expect("invalid local ip");

    rt.block_on(async {
        let local = tokio::task::LocalSet::new();
        local.run_until(async move {
            let host = url.host().expect("uri has no host");
            let port = url.port_u16().unwrap_or(80);
            let addr = format!("{}:{}", host, port);
            println!("addr: {}", addr);

            let stream = mio::sys::unix::TcpStream::connect_from_bind(remote_addr, local_addr).expect("connect failed");
            let stream = TcpStream::from_std(stream).expect("from_std failed");
            let io = TokioIo::new(stream);

            match hyper::client::conn::http1::handshake(io).await {
                Ok((mut client, connection)) => {
                    println!("Handshake successful!");

                    tokio::task::spawn_local(async move {
                        if let Err(e) = connection.await {
                            eprintln!("Connection error: {}", e);
                        }
                    });

                    let req = Request::builder()
                        .method("GET")
                        .uri(url)
                        .body(Empty::<Bytes>::new())
                        .unwrap();

                    match client.send_request(req).await {
                        Ok(mut response) => {
                            println!("Response: {}", response.status());
                            let mut reader = response.collect().await.unwrap().aggregate().reader();
                            let mut body_str = String::new();
                            reader.read_to_string(&mut body_str).unwrap();

                            println!("Public IP address for sending data: {}", body_str);
                        }
                        Err(e) => {
                            eprintln!("Request error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("handshake failed: {}", e);
                }
            }
        }).await;
    });
}

