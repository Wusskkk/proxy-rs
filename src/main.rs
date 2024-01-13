use anyhow::{anyhow, Result};
use tokio::{
    io,
    net::{TcpListener, TcpStream},
};

use log::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let listener = TcpListener::bind("127.0.0.1:10800").await?;
    let mut id = 0;
    loop {
        let (client, _) = listener.accept().await?;
        // 为每一条连接都生成一个新的任务，
        // `client` 的所有权将被移动到新的任务中，并在那里进行处理
        tokio::spawn(async move {
            if let Err(e) = handle_connection(client, id).await {
                error!("error while handling stream:\n{}", e);
            };
        });
        id = id + 1;
    }
}

async fn handle_connection(mut client: TcpStream, id: u64) -> Result<()> {
    let mut buf = [0; 4096];

    client.readable().await?;

    let n = match client.try_read(&mut buf) {
        Ok(0) => return Err(anyhow!("no data in the first request form client!")),
        Ok(n) => {
            info!("{}: received {n} bytes from the request", id);
            n
        }
        Err(e) => return Err(anyhow!("error while read request:\n{}", e)),
    };

    let req_str = std::str::from_utf8(&buf[..n]).unwrap();
    let mut req = req_str.split_whitespace();
    let method = req.nth(0).unwrap();
    let host = req.nth(3).unwrap();

    let mut server: TcpStream;

    if method == "CONNECT" {
        info!("{}: connect to {}", id, host);

        server = match TcpStream::connect(host).await {
            Ok(s) => s,
            Err(e) => return Err(anyhow!("error while connect to server:\n{}", e)),
        };

        client.writable().await?;

        match client.try_write(b"HTTP/1.1 200 Connection Established\r\n\r\n") {
            Ok(_) => (),
            Err(e) => return Err(anyhow!("error while write data to client:\n{}", e)),
        };
    } else {
        let host = format!("{}:80", host);
        info!("{}: connect to {}", id, host);

        server = match TcpStream::connect(host).await {
            Ok(s) => s,
            Err(e) => return Err(anyhow!("error while connect to server:\n{}", e)),
        };

        server.writable().await?;

        match server.try_write(&buf) {
            Ok(_) => (),
            Err(e) => return Err(anyhow!("error while write data to client:\n{}", e)),
        };
    }

    let (mut client_reader, mut client_writer) = client.split();
    let (mut server_reader, mut server_writer) = server.split();

    tokio::try_join!(
        io::copy(&mut client_reader, &mut server_writer),
        io::copy(&mut server_reader, &mut client_writer)
    )?;

    Ok(())
    // let mut nbytes_client: usize = 0;
    // let mut nbytes_server: usize = 0;

    // let mut client_buf: [u8; 4096] = [0; 4096];
    // let mut server_buf: [u8; 4096] = [0; 4096];

    // loop {
    //     if nbytes_client == 0 {
    //         // client.readable().await?;
    //         nbytes_client = match client.try_read(&mut client_buf) {
    //             Ok(0) => {
    //                 info!("{}: no data from client", id);
    //                 return Ok(());
    //             }
    //             Ok(n) => n,
    //             Err(ref e) if e.kind() == ErrorKind::WouldBlock => 0,
    //             Err(e) => return Err(anyhow!("error while read from client:\n{}", e)),
    //         }
    //     }
    //     // let req = String::from_utf8_lossy(&client_buf[..n]);
    //     // info!("{}: received data from client:\n{}", id, req);

    //     if nbytes_client > 0 {
    //         server.writable().await?;
    //         match server.try_write(&mut client_buf[..nbytes_client]) {
    //             Ok(0) => return Ok(()),
    //             Ok(n) if n == nbytes_client => nbytes_client = 0,
    //             Ok(_) => return Ok(()),
    //             Err(ref e) if e.kind() == ErrorKind::WouldBlock => (),
    //             Err(e) => return Err(anyhow!("error while write to server:\n{}", e)),
    //         }
    //     }

    //     if nbytes_server == 0 {
    //         // server.readable().await?;
    //         nbytes_server = match server.try_read(&mut server_buf) {
    //             Ok(0) => {
    //                 info!("{}: no data from client", id);
    //                 return Ok(());
    //             }
    //             Ok(n) => n,
    //             Err(ref e) if e.kind() == ErrorKind::WouldBlock => 0,
    //             Err(e) => return Err(anyhow!("error while read from server:\n{}", e)),
    //         }
    //     }
    //     // let req = String::from_utf8_lossy(&server_buf[..n]);
    //     // info!("{}: received data from server:\n{}", id, req);

    //     if nbytes_server > 0 {
    //         client.writable().await?;
    //         match client.try_write(&mut server_buf[..nbytes_server]) {
    //             Ok(0) => return Ok(()),
    //             Ok(n) if n == nbytes_server => nbytes_server = 0,
    //             Ok(_) => return Ok(()),
    //             Err(ref e) if e.kind() == ErrorKind::WouldBlock => (),
    //             Err(e) => return Err(anyhow!("error while write to client:\n{}", e)),
    //         }
    //     }
    //     sleep(Duration::from_millis(1)).await;
    // }
}
