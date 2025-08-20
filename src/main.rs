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

/// Function to handle the messages sent by the current source client. Function is exited when the source disconnects.
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

                let options = buffer[1];
                let sensitive_bit = (options >> 6) & 1;

                let checksum_bytes = &buffer[4..6];
                let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

                if buffer.len() < 8 + length { // Full message has not been received yet.
                    break;
                }

                let message: Vec<u8> = buffer.drain(..8 + length).collect();

                if sensitive_bit == 1 && !verify_checksum(&message, checksum) { // Checksum is checked and is not correct.
                    eprintln!("Checksum invalid for message: {:?}, message dropped.", message);
                    break;
                }

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

/// Function to verify the stated checksum against the given message and return an appropriate true/false value.
fn verify_checksum(message: &[u8], checksum: u16) -> bool {
    let mut sum: u32 = 0;

    // Iterate over each 2-byte word in the message.
    let mut message_index = 0;
    while message_index < message.len() {
        let word = if message_index == 4 { // The 2 bytes contained in the checksum field.
            0xCCCCu16
        } else {
            let high_byte = message[message_index] as u16;
            let low_byte = if message_index + 1 < message.len() { message[message_index + 1] as u16 } else { 0 };
            (high_byte << 8) | low_byte
        };

        sum += word as u32; // Store sum as 32 bit to avoid any overflow.

        while sum > 0xFFFF {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        message_index += 2;
    }

    let computed = !(sum as u16); // Finds the one's complement of the calculated sum.
    computed == checksum
}