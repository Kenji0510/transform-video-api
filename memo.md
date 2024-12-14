use base64::decode;
use std::process::Stdio;
use tokio::{fs::File, io::AsyncWriteExt, process::Command};

use axum::{
extract::ws::{WebSocket, WebSocketUpgrade},
response::IntoResponse,
routing::get,
Router,
};

use futures_util::TryStreamExt;

async fn handle_ws(ws: WebSocketUpgrade) -> impl IntoResponse {
println!("--> {:12} - Accessed /ws", "HANDLER");
ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
while let Ok(Some(message)) = socket.try_next().await {
match message {
// axum::extract::ws::Message::Text(msg) => {
// println!("Received message: {}", msg);

            //     if let Err(err) = execute_swift_program(msg).await {
            //         eprintln!("Error executed with swift program: {}", err);
            //     }

            //     if socket.send(axum::extract::ws::Message::Text("Hello from server!".to_string())).await.is_err() {
            //         eprintln!("Error sending message");
            //         break;
            //     }
            // },
            axum::extract::ws::Message::Text(base64_data) => {
                println!("Received Base64 data of size: {}", base64_data.len());

                //match decode_and_save_file("received_video.png", base64_data).await {
                match decode_and_save_file("received_video.mp4", base64_data).await {
                    Ok(_) => println!("File saved successfully: received_video.mp4"),
                    Err(err) => eprintln!("Error saving file: {}", err),
                }

                match  execute_swift_program(input)
            }
            axum::extract::ws::Message::Binary(data) => {
                println!("Received binary data of size: {:?}", data.len());

                if let Err(err) = save_file("received_video.png", data).await {
                    eprintln!("Error saving file: {}", err);
                } else {
                    println!("File saved successfully");
                }
            }
            axum::extract::ws::Message::Close(_) => break,
            _ => {}
        }
    }

}

async parse_request_data(data: String) -> Result<(), Box<dyn std::error::Err

async fn decode_and_save_file(
file_name: &str,
base64_data: String,
) -> Result<(), Box<dyn std::error::Error>> {
let base64_data = if let Some(index) = base64_data.find(",") {
&base64_data[index + 1..]
} else {
&base64_data
};

    let binary_data = decode(base64_data)?;

    let file_path = format!(
        "/home/kenji/workspace/Rust/websocket-server/uploads/{}",
        file_name
    );
    let mut file = File::create(file_path).await?;
    file.write_all(&binary_data).await?;
    file.flush().await?;
    Ok(())

}

async fn save_file(file_path: &str, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
let mut file = File::create(file_path).await?;
file.write(&data).await?;
file.flush().await?;
Ok(())
}

async fn execute_swift_program(input: String) -> Result<(), Box<dyn std::error::Error>> {
let command = format!(
"ffmpeg -i {} -c:v libx265 -preset faster -crf 28 -c:a aac output.mp4",
input
);
let output = Command::new(command)
.stdout(Stdio::piped())
.spawn()?
.wait_with_output()
.await?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("FFmpeg output: {}", stdout);
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("FFmpeg error: {}", stderr);
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("FFmpeg failed with error: {}", stderr),
        )))
    }

}

#[tokio::main]
async fn main() {
let app = Router::new().route("/ws", get(handle_ws));

    println!("--> {:12} - Started running server on port 8080", "INFO");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();

}
