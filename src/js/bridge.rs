//! JS 与 Native 桥接层

use super::JsRuntime;
use crate::ui::ComponentTree;
use crate::event::{Event, Touch};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// JS 桥接器
pub struct JsBridge {
    runtime: Arc<Mutex<JsRuntime>>,
    component_tree: Arc<Mutex<ComponentTree>>,
    storage: Arc<Mutex<HashMap<String, String>>>,
    timers: Arc<Mutex<HashMap<u32, TimerInfo>>>,
    event_queue: Arc<Mutex<Vec<BridgeEvent>>>,
}

struct TimerInfo {
    delay: u32,
    repeat: bool,
    start_time: u64,
}

/// 桥接事件
#[derive(Debug, Clone)]
pub enum BridgeEvent {
    ConsoleLog(String),
    ConsoleError(String),
    ConsoleWarn(String),
    ShowToast { title: String, icon: String, duration: u32 },
    HideToast,
    ShowLoading(String),
    HideLoading,
    ShowModal { title: String, content: String },
    NavigateTo(String),
    NavigateBack(u32),
    SetTimer { id: u32, delay: u32, repeat: bool },
    ClearTimer(u32),
    CanvasDraw { canvas_id: String, commands: String },
}

impl JsBridge {
    pub fn new(runtime: Arc<Mutex<JsRuntime>>) -> Self {
        Self {
            runtime,
            component_tree: Arc::new(Mutex::new(ComponentTree::new())),
            storage: Arc::new(Mutex::new(HashMap::new())),
            timers: Arc::new(Mutex::new(HashMap::new())),
            event_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// 初始化 native 函数
    pub fn init(&self) -> Result<(), String> {
        println!("    register_print_function...");
        self.register_print_function().map_err(|e| format!("print: {}", e))?;
        Ok(())
    }
    
    fn register_print_function(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        
        // 使用纯 JS 实现 print，将消息存储到数组中
        rt.eval(r#"
            var __print_buffer = [];
            function __native_print(msg) {
                __print_buffer.push(String(msg));
            }
            
            // 页面数据更新回调（供 Page.setData 调用）
            function __native_page_update(dataJson) {
                __print_buffer.push('[PageUpdate] ' + dataJson);
            }
        "#)?;
        
        println!("      Print function defined");
        Ok(())
    }
    
    fn register_console_functions(&self) -> Result<(), String> {
        let queue = self.event_queue.clone();
        let rt = self.runtime.lock().unwrap();
        
        // console.log
        let q = queue.clone();
        rt.register_function("__native_console_log", move |args| {
            let msg = args.join(" ");
            println!("[JS] {}", msg);
            q.lock().unwrap().push(BridgeEvent::ConsoleLog(msg));
            "undefined".to_string()
        })?;
        
        // console.error
        let q = queue.clone();
        rt.register_function("__native_console_error", move |args| {
            let msg = args.join(" ");
            eprintln!("[JS ERROR] {}", msg);
            q.lock().unwrap().push(BridgeEvent::ConsoleError(msg));
            "undefined".to_string()
        })?;
        
        // console.warn
        let q = queue.clone();
        rt.register_function("__native_console_warn", move |args| {
            let msg = args.join(" ");
            eprintln!("[JS WARN] {}", msg);
            q.lock().unwrap().push(BridgeEvent::ConsoleWarn(msg));
            "undefined".to_string()
        })?;
        
        Ok(())
    }
    
    fn register_storage_functions(&self) -> Result<(), String> {
        let storage = self.storage.clone();
        let rt = self.runtime.lock().unwrap();
        
        // setStorage
        let s = storage.clone();
        rt.register_function("__native_storage_set", move |args| {
            if args.len() >= 2 {
                s.lock().unwrap().insert(args[0].clone(), args[1].clone());
            }
            "undefined".to_string()
        })?;
        
        // getStorage
        let s = storage.clone();
        rt.register_function("__native_storage_get", move |args| {
            if let Some(key) = args.first() {
                s.lock().unwrap().get(key).cloned().unwrap_or_default()
            } else {
                String::new()
            }
        })?;
        
        // removeStorage
        let s = storage.clone();
        rt.register_function("__native_storage_remove", move |args| {
            if let Some(key) = args.first() {
                s.lock().unwrap().remove(key);
            }
            "undefined".to_string()
        })?;
        
        // clearStorage
        let s = storage.clone();
        rt.register_function("__native_storage_clear", move |_args| {
            s.lock().unwrap().clear();
            "undefined".to_string()
        })?;
        
        Ok(())
    }
    
    fn register_timer_functions(&self) -> Result<(), String> {
        let queue = self.event_queue.clone();
        let timers = self.timers.clone();
        let rt = self.runtime.lock().unwrap();
        
        // setTimer
        let q = queue.clone();
        let t = timers.clone();
        rt.register_function("__native_set_timer", move |args| {
            if args.len() >= 3 {
                let id: u32 = args[0].parse().unwrap_or(0);
                let delay: u32 = args[1].parse().unwrap_or(0);
                let repeat = args[2] == "true";
                
                t.lock().unwrap().insert(id, TimerInfo {
                    delay,
                    repeat,
                    start_time: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                });
                
                q.lock().unwrap().push(BridgeEvent::SetTimer { id, delay, repeat });
            }
            "undefined".to_string()
        })?;
        
        // clearTimer
        let q = queue.clone();
        let t = timers.clone();
        rt.register_function("__native_clear_timer", move |args| {
            if let Some(id_str) = args.first() {
                let id: u32 = id_str.parse().unwrap_or(0);
                t.lock().unwrap().remove(&id);
                q.lock().unwrap().push(BridgeEvent::ClearTimer(id));
            }
            "undefined".to_string()
        })?;
        
        Ok(())
    }
    
    fn register_ui_functions(&self) -> Result<(), String> {
        let queue = self.event_queue.clone();
        let rt = self.runtime.lock().unwrap();
        
        // showToast
        let q = queue.clone();
        rt.register_function("__native_show_toast", move |args| {
            let title = args.get(0).cloned().unwrap_or_default();
            let icon = args.get(1).cloned().unwrap_or_else(|| "success".to_string());
            let duration: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1500);
            q.lock().unwrap().push(BridgeEvent::ShowToast { title, icon, duration });
            "undefined".to_string()
        })?;
        
        // hideToast
        let q = queue.clone();
        rt.register_function("__native_hide_toast", move |_args| {
            q.lock().unwrap().push(BridgeEvent::HideToast);
            "undefined".to_string()
        })?;
        
        // showLoading
        let q = queue.clone();
        rt.register_function("__native_show_loading", move |args| {
            let title = args.get(0).cloned().unwrap_or_default();
            q.lock().unwrap().push(BridgeEvent::ShowLoading(title));
            "undefined".to_string()
        })?;
        
        // hideLoading
        let q = queue.clone();
        rt.register_function("__native_hide_loading", move |_args| {
            q.lock().unwrap().push(BridgeEvent::HideLoading);
            "undefined".to_string()
        })?;
        
        // showModal
        let q = queue.clone();
        rt.register_function("__native_show_modal", move |args| {
            let title = args.get(0).cloned().unwrap_or_default();
            let content = args.get(1).cloned().unwrap_or_default();
            q.lock().unwrap().push(BridgeEvent::ShowModal { title, content });
            "undefined".to_string()
        })?;
        
        // 窗口尺寸
        rt.register_function("__native_get_window_width", move |_args| {
            "800".to_string()
        })?;
        
        rt.register_function("__native_get_window_height", move |_args| {
            "600".to_string()
        })?;
        
        // Canvas 绘制
        let q = queue.clone();
        rt.register_function("__native_canvas_draw", move |args| {
            let canvas_id = args.get(0).cloned().unwrap_or_default();
            let commands = args.get(1).cloned().unwrap_or_else(|| "[]".to_string());
            q.lock().unwrap().push(BridgeEvent::CanvasDraw { canvas_id, commands });
            "undefined".to_string()
        })?;
        
        Ok(())
    }
    
    fn register_navigation_functions(&self) -> Result<(), String> {
        let queue = self.event_queue.clone();
        let rt = self.runtime.lock().unwrap();
        
        // navigateTo
        let q = queue.clone();
        rt.register_function("__native_navigate_to", move |args| {
            if let Some(url) = args.first() {
                q.lock().unwrap().push(BridgeEvent::NavigateTo(url.clone()));
            }
            "undefined".to_string()
        })?;
        
        // redirectTo
        let q = queue.clone();
        rt.register_function("__native_redirect_to", move |args| {
            if let Some(url) = args.first() {
                q.lock().unwrap().push(BridgeEvent::NavigateTo(url.clone()));
            }
            "undefined".to_string()
        })?;
        
        // navigateBack
        let q = queue.clone();
        rt.register_function("__native_navigate_back", move |args| {
            let delta: u32 = args.first().and_then(|s| s.parse().ok()).unwrap_or(1);
            q.lock().unwrap().push(BridgeEvent::NavigateBack(delta));
            "undefined".to_string()
        })?;
        
        // switchTab
        let q = queue.clone();
        rt.register_function("__native_switch_tab", move |args| {
            if let Some(url) = args.first() {
                q.lock().unwrap().push(BridgeEvent::NavigateTo(url.clone()));
            }
            "undefined".to_string()
        })?;
        
        Ok(())
    }
    
    /// 获取并清空事件队列
    pub fn drain_events(&self) -> Vec<BridgeEvent> {
        let mut queue = self.event_queue.lock().unwrap();
        std::mem::take(&mut *queue)
    }
    
    /// 触发 JS 事件
    pub fn dispatch_event(&self, event: &Event) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        
        match event {
            Event::Tap(tap) => {
                rt.eval(&format!(
                    "__app && __app.onTap && __app.onTap({{ x: {}, y: {}, timestamp: {} }})",
                    tap.x, tap.y, tap.timestamp
                ))?;
            }
            Event::TouchStart(touch) => {
                let touches_json = self.touches_to_json(&touch.touches);
                rt.eval(&format!(
                    "__app && __app.onTouchStart && __app.onTouchStart({{ touches: {} }})",
                    touches_json
                ))?;
            }
            Event::AppShow => {
                rt.eval("__app && __app.onShow && __app.onShow()")?;
            }
            Event::AppHide => {
                rt.eval("__app && __app.onHide && __app.onHide()")?;
            }
            Event::PageLoad => {
                rt.eval("__currentPage && __currentPage.onLoad && __currentPage.onLoad()")?;
            }
            Event::PageShow => {
                rt.eval("__currentPage && __currentPage.onShow && __currentPage.onShow()")?;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn touches_to_json(&self, touches: &[Touch]) -> String {
        let items: Vec<String> = touches
            .iter()
            .map(|t| format!("{{ id: {}, x: {}, y: {} }}", t.id, t.x, t.y))
            .collect();
        format!("[{}]", items.join(", "))
    }
    
    /// 触发定时器
    pub fn trigger_timer(&self, id: u32) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        rt.eval(&format!("__trigger_timer({})", id))?;
        Ok(())
    }
    
    /// 获取组件树
    pub fn component_tree(&self) -> Arc<Mutex<ComponentTree>> {
        self.component_tree.clone()
    }
}
