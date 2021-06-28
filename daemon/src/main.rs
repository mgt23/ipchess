use std::str::FromStr;

use clap::Clap;
use libp2p::futures::StreamExt;

mod api;
mod protocol;

#[derive(Clap)]
struct Opts {
    #[clap(long, default_value = "3030")]
    api_port: u16,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let opts = Opts::parse();

    let id_key_pair = libp2p::identity::Keypair::generate_ed25519();
    let local_peer_id = libp2p::PeerId::from(id_key_pair.public());

    log::info!("Local peer id {}", local_peer_id);

    let transport =
        libp2p::tokio_development_transport(id_key_pair).expect("failed creating transport");

    let behaviour = protocol::Ipchess::new();

    let mut swarm = libp2p::swarm::SwarmBuilder::new(transport, behaviour, local_peer_id.clone())
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();
    swarm
        .listen_on(libp2p::Multiaddr::from_str("/ip4/127.0.0.1/tcp/0").unwrap())
        .expect("swarm listen_on failed");

    let mut api_server = api::Server::new(opts.api_port)
        .await
        .expect("failed starting API server");

    log::info!("API listening at ws://{:?}", api_server.local_addr());

    let (signal_tx, mut signal_rx) = tokio::sync::mpsc::unbounded_channel();
    ctrlc::set_handler(move || {
        let _ = signal_tx.send(());
    })
    .expect("failed setting signal handler");

    loop {
        tokio::select! {
            swarm_event = swarm.select_next_some() => {
                match &swarm_event {
                    libp2p::swarm::SwarmEvent::NewListenAddr(addr) => {
                        log::info!("Swarm listening at {}", addr);
                    }

                    _ => {}
                }
            }

            api_event = api_server.next() => {
                match api_event {
                    Some(api::ServerEvent::NodeIdRequest(res_tx)) => {
                        let _ = res_tx.send(api::NodeIdResponse(*swarm.local_peer_id()));
                    }

                    None => {}
                }
            }

            _ = signal_rx.recv() => {
                break;
            }
        }
    }

    log::info!("shutting down...");
}
