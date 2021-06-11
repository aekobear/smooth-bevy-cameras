use crate::{LookPolarity, OrbitTransform, OrbitTransformBundle, PolarDirection, Smoother};

use bevy::{
    app::prelude::*,
    ecs::{bundle::Bundle, prelude::*},
    input::{mouse::MouseMotion, prelude::*},
    math::prelude::*,
    render::prelude::*,
    transform::components::Transform,
};
use serde::{Deserialize, Serialize};

pub struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(default_input_map.system())
            .add_system(control_system.system())
            .add_event::<ControlEvent>();
    }
}

#[derive(Bundle)]
pub struct OrbitCameraBundle {
    controller: OrbitCameraController,
    #[bundle]
    orbit_transform: OrbitTransformBundle,
    #[bundle]
    perspective: PerspectiveCameraBundle,
}

impl OrbitCameraBundle {
    pub fn new(
        controller: OrbitCameraController,
        mut perspective: PerspectiveCameraBundle,
        eye: Vec3,
        target: Vec3,
    ) -> Self {
        // Make sure the transform is consistent with the controller to start.
        perspective.transform = Transform::from_translation(eye).looking_at(target, Vec3::Y);

        Self {
            controller,
            orbit_transform: OrbitTransformBundle {
                transform: OrbitTransform {
                    pivot: target,
                    orbit: eye,
                },
                polarity: LookPolarity::OrbitLookAtPivot,
                smoother: Smoother::new(controller.smoothing_weight),
            },
            perspective,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct OrbitCameraController {
    pub mouse_rotate_sensitivity: f32,
    pub mouse_translate_sensitivity: f32,
    pub smoothing_weight: f32,
    pub enabled: bool,
}

impl Default for OrbitCameraController {
    fn default() -> Self {
        Self {
            mouse_rotate_sensitivity: 0.002,
            mouse_translate_sensitivity: 0.1,
            smoothing_weight: 0.8,
            enabled: true,
        }
    }
}

pub enum ControlEvent {
    Rotate(Vec2),
    Translate(Vec2),
}

pub fn default_input_map(
    mut events: EventWriter<ControlEvent>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mouse_buttons: Res<Input<MouseButton>>,
    keyboard: Res<Input<KeyCode>>,
    controllers: Query<&OrbitCameraController>,
) {
    // Can only control one camera at a time.
    let controller = if let Some(controller) = controllers.iter().next() {
        controller
    } else {
        return;
    };
    let OrbitCameraController {
        enabled,
        mouse_rotate_sensitivity,
        mouse_translate_sensitivity,
        ..
    } = *controller;

    if !enabled {
        return;
    }

    let mut mouse_delta = Vec2::ZERO;
    for event in mouse_motion_events.iter() {
        mouse_delta += event.delta;
    }

    if mouse_buttons.pressed(MouseButton::Right) {
        if keyboard.pressed(KeyCode::LControl) {
            events.send(ControlEvent::Rotate(mouse_rotate_sensitivity * mouse_delta));
        } else {
            events.send(ControlEvent::Translate(
                mouse_translate_sensitivity * mouse_delta,
            ));
        }
    }
}

pub fn control_system(
    mut events: EventReader<ControlEvent>,
    mut cameras: Query<(&OrbitCameraController, &mut OrbitTransform, &Transform)>,
) {
    let (controller, mut transform, scene_transform) =
        if let Some((controller, transform, scene_transform)) = cameras.iter_mut().next() {
            (controller, transform, scene_transform)
        } else {
            return;
        };

    if controller.enabled {
        let mut polar_vector = PolarDirection::from_vector(transform.pivot_to_orbit_direction());

        for event in events.iter() {
            match event {
                ControlEvent::Rotate(delta) => {
                    polar_vector.add_yaw(-delta.x);
                    polar_vector.add_pitch(delta.y);
                    polar_vector.assert_not_looking_up();
                }
                ControlEvent::Translate(delta) => {
                    let right_dir = scene_transform.rotation * -Vec3::X;
                    let up_dir = scene_transform.rotation * Vec3::Y;
                    transform.pivot += delta.x * right_dir + delta.y * up_dir;
                }
            }
        }

        transform.set_orbit_in_direction(polar_vector.unit_vector());
    } else {
        events.iter(); // Drop the events.
    }
}