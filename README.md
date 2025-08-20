# coretech-wirestorm-challenge
Solution to CoreTech's Operation WIRE STORM challenge.

To build and run solution:
  - Open a terminal and navigate to the project folder (folder containing Cargo.toml)
  - Run the commands "cargo build" and then "cargo run" to run the solution
  - Once the solution is running, it is now able to accept connections from both a source client and destination client(s).

Expected usage:
  - A source client must be connected to port 33333, and destination clients must connect to port 44444.
  - The source client must send messages with the correct CTMP header. Invalid messages will be dropped.
  - Destination clients can join/disconnect whenever, and will be safely removed when they disconnect.
  - A source client can only join if there is no current source client connected. A source client can disconnect at any time.
