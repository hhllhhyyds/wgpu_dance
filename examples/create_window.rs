use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use tokio::runtime::Runtime;

struct WgpuApp {
    #[allow(unused)]
    window: Arc<Window>,
}

impl WgpuApp {
    async fn new(window: Arc<Window>) -> Self {
        Self { window }
    }
}

#[derive(Default)]
struct WgpuAppHandler {
    app: Arc<Mutex<Option<WgpuApp>>>,
}

impl ApplicationHandler for WgpuAppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.app.lock().unwrap().deref().is_some() {
            return;
        }

        let rt = Runtime::new().unwrap();

        let window_attributes = Window::default_attributes().with_title("create window");
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let wgpu_app = rt.block_on(WgpuApp::new(window));

        self.app.lock().unwrap().deref_mut().replace(wgpu_app);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // 暂停事件
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // 窗口事件
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(_size) => {
                // 窗口大小改变
            }
            WindowEvent::KeyboardInput { .. } => {
                // 键盘事件
            }
            WindowEvent::RedrawRequested => {
                // surface重绘事件
            }
            _ => (),
        }
    }
}

fn main() -> Result<(), impl std::error::Error> {
    let events_loop = EventLoop::new().unwrap();
    let mut app = WgpuAppHandler::default();
    events_loop.run_app(&mut app)
}
