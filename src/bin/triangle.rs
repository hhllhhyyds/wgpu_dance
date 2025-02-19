use std::{rc::Rc, sync::Arc};

use parking_lot::Mutex;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

struct WgpuApp {
    /// 避免窗口被释放
    #[allow(unused)]
    window: Arc<Window>,
}

impl WgpuApp {
    async fn new(window: Arc<Window>) -> Self {
        // ...
        Self { window }
    }
}

#[derive(Default)]
struct WgpuAppHandler {
    app: Rc<Mutex<Option<WgpuApp>>>,
}

impl ApplicationHandler for WgpuAppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // 恢复事件
        if self.app.as_ref().lock().is_some() {
            return;
        }

        let window_attributes = Window::default_attributes().with_title("tutorial1-window");
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let wgpu_app = pollster::block_on(WgpuApp::new(window));
        self.app.lock().replace(wgpu_app);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // 暂停事件
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        window_event: WindowEvent,
    ) {
        // 窗口事件
        match window_event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(_size) => {
                // 窗口大小改变
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent { physical_key, .. },
                ..
            } => match physical_key {
                PhysicalKey::Code(KeyCode::Escape) => event_loop.exit(),
                _ => {}
            },
            WindowEvent::RedrawRequested => {
                // surface重绘事件
            }
            _ => (),
        }
    }
}

// 根据不同平台初始化日志。
pub fn init_logger() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            // 使用查询字符串来获取日志级别。
            let query_string = web_sys::window().unwrap().location().search().unwrap();
            let query_level: Option<log::LevelFilter> = parse_url_query_string(&query_string, "RUST_LOG")
                .and_then(|x| x.parse().ok());

            // 我们将 wgpu 日志级别保持在错误级别，因为 Info 级别的日志输出非常多。
            let base_level = query_level.unwrap_or(log::LevelFilter::Info);
            let wgpu_level = query_level.unwrap_or(log::LevelFilter::Error);

            // 在 web 上，我们使用 fern，因为 console_log 没有按模块级别过滤功能。
            fern::Dispatch::new()
                .level(base_level)
                .level_for("wgpu_core", wgpu_level)
                .level_for("wgpu_hal", wgpu_level)
                .level_for("naga", wgpu_level)
                .chain(fern::Output::call(console_log::log))
                .apply()
                .unwrap();
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        } else if #[cfg(target_os = "android")] {
            // 添加 Android 平台的日志初始化
            android_logger::init_once(
                android_logger::Config::default()
                    .with_max_level(log::LevelFilter::Info)
            );
            log_panics::init();
        } else {
            // parse_default_env 会读取 RUST_LOG 环境变量，并在这些默认过滤器之上应用它。
            env_logger::builder()
                .filter_level(log::LevelFilter::Info)
                .filter_module("wgpu_core", log::LevelFilter::Info)
                .filter_module("wgpu_hal", log::LevelFilter::Error)
                .filter_module("naga", log::LevelFilter::Error)
                .parse_default_env()
                .init();
        }
    }
}

fn main() -> Result<(), impl std::error::Error> {
    init_logger();

    let events_loop = EventLoop::new().unwrap();
    let mut app = WgpuAppHandler::default();
    events_loop.run_app(&mut app)
}
