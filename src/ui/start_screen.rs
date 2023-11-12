use bevy::sprite::Mesh2dHandle;

use super::manual_ui::*;
use super::path_tracing::*;
use super::ui_cameras::CorrectCamera;
use crate::prelude::*;

/// Plugin
pub struct StartScreen;

/// Which CAM this entity belongs to, for start screen only
#[derive(Component, Deref)]
struct Cam(UiCameras);

impl Plugin for StartScreen {
	fn build(&self, app: &mut App) {
		app
			.add_state::<StartScreenStates>()
			.add_systems(OnEnter(StartScreenStates::Initial), Self::spawn_initial)
			.add_systems(
				OnEnter(StartScreenStates::ConfigureHost),
				Self::spawn_configure_host,
			)
			.add_systems(
				Update,
				(
					ButtonParticle::follow_parent_bbox,
					StartScreen::handle_hover_interactions,
					StartScreen::handle_click_interactions,
				),
			);
	}
}

#[derive(States, Component, Debug, Default, PartialEq, Eq, Hash, Clone, Copy)]
enum StartScreenStates {
	#[default]
	Initial,

	ConfigureHost,

	ConfigureClient,
	// ConfigureSolo,
}

/// List of buttons that can be clicked
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
enum InitialUiButtons {
	InitialHostGame,
	InitialJoinGame,
	// InitialSolo,
}

impl InitialUiButtons {
	const fn get_text(self) -> &'static str {
		match self {
			InitialUiButtons::InitialHostGame => "Host Game",
			InitialUiButtons::InitialJoinGame => "Join Game",
		}
	}
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
enum HostGameButtons {
	HostPublicGame,
	HostMachineLocalGame,
}

impl HostGameButtons {
	const fn get_text(self) -> &'static str {
		match self {
			HostGameButtons::HostPublicGame => "Host Public Game",
			HostGameButtons::HostMachineLocalGame => "Host Machine-Local Game",
		}
	}
}

impl StartScreen {
	fn spawn_initial(
		mut commands: Commands,
		mut mma: MM2,
		ass: Res<AssetServer>,
		mut effects: ResMut<Assets<EffectAsset>>,
	) {
		const INITIAL_CAM: UiCameras = UiCameras::MiddleLeft;

		let mut column = ManualColumn {
			const_x: 200.,
			const_width: 200.,
			current_y: 0.,
			item_height: 50.,
			margin: 10.,
		}
		.center_with(2);

		for btn in InitialUiButtons::iter() {
			commands
				.spawn(GameButtonBundle::new(
					INITIAL_CAM,
					btn,
					column.next(),
					&mut mma,
				))
				.with_children(|parent| {
					parent.spawn(ButtonParticles::new(INITIAL_CAM, &mut effects));
					parent.spawn(ButtonText::new(INITIAL_CAM, btn.get_text(), &ass));
				});
		}

		// commands
		// 	.spawn(GameButtonBundle::new(column.next(), &mut mma))
		// 	.with_children(|parent| {
		// 		parent.spawn(ButtonParticles::new(&mut effects));
		// 		parent.spawn(ButtonText::new("Solo", &ass));
		// 	});
	}

	fn spawn_configure_host(
		mut commands: Commands,
		mut mma: MM2,
		ass: Res<AssetServer>,
		mut effects: ResMut<Assets<EffectAsset>>,
	) {
		const CONFIG_HOST: UiCameras = UiCameras::MiddleRight;

		let mut column = ManualColumn {
			const_x: -200.,
			const_width: 200.,
			current_y: 0.,
			item_height: 50.,
			margin: 10.,
		}
		.center_with(2);

		for btn in HostGameButtons::iter() {
			commands
				.spawn(GameButtonBundle::new(
					CONFIG_HOST,
					btn,
					column.next(),
					&mut mma,
				))
				.with_children(|parent| {
					parent.spawn(ButtonParticles::new(CONFIG_HOST, &mut effects));
					parent.spawn(ButtonText::new(CONFIG_HOST, btn.get_text(), &ass));
				});
		}
	}

	fn handle_hover_interactions(
		mut start_hover_events: EventReader<Pointer<Move>>,
		mut end_hover_events: EventReader<Pointer<Out>>,
		this: Query<(&Cam, &Children)>,
		mut particle_spawners: Query<&mut EffectSpawner>,
		correct_camera: CorrectCamera,
	) {
		for start_event in start_hover_events.read() {
			if let Ok((cam, this)) = this.get(start_event.target) {
				// found callback target
				let camera = start_event.event.hit.camera;
				if correct_camera.confirm(&camera, **cam) {
					// correct camera

					if let Some(particle_spawner_entity) = this
						.iter()
						.find(|child| particle_spawners.get(**child).is_ok())
					{
						// found particle spawner
						let mut spawner = particle_spawners.get_mut(*particle_spawner_entity).unwrap();

						spawner.set_active(true);
					} else {
						warn!("Cannot find particle spawner");
					}
				}
			} else {
				warn!("Cannot find target callback");
			}
		}

		for end_event in end_hover_events.read() {
			if let Ok((cam, this)) = this.get(end_event.target) {
				// found callback target
				let camera = end_event.event.hit.camera;
				if correct_camera.confirm(&camera, **cam) {
					// correct camera

					if let Some(particle_spawner_entity) = this
						.iter()
						.find(|child| particle_spawners.get(**child).is_ok())
					{
						// found particle spawner
						let mut spawner = particle_spawners.get_mut(*particle_spawner_entity).unwrap();

						spawner.set_active(false);
					} else {
						warn!("Cannot find particle spawner");
					}
				}
			} else {
				warn!("Cannot find target callback");
			}
		}
	}

	fn handle_click_interactions(mut click_events: EventReader<Pointer<Click>>, this: Query<&Cam>) {}
}

#[derive(Bundle)]
struct GameButtonBundle<T: Component + Send + Sync + 'static> {
	mesh: Mesh2dHandle,
	material: Handle<ColorMaterial>,
	spatial: SpatialBundle,
	path: BevyPath,

	btn: T,

	cam: Cam,
	name: Name,
	layer: RenderLayers,
}

impl<T: Component + Send + Sync + 'static> GameButtonBundle<T> {
	fn new(cam: UiCameras, btn: T, manual_node: ManualNode, mma: &mut MM2) -> Self {
		Self {
			cam: Cam(cam),
			btn,
			mesh: mma
				.meshs
				.add(
					shape::Quad::new(Vec2::new(
						manual_node.bbox.half_width * 2.,
						manual_node.bbox.half_height * 2.,
					))
					.into(),
				)
				.into(),
			material: mma.mats.add(Color::WHITE.into()),
			spatial: SpatialBundle::from_transform(Transform::from_xyz(
				manual_node.position.x,
				manual_node.position.y,
				1.,
			)),
			name: Name::new("Host Game Button"),
			path: BevyPath::rectangle_from_bbox(manual_node.bbox),
			layer: GlobalRenderLayers::Ui(cam).into(),
		}
	}
}

#[derive(Bundle)]
struct ButtonText {
	text_bundle: Text2dBundle,

	name: Name,
	render_layer: RenderLayers,
}

impl ButtonText {
	fn new(cam: UiCameras, text: impl Into<Cow<'static, str>>, ass: &AssetServer) -> Self {
		let style = TextStyle {
			font: ass.load(GlobalFont::Default),
			font_size: 40.,
			color: Color::MIDNIGHT_BLUE,
		};

		ButtonText {
			text_bundle: Text2dBundle {
				text: Text::from_section(text.into(), style.clone()).with_alignment(TextAlignment::Center),
				transform: Transform::from_translation(Vec3::Z),
				..default()
			},
			name: Name::new("Button Text"),
			render_layer: GlobalRenderLayers::Ui(cam).into(),
		}
	}
}

#[derive(Component)]
struct ButtonParticle;

#[derive(Bundle)]
struct ButtonParticles {
	particles: ParticleEffectBundle,
	marker: ButtonParticle,

	layer: RenderLayers,
	name: Name,
}

impl ButtonParticles {
	fn new(cam: UiCameras, mut effects: &mut Assets<EffectAsset>) -> Self {
		let mut gradient = Gradient::new();
		// gradient.add_key(0.0, Vec4::new(0.5, 0.5, 0.5, 1.0));
		// gradient.add_key(0.1, Vec4::new(0.5, 0.5, 0.0, 1.0));
		// gradient.add_key(0.4, Vec4::new(0.5, 0.0, 0.0, 1.0));
		// gradient.add_key(1.0, Vec4::splat(0.0));
		gradient.add_key(0.0, Vec4::splat(1.));
		gradient.add_key(1.0, Vec4::new(0., 0., 0., 1.));

		let writer = ExprWriter::new();

		let age = writer.lit(0.).uniform(writer.lit(0.3)).expr();
		let init_age = SetAttributeModifier::new(Attribute::AGE, age);

		let lifetime = writer.lit(1.).uniform(writer.lit(1.5)).expr();
		let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

		let init_pos = SetPositionSphereModifier {
			center: writer.lit(Vec3::ZERO).expr(),
			radius: writer.lit(25.).expr(),
			dimension: ShapeDimension::Volume,
		};

		let init_vel = SetVelocitySphereModifier {
			center: writer.lit(Vec3::ZERO).expr(),
			speed: writer.lit(15.).expr(),
		};

		let effect = effects.add(
			EffectAsset::new(
				32768,
				Spawner::rate(1000.0.into()).with_starts_active(false),
				writer.finish(),
			)
			.with_name("gradient")
			.init(init_pos)
			.init(init_vel)
			.init(init_age)
			.init(init_lifetime)
			.render(ColorOverLifetimeModifier { gradient }),
		);

		Self {
			particles: ParticleEffectBundle::new(effect),
			marker: ButtonParticle,
			layer: GlobalRenderLayers::Ui(cam).into(),
			name: Name::new("Button Particles"),
		}
	}
}

impl ButtonParticle {
	fn follow_parent_bbox(
		mut spawner: Query<(&Parent, &mut Transform), With<Self>>,
		parents: Query<&BevyPath>,
		time: Res<Time>,
	) {
		for (parent, mut transform) in spawner.iter_mut() {
			if let Ok(path) = parents.get(parent.get()) {
				const FACTOR: f32 = 250.;
				let time = time.elapsed().as_millis() as f32 % FACTOR / FACTOR;
				let pos = path.get_pos_at_time(time);
				transform.translation.x = pos.x;
				transform.translation.y = pos.y;
			} else {
				error!("Particle spawner's parent does not have a BBox component");
			}
		}
	}
}
