use std::{
    future::Future,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use tokio::runtime::Runtime;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

pub trait WindowApp {
    fn new(window: Arc<Window>) -> impl Future<Output = Self>;
    fn set_window_resized(&mut self, new_size: PhysicalSize<u32>);
    fn resize_surface_if_needed(&mut self);
    fn keyboard_input(&mut self, event: &KeyEvent) -> bool;
    fn render(&mut self) -> Result<(), wgpu::SurfaceError>;
    fn update(&mut self);
}

#[derive(Default)]
pub struct WindowAppHandler<A: WindowApp> {
    app: Arc<Mutex<Option<A>>>,
    window: Option<Arc<Window>>,
    title: String,
}

impl<A: WindowApp> WindowAppHandler<A> {
    pub fn new(title: &str) -> Self {
        Self {
            app: Arc::new(Mutex::new(None)),
            window: None,
            title: title.to_string(),
        }
    }

    pub fn pre_present_notify(&self) {
        if let Some(window) = self.window.as_ref() {
            window.pre_present_notify();
        }
    }

    pub fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

impl<A: WindowApp> ApplicationHandler for WindowAppHandler<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.app.lock().unwrap().deref().is_some() {
            return;
        }

        let rt = Runtime::new().unwrap();

        let window_attributes = Window::default_attributes().with_title(&self.title);
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let wgpu_app = rt.block_on(A::new(window.clone()));

        self.app.lock().unwrap().deref_mut().replace(wgpu_app);
        self.window.replace(window);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let mut guard = self.app.lock().unwrap();
        let app = guard.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if physical_size.width == 0 || physical_size.height == 0 {
                    // 处理最小化窗口的事件
                } else {
                    app.set_window_resized(physical_size);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let _ = app.keyboard_input(&event);
            }
            WindowEvent::RedrawRequested => {
                app.update();

                self.pre_present_notify();

                match app.render() {
                    Ok(_) => {}
                    // 当展示平面的上下文丢失，就需重新配置
                    Err(wgpu::SurfaceError::Lost) => eprintln!("Surface is lost"),
                    // 所有其他错误（过期、超时等）应在下一帧解决
                    Err(e) => eprintln!("{e:?}"),
                }

                // 除非我们手动请求，RedrawRequested 将只会触发一次。
                self.request_redraw();
            }
            _ => (),
        }
    }
}
