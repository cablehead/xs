use std::net::UdpSocket;
use std::sync::Once;

// Start windows socket library - From socket2-rs
pub(crate) fn init_winsock() {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        // Initialize winsock through the standard library by just creating a
        // dummy socket. Whether this is successful or not we drop the result as
        // libstd will be sure to have initialized winsock.
        let _ = UdpSocket::bind("127.0.0.1:34254");
    });
}
