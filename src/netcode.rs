use crate::{players::PlayerBlueprint, prelude::*};

pub use bevy_replicon::{
	prelude::*,
	renet::{
		transport::{
			ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport, ServerAuthentication,
			ServerConfig,
		},
		ConnectionConfig, ServerEvent,
	},
};

pub struct NetcodePlugin;

impl Plugin for NetcodePlugin {
	fn build(&self, app: &mut App) {
		app
			.add_systems(OnEnter(GlobalGameStates::InGame), Self::add_netcode)
			.add_systems(OnExit(GlobalGameStates::InGame), Self::disconnect_netcode)
			.add_systems(Update, Self::server_event_system.run_if(has_authority()))
			.add_systems(FixedUpdate, Self::frame_inc_and_replicon_tick_sync);
	}
}

impl NetcodePlugin {
	fn frame_inc_and_replicon_tick_sync(
		mut game_clock: ResMut<GameClock>, // your own tick counter
		mut replicon_tick: ResMut<RepliconTick>,
	) {
		// advance your tick and replicon's tick in lockstep
		game_clock.advance(1);
		let delta = game_clock.frame().saturating_sub(replicon_tick.get());
		replicon_tick.increment_by(delta);
	}
}

#[derive(SystemParam)]
pub struct ClientID<'w> {
	res: Option<Res<'w, NetcodeClientTransport>>,
}

impl ClientID<'_> {
	pub fn id(&self) -> ClientId {
		self
			.res
			.as_ref()
			.map(|client| client.client_id())
			.unwrap_or(SERVER_ID)
	}
}

/// Holds information about what ip and port to connect to, or host on.
#[derive(Resource, Debug, clap::Parser)]
pub enum NetcodeConfig {
	Server {
		#[arg(short, long, default_value_t = Ipv4Addr::LOCALHOST.into())]
		ip: IpAddr,

		#[arg(short, long, default_value_t = DEFAULT_PORT)]
		port: u16,
	},
	Client {
		#[arg(short, long, default_value_t = Ipv4Addr::LOCALHOST.into())]
		ip: IpAddr,

		#[arg(short, long, default_value_t = DEFAULT_PORT)]
		port: u16,
	},
}

impl NetcodeConfig {
	pub const fn new_hosting_public() -> Self {
		// TODO: Verify this actually hosts, don't we need 0.0.0.0?
		NetcodeConfig::Server {
			ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
			port: DEFAULT_PORT,
		}
	}

	pub const fn new_hosting_machine_local() -> Self {
		NetcodeConfig::Server {
			ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
			port: DEFAULT_PORT,
		}
	}

	pub const fn new_client_machine_local() -> Self {
		NetcodeConfig::Client {
			ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
			port: DEFAULT_PORT,
		}
	}
}

impl NetcodePlugin {
	/// sets up the server / client depending on [NetcodeConfig]
	fn add_netcode(
		mut commands: Commands,
		network_channels: Res<NetworkChannels>,
		config: Res<NetcodeConfig>,
		_manual_server_feedback: EventWriter<ServerEvent>,
	) {
		match config.into_inner() {
			NetcodeConfig::Server { ip: addr, port } => {
				info!("Setting up as server, hosting on {}:{}", addr, port);
				let server_channels_config = network_channels.get_server_configs();
				let client_channels_config = network_channels.get_client_configs();

				let server = RenetServer::new(ConnectionConfig {
					server_channels_config,
					client_channels_config,
					..Default::default()
				});

				let current_time = SystemTime::now()
					.duration_since(SystemTime::UNIX_EPOCH)
					.unwrap();
				let public_addr = SocketAddr::new(*addr, *port);

				let socket = UdpSocket::bind(public_addr).expect("Couldn't bind to UdpSocket");

				let server_config = ServerConfig {
					current_time,
					max_clients: 10,
					protocol_id: PROTOCOL_ID,
					public_addresses: vec![public_addr],
					authentication: ServerAuthentication::Unsecure,
				};
				let transport = NetcodeServerTransport::new(server_config, socket).unwrap();

				commands.insert_resource(server);
				commands.insert_resource(transport);

				commands.spawn(PlayerBlueprint::default_at(SERVER_ID, Transform::default()));
			}
			NetcodeConfig::Client { ip, port } => {
				info!(
					"Setting up as client, connecting to {:?} on port {}",
					ip, port
				);
				let server_channels_config = network_channels.get_server_configs();
				let client_channels_config = network_channels.get_client_configs();

				let client = RenetClient::new(ConnectionConfig {
					server_channels_config,
					client_channels_config,
					..Default::default()
				});

				let current_time = SystemTime::now()
					.duration_since(SystemTime::UNIX_EPOCH)
					.unwrap();
				let client_id = current_time.as_millis() as u64;
				let server_addr = SocketAddr::new(*ip, *port);
				let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
					.expect("Couldn't bind to (unspecified) socket");
				let authentication = ClientAuthentication::Unsecure {
					client_id,
					protocol_id: PROTOCOL_ID,
					server_addr,
					user_data: None,
				};
				let transport = NetcodeClientTransport::new(current_time, authentication, socket)
					.expect("Couldn't join to server");

				commands.insert_resource(client);
				commands.insert_resource(transport);
			}
		}
	}

	fn disconnect_netcode(
		config: Res<NetcodeConfig>,
		mut client: ResMut<RenetClient>,
		mut server: ResMut<RenetServer>,
		mut commands: Commands,
	) {
		match config.into_inner() {
			NetcodeConfig::Server { .. } => {
				info!("Disconnecting as server");
				server.disconnect_all();
				commands.remove_resource::<RenetServer>();
				commands.remove_resource::<NetcodeClientTransport>();
			}
			NetcodeConfig::Client { .. } => {
				info!("Disconnecting client");
				client.disconnect();
				commands.remove_resource::<RenetClient>();
				commands.remove_resource::<NetcodeClientTransport>();
			}
		}
	}

	/// Logs server events and spawns a new player whenever a client connects.
	fn server_event_system(mut server_event: EventReader<ServerEvent>, mut commands: Commands) {
		for event in server_event.read() {
			match event {
				ServerEvent::ClientConnected { client_id } => {
					info!("New player with id {client_id} connected");

					commands.spawn(PlayerBlueprint::default_at(
						*client_id,
						Transform::default(),
					));
				}
				ServerEvent::ClientDisconnected { client_id, reason } => {
					info!("Client {client_id} disconnected because: {reason}");
				}
			}
		}
	}
}
