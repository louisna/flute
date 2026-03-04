use flute::{
    core::UDPEndpoint,
    receiver::{writer, MultiReceiver},
};
use std::rc::Rc;

mod msocket;

fn main() {
    env_logger::builder().try_init().ok();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        println!("Save FLUTE objects to a destination folder received from UDP/FLUTE");
        println!("Usage: {} <dest_dir> <multicast_addr:port> <bind_addr>", args[0]);
        println!("  e.g. {} ./output 224.0.0.1:3400 127.0.0.1", args[0]);
        std::process::exit(0);
    }

    let dest_dir = std::path::Path::new(&args[1]);
    if !dest_dir.is_dir() {
        log::error!("{:?} is not a directory", dest_dir);
        std::process::exit(-1);
    }

    let multicast_addr: std::net::SocketAddr = args[2].parse().unwrap_or_else(|_| {
        eprintln!("Invalid multicast address: {}", args[2]);
        std::process::exit(-1);
    });
    let bind_addr = &args[3];

    let group_addr = multicast_addr.ip().to_string();
    let port = multicast_addr.port();
    let endpoint = UDPEndpoint::new(None, group_addr, port);

    log::info!("Create FLUTE, write objects to {:?}", dest_dir);

    let mut config = flute::receiver::Config::default();
    config.object_receive_once = true;

    let socket = msocket::MSocket::new(&endpoint, Some(bind_addr.as_str()), false)
        .expect("Fail to create Multicast Socket");

    // Writer is constructed after the socket so its internal Instant marks socket-ready time.
    let enable_md5_check = true;
    let writer = Rc::new(writer::ObjectWriterFSBuilder::new(dest_dir, enable_md5_check).unwrap());
    let mut receiver = MultiReceiver::new(writer, Some(config), false);

    let mut buf = [0; 2048];
    loop {
        let (n, _src) = socket
            .sock
            .recv_from(&mut buf)
            .expect("Failed to receive data");

        let now = std::time::SystemTime::now();
        match receiver.push(&endpoint, &buf[..n], now) {
            Err(_) => log::error!("Wrong ALC/LCT packet"),
            _ => {}
        };
        receiver.cleanup(now);
    }
}
