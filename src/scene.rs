use crate::prelude::*;
use bevy::utils::tracing;
use godot::engine::{
    node::InternalMode, packed_scene::GenEditState, resource_loader::CacheMode, ResourceLoader,
};

pub(crate) struct PackedScenePlugin;
impl Plugin for PackedScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spawn_scene.in_base_set(CoreSet::PostUpdate));
    }
}

/// A to-be-instanced-and-spawned Godot scene.
///
/// [`GodotScene`]s that are spawned/inserted into the bevy world will be instanced from the provided
/// handle/path and the instance will be added as an [`ErasedGd`] in the next PostUpdateFlush set.
/// (see [`spawn_scene`])
#[derive(Debug, Component)]
pub struct GodotScene {
    resource: GodotSceneResource,
    transform: Option<GodotSceneTransform>,
}

#[derive(Debug)]
enum GodotSceneResource {
    Path(String),
    Handle(Handle<ErasedGdResource>),
    Resource(ErasedGdResource),
}

#[derive(Debug)]
enum GodotSceneTransform {
    Transform2D(Transform2D),
    Transform3D(Transform3D),
}

impl GodotScene {
    /// Instantiate the godot scene from the given path.
    ///
    /// Note that this will call [`ResourceLoader`].load() - which is a blocking load.
    /// If you want "preload" functionality, you should load your resources in a dedicated Bevy
    /// state into a Bevy [`Resource`], and use from_handle or from_resource.
    pub fn from_path(path: &str) -> Self {
        Self {
            resource: GodotSceneResource::Path(path.to_string()),
            transform: None,
        }
    }

    /// Instantiate the godot scene from a Bevy Asset [`Handle`].
    pub fn from_handle(handle: &Handle<ErasedGdResource>) -> Self {
        Self {
            resource: GodotSceneResource::Handle(handle.clone()),
            transform: None,
        }
    }

    /// Instantiate the godot scene from an ErasedGdResource.
    pub fn from_resource(res: ErasedGdResource) -> Self {
        Self {
            resource: GodotSceneResource::Resource(res),
            transform: None,
        }
    }

    pub fn with_transform3d(mut self, transform: Transform3D) -> Self {
        self.transform = Some(GodotSceneTransform::Transform3D(transform));
        self
    }

    pub fn with_transform2d(mut self, transform: Transform2D) -> Self {
        self.transform = Some(GodotSceneTransform::Transform2D(transform));
        self
    }

    pub fn with_translation3d(mut self, translation: Vector3) -> Self {
        self.transform = Some(GodotSceneTransform::Transform3D(
            Transform3D::default().translated(translation),
        ));
        self
    }

    pub fn with_translation2d(mut self, translation: Vector2) -> Self {
        self.transform = Some(GodotSceneTransform::Transform2D(
            Transform2D::IDENTITY.translated(translation),
        ));
        self
    }
}

#[derive(Component, Debug, Default)]
struct GodotSceneSpawned;

fn spawn_scene(
    mut commands: Commands,
    mut new_scenes: Query<(&mut GodotScene, Entity), Without<GodotSceneSpawned>>,
    mut assets: ResMut<Assets<ErasedGdResource>>,
    mut scene_tree: SceneTreeRef,
) {
    for (mut scene, ent) in new_scenes.iter_mut() {
        let packed_scene = match &mut scene.resource {
            GodotSceneResource::Handle(handle) => assets
                .get_mut(&handle)
                .expect("packed scene to exist in assets")
                .get(),
            GodotSceneResource::Path(path) => ResourceLoader::singleton()
                .load(
                    path.into(),
                    "PackedScene".into(),
                    CacheMode::CACHE_MODE_REUSE,
                )
                .expect("packed scene to load"),
            GodotSceneResource::Resource(res) => res.get(),
        };

        let instance = packed_scene
            .try_cast::<PackedScene>()
            .expect("resource to be a packed scene")
            .instantiate(GenEditState::GEN_EDIT_STATE_DISABLED)
            .unwrap();

        scene_tree.get().get_current_scene().unwrap().add_child(
            instance.share(),
            false,
            InternalMode::INTERNAL_MODE_DISABLED,
        );

        if let Some(transform) = &scene.transform {
            match transform {
                GodotSceneTransform::Transform2D(transform) => {
                    match instance.share().try_cast::<Node2D>() {
                        Some(mut node2d) => node2d.set_global_transform(*transform),
                        None => tracing::error!("attempted to spawn a scene with a transform on Node that did not inherit from Node3D, the transform was not set"),
                    }
                }
                GodotSceneTransform::Transform3D(transform) => {
                    match instance.share().try_cast::<Node3D>() {
                    Some(mut node3d) => node3d.set_global_transform(*transform),
                    None => tracing::error!("attempted to spawn a scene with a transform on Node that did not inherit from Node3D, the transform was not set"),
                }
                }
            }
        }

        commands
            .entity(ent)
            .insert(ErasedGd::new(instance))
            .insert(GodotSceneSpawned);
    }
}
