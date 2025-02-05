mod request;
mod processing;
mod response;

use std::io::ErrorKind;

use processing::Router;
use tokio::{
    io::AsyncWriteExt,
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener, TcpStream,
    },
};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    directory: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").await?;

    loop {
        let (socket, _) = listener.accept().await?;
        let directory = args.directory.clone();

        tokio::spawn(async move {
            handle_connection(socket, directory)
                .await
                .expect("Unable to handle the connection");
        });
    }
}

async fn handle_connection(stream: TcpStream, directory: Option<String>) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut in_buf = Vec::with_capacity(4 * 1024);
    let mut out_buf = Vec::with_capacity(4 * 1024);
    let r = Router { directory };

    loop {
        in_buf.clear();
        out_buf.clear();

        // wait until the channel is reable
        reader.readable().await?;

        let (size, closed) = load_request(&reader, &mut in_buf).await?;

        if closed {
            break Ok(());
        }

        let (request, _) = request::parse(&in_buf[..size])?;
        println!("{:?}", request);

        let resp = r.process(&request).await;
        resp.write(&mut out_buf);

        write_response(&mut writer, &out_buf).await?;

        if let Some(v) = request.header.headers.get("Connection") {
            if v == "keep-alive" {
                continue;
            }
        } else {
            break Ok(());
        }
    }
}

async fn write_response(writer: &mut OwnedWriteHalf, buf: &[u8]) -> anyhow::Result<()> {
    writer.write_all(buf).await?;
    Ok(())
}

async fn load_request(
    reader: &OwnedReadHalf,
    in_buf: &mut Vec<u8>,
) -> anyhow::Result<(usize, bool)> {
    // load all the data
    let mut size = 0;
    let mut buf = [0; 1024];

    loop {
        // wait for the stream to become readable
        reader.readable().await?;

        match reader.try_read(&mut buf) {
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => continue,
            Err(err) => Err(err)?,
            Ok(0) => break Ok((size, true)),
            Ok(n) => {
                // copy read data into main buffer
                in_buf.extend_from_slice(&buf[..n]);
                size += n;
                if n < buf.len() {
                    break Ok((size, false));
                }
            }
        }
    }
}
