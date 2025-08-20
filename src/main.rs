use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::sync::{Arc, Mutex};

fn main() -> std::io::Result<()> {
    // Vector containing the TcpStreams of the destination source clients.
    // Arc<> and Mutex<> used to ensure thread safety when accessing these destination client streams.
    let destinations: Arc<Mutex<Vec<TcpStream>>> = Arc::new(Mutex::new(Vec::new()));

    // Clone of destinations to be owned by thread accepting destination clients.
    let dest_list = Arc::clone(&destinations);

    // TcpListener for the single source client.
    let source_listener = TcpListener::bind("0.0.0.0:33333")?;

    // TcpListener for the destination clients.
    let dest_listener = TcpListener::bind("0.0.0.0:44444")?;

    // Thread to run continuously in the background, accepting new destination clients.
    thread::spawn(move || {
        for stream in dest_listener.incoming() {
            if let Ok(stream) = stream {
                dest_list.lock().unwrap().push(stream);
            }
        }
    });

    // Loop to run continuously. This loop is necessary to allow for a new source client to connect if the current client disconnects.
    loop {
        let (source_stream, _) = source_listener.accept()?;
        handle_source(source_stream, &destinations)?;
    }
}

/// Handles the messages sent by the current source client. Function is exited when the source disconnects.
fn handle_source(mut source_stream: TcpStream, destinations: &Arc<Mutex<Vec<TcpStream>>>) -> std::io::Result<()> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut read_buffer = [0u8; 1024];

    loop {
        let bytes_read = source_stream.read(&mut read_buffer)?;
        if bytes_read == 0 { // If the source disconnects, exit the loop.
            break;
        }
        buffer.extend_from_slice(&read_buffer[..bytes_read]);

        // Loop to process all of the complete messages in the buffer.
        loop {
            // Look for magic byte.
            if let Some(pos) = buffer.iter().position(|&b| b == 0xCC) {
                if pos > 0 {
                    // Discard anything before the magic byte.
                    buffer.drain(..pos);
                }

                if buffer.len() < 8 { // Message is not long enough to have the full header yet.
                    break;
                }

                let length_bytes = &buffer[2..4];
                let length = u16::from_be_bytes([length_bytes[0], length_bytes[1]]) as usize;

                if buffer.len() < 8 + length { // Full message has not been received yet.
                    break;
                }

                let message: Vec<u8> = buffer.drain(..8 + length).collect();

                // Broadcast message to destination clients.
                // Vector to contain the indexes of the destination clients to be removed from destinations vector.
                let mut to_remove = Vec::new();
                let mut list = destinations.lock().unwrap();
                for (i, dest) in list.iter_mut().enumerate() {
                    if let Err(_) = dest.write_all(&message) { // If there is an error with the connection with a destination client, add it to the list of clients to be removed.
                        to_remove.push(i);
                    }
                }
                // Clients are removed in reverse order in order to not offset the indexes of the other clients to be removed.
                for i in to_remove.into_iter().rev() {
                    list.remove(i);
                }
            } else {
                // If there is no magic byte found, discard everything in the buffer.
                buffer.clear();
                break;
            }
        }
    }

    Ok(())
}
