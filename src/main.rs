use std::str::FromStr;
use std::{error::Error, io::prelude::*, io::BufRead};

use hex_color::HexColor;
use nom::{branch::alt, bytes::complete::tag, combinator::map, IResult};
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::{channel, Receiver, Sender},
};

struct State {
    height: usize,
    width: usize,
    area: Vec<HexColor>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("0.0.0.0:1337").await?;
    let (tx, rx) = channel::<(Command, TcpStream)>(32);
    tokio::spawn(async {
        command_handler(rx).await;
    });
    loop {
        let (stream, _) = listener.accept().await?;
        let tx = tx.clone();
        tokio::spawn(async move {
            if let Err(e) = process(stream, tx).await {
                eprintln!("an error occurred; error = {:?}", e);
            }
        });
    }
}

async fn command_handler(mut rx: Receiver<(Command, TcpStream)>) {
    let width = 800;
    let height = 600;
    let mut state = State {
        area: vec![
            HexColor {
                r: 0,
                g: 0,
                b: 0,
                a: 255
            };
            height * width
        ],
        height,
        width,
    };
    while let Some((cmd, mut stream)) = rx.recv().await {
        match cmd {
            Command::Help => {
                let _ = stream.write_all("HELP\n".as_bytes()).await;
            }
            Command::Size => {
                let _ = stream
                    .write_all(format!("SIZE {} {}\n", state.width, state.height).as_bytes())
                    .await;
            }
            Command::GetPx { x, y } => {
                let px = state.area.get((y * state.width) + x);
                if px.is_none() {
                    return;
                }
                let px = px.unwrap();
                let px_str = &px.to_string()[1..];
                let _ = stream
                    .write_all(format!("PX {} {} {}\n", x, y, &px_str.to_lowercase()).as_bytes())
                    .await;
            }
            Command::SetPx { x, y, color } => {
                if let Some(elem) = state.area.get_mut((y * state.width) + x) {
                    *elem = color;
                }
            }
        }
    }
}

#[derive(Debug)]
enum Command {
    Help,
    Size,
    GetPx { x: usize, y: usize },
    SetPx { x: usize, y: usize, color: HexColor },
}

async fn process(
    mut stream: TcpStream,
    tx: Sender<(Command, TcpStream)>,
) -> Result<(), Box<dyn Error>> {
    let mut buf = BufReader::new(&mut stream);
    let mut data = String::with_capacity(10);
    buf.read_line(&mut data).await?;
    let cmd = match_cmd(&data)?;
    tx.send((cmd, stream)).await?;
    Ok(())
}

fn match_cmd(s: &str) -> Result<Command, Box<dyn Error>> {
    // args
    let args = s.split_whitespace().collect::<Vec<&str>>();
    let cmd = match args.get(0) {
        Some(&"HELP") => Command::Help,
        Some(&"SIZE") => Command::Size,
        Some(&"PX") => {
            let x: usize = args.get(1).ok_or("Missing X")?.parse()?;
            let y: usize = args.get(2).ok_or("Missing Y")?.parse()?;
            let color = args.get(3);
            match color {
                Some(color) => {
                    let mut c = String::from_str(color).unwrap();
                    if !c.starts_with("#") {
                        c = "#".to_string() + &c;
                    }
                    Command::SetPx {
                        x,
                        y,
                        color: HexColor::parse(&c)?,
                    }
                }
                None => Command::GetPx { x, y },
            }
        }
        _ => Err("Unknown command")?,
    };
    return Ok(cmd);
}
