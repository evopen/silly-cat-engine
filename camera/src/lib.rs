use bytemuck::{Pod, Zeroable};
use glam::Vec3A as Vec3;

#[derive(Debug, Default)]
pub struct Camera {
    position: Vec3,
    front: Vec3,
    yaw: f32,
    pitch: f32,
    world_up: Vec3,
    right: Vec3,
    up: Vec3,
    right_button_pressed: bool,
    camera_uniform: CameraUniform,
    key_pressed: KeyPressed,
}

#[derive(Debug, Default)]

struct KeyPressed {
    w: bool,
    s: bool,
    a: bool,
    d: bool,
    q: bool,
    e: bool,
}

enum Direction {
    Forward,
    Backward,
    Left,
    Right,
    Up,
    Down,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, Pod, Zeroable)]
pub struct CameraUniform {
    pub origin: glam::Vec3,
}

impl Camera {
    pub fn new(position: Vec3, look_at: Vec3) -> Self {
        let front: Vec3 = look_at - position;
        let pitch = (front.y / front.length())
            .asin()
            .to_degrees()
            .clamp(-89.0, 89.0);
        let mut yaw = (front.z / front.length()).asin().to_degrees();

        if front.z >= 0.0 && front.x < 0.0 {
            yaw = 180.0 - yaw;
        }

        let mut camera = Self {
            position,
            front,
            yaw,
            pitch,
            world_up: Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        };

        camera.update_vectors();

        camera
    }

    pub fn input(&mut self, event: &winit::event::Event<()>) {
        match event {
            winit::event::Event::NewEvents(_) => {}
            winit::event::Event::WindowEvent { window_id, event } => {
                match event {
                    winit::event::WindowEvent::Resized(_) => {}
                    winit::event::WindowEvent::Moved(_) => {}
                    winit::event::WindowEvent::ReceivedCharacter(_) => {}
                    winit::event::WindowEvent::Focused(_) => {}
                    winit::event::WindowEvent::KeyboardInput {
                        device_id,
                        input,
                        is_synthetic,
                    } => {}
                    winit::event::WindowEvent::ModifiersChanged(_) => {}
                    winit::event::WindowEvent::CursorMoved {
                        device_id,
                        position,
                        ..
                    } => {}
                    winit::event::WindowEvent::CursorEntered { device_id } => {}
                    winit::event::WindowEvent::CursorLeft { device_id } => {}
                    winit::event::WindowEvent::MouseWheel {
                        device_id,
                        delta,
                        phase,
                        ..
                    } => {}
                    winit::event::WindowEvent::MouseInput {
                        device_id,
                        state,
                        button,
                        ..
                    } => {
                        match button {
                            winit::event::MouseButton::Left => {}
                            winit::event::MouseButton::Right => {
                                match state {
                                    winit::event::ElementState::Pressed => {
                                        self.right_button_pressed = true;
                                    }
                                    winit::event::ElementState::Released => {
                                        self.right_button_pressed = false;
                                    }
                                }
                            }
                            winit::event::MouseButton::Middle => {}
                            winit::event::MouseButton::Other(_) => {}
                        }
                    }
                    winit::event::WindowEvent::AxisMotion {
                        device_id,
                        axis,
                        value,
                    } => {}
                    winit::event::WindowEvent::ScaleFactorChanged {
                        scale_factor,
                        new_inner_size,
                    } => {}
                    winit::event::WindowEvent::ThemeChanged(_) => {}
                    _ => {}
                }
            }
            winit::event::Event::DeviceEvent { device_id, event } => {
                match event {
                    winit::event::DeviceEvent::Added => {}
                    winit::event::DeviceEvent::Removed => {}
                    winit::event::DeviceEvent::MouseMotion { delta: (x, y) } => {
                        if self.right_button_pressed {
                            self.process_mouse_movement((x * 0.08) as f32, (y * 0.08) as f32);
                        }
                    }
                    winit::event::DeviceEvent::MouseWheel { delta } => {}
                    winit::event::DeviceEvent::Motion { axis, value } => {}
                    winit::event::DeviceEvent::Button { button, state } => {}
                    winit::event::DeviceEvent::Key(input) => {
                        if let Some(keycode) = input.virtual_keycode {
                            match keycode {
                                winit::event::VirtualKeyCode::W => {
                                    self.key_pressed.w = match input.state {
                                        winit::event::ElementState::Pressed => true,
                                        winit::event::ElementState::Released => false,
                                    }
                                }
                                winit::event::VirtualKeyCode::S => {
                                    self.key_pressed.s = match input.state {
                                        winit::event::ElementState::Pressed => true,
                                        winit::event::ElementState::Released => false,
                                    }
                                }
                                winit::event::VirtualKeyCode::A => {
                                    self.key_pressed.a = match input.state {
                                        winit::event::ElementState::Pressed => true,
                                        winit::event::ElementState::Released => false,
                                    }
                                }
                                winit::event::VirtualKeyCode::D => {
                                    self.key_pressed.d = match input.state {
                                        winit::event::ElementState::Pressed => true,
                                        winit::event::ElementState::Released => false,
                                    }
                                }
                                winit::event::VirtualKeyCode::Q => {
                                    self.key_pressed.q = match input.state {
                                        winit::event::ElementState::Pressed => true,
                                        winit::event::ElementState::Released => false,
                                    }
                                }
                                winit::event::VirtualKeyCode::E => {
                                    self.key_pressed.e = match input.state {
                                        winit::event::ElementState::Pressed => true,
                                        winit::event::ElementState::Released => false,
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    winit::event::DeviceEvent::Text { codepoint } => {}
                }
            }
            winit::event::Event::UserEvent(_) => {}
            winit::event::Event::MainEventsCleared => {}
            winit::event::Event::RedrawRequested(_) => {}
            winit::event::Event::RedrawEventsCleared => {}
            winit::event::Event::LoopDestroyed => {}
            _ => {}
        }
        self.update();
    }

    fn update(&mut self) {
        let speed = 0.01;
        if self.key_pressed.w {
            self.process_keyboard(Direction::Forward, speed);
        }
        if self.key_pressed.s {
            self.process_keyboard(Direction::Backward, speed);
        }
        if self.key_pressed.a {
            self.process_keyboard(Direction::Left, speed);
        }
        if self.key_pressed.d {
            self.process_keyboard(Direction::Right, speed);
        }
        if self.key_pressed.q {
            self.process_keyboard(Direction::Down, speed);
        }
        if self.key_pressed.e {
            self.process_keyboard(Direction::Up, speed);
        }
    }

    fn process_mouse_movement(&mut self, yaw_offset: f32, pitch_offset: f32) {
        self.yaw += yaw_offset;
        self.pitch = (self.pitch + pitch_offset).clamp(-89.0, 89.0);
        self.update_vectors();
    }

    fn process_keyboard(&mut self, direction: Direction, distance: f32) {
        match direction {
            Direction::Forward => {
                self.position += self.front * distance;
            }
            Direction::Backward => {
                self.position -= self.front * distance;
            }
            Direction::Left => {
                self.position -= self.right * distance;
            }
            Direction::Right => {
                self.position += self.right * distance;
            }
            Direction::Up => {
                self.position += self.world_up * distance;
            }
            Direction::Down => {
                self.position -= self.world_up * distance;
            }
        }
    }

    pub fn camera_uniform(&self) -> CameraUniform {
        CameraUniform {
            origin: self.position.into(),
        }
    }

    fn update_vectors(&mut self) {
        self.front = Vec3::new(
            self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.yaw.to_radians().sin() * self.pitch.to_radians().cos(),
        )
        .normalize();
        self.right = self.front.cross(self.world_up).normalize();
        self.up = self.right.cross(self.front).normalize();
    }

    pub fn position(&self) -> glam::Vec3A {
        self.position
    }
}
