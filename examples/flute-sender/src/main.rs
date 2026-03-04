use flute::{
    core::lct::Cenc,
    core::UDPEndpoint,
    sender::{CarouselRepeatMode, ObjectDesc, Sender},
};
use std::{net::UdpSocket, time::SystemTime};

fn main() {
    std::env::set_var("RUST_LOG", "info");
    env_logger::builder().try_init().ok();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!("Send a list of files over UDP/FLUTE");
        println!("Usage: {} <dest_addr:port> path/to/file1 ...", args[0]);
        println!("  e.g. {} 224.0.0.1:3400 myfile.bin", args[0]);
        std::process::exit(0);
    }

    let dest = &args[1];
    let dest_addr: std::net::SocketAddr = dest.parse().unwrap_or_else(|_| {
        eprintln!("Invalid destination address: {}", dest);
        std::process::exit(-1);
    });
    let group_addr = dest_addr.ip().to_string();
    let port = dest_addr.port();

    let endpoint = UDPEndpoint::new(None, group_addr, port);

    log::info!("Create UDP Socket");
    let udp_socket = UdpSocket::bind("0.0.0.0:0").unwrap();

    log::info!("Create FLUTE Sender");
    let tsi = 1;
    let mut sender = Sender::new(endpoint, tsi, &Default::default(), &Default::default());

    for file in &args[2..] {
        let path = std::path::Path::new(file);
        if !path.is_file() {
            log::error!("{} is not a file", file);
            std::process::exit(-1);
        }

        log::info!("Insert file {} to FLUTE sender", file);
        let carousel = Some(CarouselRepeatMode::DelayBetweenTransfers(
            std::time::Duration::from_secs(0),
        ));
        let obj = ObjectDesc::create_from_file(
            path,
            None,
            "application/octet-stream",
            true,
            1,
            carousel,
            None,
            None,
            None,
            Cenc::Null,
            true,
            None,
            true,
        )
        .unwrap();
        sender.add_object(0, obj).expect("Add object failed");
    }

    log::info!("Publish FDT update");
    sender.publish(SystemTime::now()).expect("Publish failed");

    // Send a "close session" packet to notify receivers that a new session is starting.
    let close_session_pkt = sender.read_close_session(SystemTime::now());
    udp_socket.send_to(&close_session_pkt, dest).expect("Send failed");

    loop {
        if let Some(pkt) = sender.read(SystemTime::now()) {
            udp_socket.send_to(&pkt, dest).expect("Send failed");
        } else {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
