use base64::decode;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::result::Result;
use tokio::fs;
use tokio::io::AsyncReadExt;
use tokio::{fs::File, io::AsyncWriteExt, process::Command};

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};

use futures_util::TryStreamExt;

#[derive(Deserialize, Clone)]
struct RequestFormat {
    file_name: String,
    video_data: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ResponseFormat {
    status: String,
    message: String,
}

async fn handle_ws(ws: WebSocketUpgrade) -> impl IntoResponse {
    println!("--> {:12} - Accessed /ws", "HANDLER");
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Ok(Some(message)) = socket.try_next().await {
        match message {
            axum::extract::ws::Message::Text(req_data) => {
                println!("--> {:12} - Received data from client", "LOGGER");

                match serde_json::from_str::<RequestFormat>(&req_data) {
                    Ok(request) => {
                        println!("File name: {}", request.file_name);
                        println!("Video data size: {}", request.video_data.len());

                        match decode_and_save_file(request.clone()).await {
                            Ok(_) => println!("File saved successfully: {}", request.file_name),
                            Err(err) => {
                                eprintln!("Error saving file: {}", err);
                                match send_status_response(
                                    &mut socket,
                                    "error".to_string(),
                                    "Failed to decode video data.".to_string(),
                                )
                                .await
                                {
                                    Ok(_) => {}
                                    Err(err) => eprintln!("Error sending response: {}", err),
                                }
                                continue;
                            }
                        }

                        match send_status_response(
                            &mut socket,
                            "success".to_string(),
                            "Converting video to HEVC".to_string(),
                        )
                        .await
                        {
                            Ok(_) => {}
                            Err(err) => {
                                eprintln!("Error sending response: {}", err);
                                continue;
                            }
                        }

                        match call_ffmpeg_for_hevc(request.file_name.clone()).await {
                            Ok(output_path) => {
                                println!("FFmpeg program executed successfully");

                                match send_video(&mut socket, output_path).await {
                                    Ok(_) => println!("File sent successfully"),
                                    Err(err) => eprintln!("Error sending file: {}", err),
                                }
                            }
                            Err(err) => eprintln!("Error executing FFmpeg program: {}", err),
                        }

                        match cleanup_file(request.file_name.clone(), "output.mp4".to_string())
                            .await
                        {
                            Ok(_) => println!("Files cleaned up successfully"),
                            Err(err) => eprintln!("Error cleaning up files: {}", err),
                        }
                    }
                    Err(err) => {
                        println!("Error parsing JSON: {}", err);
                    }
                }
            }
            axum::extract::ws::Message::Close(_) => break,
            _ => {}
        }
    }
}

async fn cleanup_file(input_file: String, output_file: String) -> Result<(), String> {
    let input_path = format!("./uploads/{}", input_file);
    let output_path = format!("./transform_data/output.mp4");

    fs::remove_file(&input_path)
        .await
        .map_err(|e| format!("Failed to remove file: {}", e))?;

    fs::remove_file(&output_path)
        .await
        .map_err(|e| format!("Failed to remove file: {}", e))?;

    Ok(())
}

async fn send_status_response(
    socket: &mut WebSocket,
    status: String,
    message: String,
) -> Result<(), String> {
    let response = ResponseFormat {
        status: status,
        message: message,
    };
    let response_text = serde_json::to_string(&response).unwrap();
    socket
        .send(Message::Text(response_text))
        .await
        .map_err(|e| format!("Failed to send text message: {}", e))?;
    Ok(())
}

async fn send_video(socket: &mut WebSocket, output_path: String) -> Result<(), String> {
    let file_path = output_path;
    let mut file = fs::File::open(&file_path)
        .await
        .map_err(|e| format!("Failed to open file: {}", e))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;
    socket
        .send(Message::Binary(buffer))
        .await
        .map_err(|e| format!("Failed to send binary message: {}", e))?;
    println!("File sent successfully");
    Ok(())
}

async fn decode_and_save_file(request: RequestFormat) -> Result<(), String> {
    let base64_data = &request.video_data;
    let base64_data = if let Some(index) = base64_data.find(",") {
        &base64_data[index + 1..]
    } else {
        &base64_data
    };

    let binary_data =
        decode(base64_data).map_err(|e| format!("Failed to decode base64 data: {}", e))?;
    let full_path = format!("./uploads/{}", request.file_name);
    let mut file = File::create(&full_path)
        .await
        .map_err(|e| format!("Failed to create file: {}", e))?;
    if let Err(e) = file.write_all(&binary_data).await {
        tokio::fs::remove_file(&full_path)
            .await
            .map_err(|e| format!("Failed to remove file: {}", e))?;
        return Err(format!("Failed to write file: {}", e));
    }

    if let Err(e) = file.flush().await {
        tokio::fs::remove_file(&full_path)
            .await
            .map_err(|e| format!("Failed to remove file: {}", e))?;
        return Err(format!("Failed to flush file: {}", e));
    }
    Ok(())
}

async fn call_ffmpeg_for_hevc(video_name: String) -> Result<String, String> {
    let command = "ffmpeg";
    let input_file = format!("./uploads/{}", video_name);
    let output_path = format!("./transform_data/output.mp4");
    let args = [
        "-i",
        &input_file,
        "-c:v",
        "libx265",
        "-preset",
        "faster",
        "-crf",
        "28",
        "-c:a",
        "aac",
        &output_path,
    ];

    let output = Command::new(command)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to execute FFmpeg program: {}", e))?
        .wait_with_output()
        .await
        .map_err(|e| format!("Failed to execute FFmpeg program: {}", e))?;

    if output.status.success() {
        //let stdout = String::from_utf8_lossy(&output.stdout);
        //println!("FFmpeg output: {}", stdout);
        Ok(output_path)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("FFmpeg error: {}", stderr);
        Err(format!("FFmpeg program failed"))
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/ws", get(handle_ws));

    println!("--> {:12} - Started running server on port 8080", "INFO");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
