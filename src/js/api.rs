//! 小程序 API 实现

use super::JsRuntime;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// 小程序 API
pub struct MiniAppApi {
    runtime: Arc<Mutex<JsRuntime>>,
    storage: Arc<Mutex<HashMap<String, String>>>,
}

impl MiniAppApi {
    pub fn new(runtime: Arc<Mutex<JsRuntime>>) -> Self {
        Self {
            runtime,
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// 初始化所有 API
    pub fn init(&self) -> Result<(), String> {
        // 只初始化最基本的
        println!("    init_wx_object...");
        self.init_wx_object().map_err(|e| format!("wx: {}", e))?;
        println!("    init_app...");
        self.init_app().map_err(|e| format!("app: {}", e))?;
        Ok(())
    }
    
    /// 初始化 wx 对象
    fn init_wx_object(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        rt.eval("var wx = {};")?;
        Ok(())
    }
    
    /// 初始化 console API
    fn init_console(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        
        // 先定义 native 函数的占位符
        rt.eval(r#"
            var __console_buffer = [];
            
            var console = {
                log: function() {
                    var msg = Array.prototype.slice.call(arguments).join(' ');
                    __console_buffer.push('[LOG] ' + msg);
                    // 直接打印到标准输出（通过 native 函数）
                    if (typeof __native_print === 'function') {
                        __native_print(msg);
                    }
                },
                error: function() {
                    var msg = Array.prototype.slice.call(arguments).join(' ');
                    __console_buffer.push('[ERROR] ' + msg);
                    if (typeof __native_print === 'function') {
                        __native_print('[ERROR] ' + msg);
                    }
                },
                warn: function() {
                    var msg = Array.prototype.slice.call(arguments).join(' ');
                    __console_buffer.push('[WARN] ' + msg);
                    if (typeof __native_print === 'function') {
                        __native_print('[WARN] ' + msg);
                    }
                },
                info: function() {
                    var msg = Array.prototype.slice.call(arguments).join(' ');
                    __console_buffer.push('[INFO] ' + msg);
                    if (typeof __native_print === 'function') {
                        __native_print(msg);
                    }
                }
            };
        "#)?;
        
        Ok(())
    }
    
    /// 初始化 storage API
    fn init_storage(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        
        rt.eval(r#"
            var wx = wx || {};
            
            wx.setStorageSync = function(key, value) {
                __native_storage_set(key, JSON.stringify(value));
            };
            
            wx.getStorageSync = function(key) {
                var value = __native_storage_get(key);
                try {
                    return JSON.parse(value);
                } catch(e) {
                    return value;
                }
            };
            
            wx.removeStorageSync = function(key) {
                __native_storage_remove(key);
            };
            
            wx.clearStorageSync = function() {
                __native_storage_clear();
            };
            
            // 异步版本
            wx.setStorage = function(options) {
                try {
                    wx.setStorageSync(options.key, options.data);
                    options.success && options.success();
                } catch(e) {
                    options.fail && options.fail(e);
                }
                options.complete && options.complete();
            };
            
            wx.getStorage = function(options) {
                try {
                    var data = wx.getStorageSync(options.key);
                    options.success && options.success({ data: data });
                } catch(e) {
                    options.fail && options.fail(e);
                }
                options.complete && options.complete();
            };
        "#)?;
        
        Ok(())
    }
    
    /// 初始化定时器 API
    fn init_timer(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        
        rt.eval(r#"
            var __timers = {};
            var __timer_id = 0;
            
            function setTimeout(callback, delay) {
                var id = ++__timer_id;
                __timers[id] = {
                    callback: callback,
                    delay: delay,
                    type: 'timeout',
                    startTime: Date.now()
                };
                __native_set_timer(id, delay, false);
                return id;
            }
            
            function setInterval(callback, delay) {
                var id = ++__timer_id;
                __timers[id] = {
                    callback: callback,
                    delay: delay,
                    type: 'interval',
                    startTime: Date.now()
                };
                __native_set_timer(id, delay, true);
                return id;
            }
            
            function clearTimeout(id) {
                delete __timers[id];
                __native_clear_timer(id);
            }
            
            function clearInterval(id) {
                delete __timers[id];
                __native_clear_timer(id);
            }
            
            // 由 native 调用
            function __trigger_timer(id) {
                var timer = __timers[id];
                if (timer) {
                    timer.callback();
                    if (timer.type === 'timeout') {
                        delete __timers[id];
                    }
                }
            }
        "#)?;
        
        Ok(())
    }
    
    /// 初始化 App API
    fn init_app(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        
        rt.eval(r#"
            var __app = null;
            var __pages = {};
            var __currentPage = null;
            var __pendingNavigation = null;
            
            function App(config) {
                __app = config;
                if (config.onLaunch) {
                    config.onLaunch();
                }
                return __app;
            }
            
            function getApp() {
                return __app;
            }
            
            function Page(config) {
                // 创建页面实例
                var page = {
                    data: config.data || {},
                    
                    // setData 方法 - 更新数据并触发重新渲染
                    setData: function(newData, callback) {
                        // 合并数据
                        for (var key in newData) {
                            if (newData.hasOwnProperty(key)) {
                                this.data[key] = newData[key];
                            }
                        }
                        // 通知 native 层数据更新
                        if (typeof __native_page_update === 'function') {
                            __native_page_update(JSON.stringify(this.data));
                        }
                        // 执行回调
                        if (callback) {
                            callback();
                        }
                    }
                };
                
                // 复制所有方法到页面实例
                for (var key in config) {
                    if (config.hasOwnProperty(key) && key !== 'data') {
                        if (typeof config[key] === 'function') {
                            page[key] = config[key].bind(page);
                        } else {
                            page[key] = config[key];
                        }
                    }
                }
                
                // 保存当前页面
                __currentPage = page;
                
                return page;
            }
            
            function getCurrentPages() {
                return __currentPage ? [__currentPage] : [];
            }
            
            // 获取当前页面实例（供 native 调用）
            function __getPageInstance() {
                return __currentPage;
            }
            
            // 调用页面方法（供 native 调用事件处理）
            function __callPageMethod(methodName, eventData) {
                if (__currentPage && typeof __currentPage[methodName] === 'function') {
                    var event = {
                        type: 'tap',
                        currentTarget: {
                            dataset: eventData || {}
                        },
                        detail: eventData || {}
                    };
                    __currentPage[methodName](event);
                    return true;
                }
                return false;
            }
            
            // 获取页面数据（供 native 调用）
            function __getPageData() {
                if (__currentPage) {
                    return JSON.stringify(__currentPage.data);
                }
                return '{}';
            }
            
            // 导航 API
            wx.navigateTo = function(options) {
                __pendingNavigation = {
                    type: 'navigateTo',
                    url: options.url
                };
                __native_print('[Navigate] navigateTo: ' + options.url);
                options.success && options.success();
            };
            
            wx.navigateBack = function(options) {
                options = options || {};
                __pendingNavigation = {
                    type: 'navigateBack',
                    delta: options.delta || 1
                };
                __native_print('[Navigate] navigateBack');
                options.success && options.success();
            };
            
            wx.switchTab = function(options) {
                __pendingNavigation = {
                    type: 'switchTab',
                    url: options.url
                };
                __native_print('[Navigate] switchTab: ' + options.url);
                options.success && options.success();
            };
            
            wx.redirectTo = function(options) {
                __pendingNavigation = {
                    type: 'navigateTo',
                    url: options.url
                };
                __native_print('[Navigate] redirectTo: ' + options.url);
                options.success && options.success();
            };
            
            wx.reLaunch = function(options) {
                __pendingNavigation = {
                    type: 'switchTab',
                    url: options.url
                };
                __native_print('[Navigate] reLaunch: ' + options.url);
                options.success && options.success();
            };
            
            // Toast API
            wx.showToast = function(options) {
                __native_print('[Toast] ' + (options.title || ''));
                options.success && options.success();
            };
            
            wx.hideToast = function() {};
            wx.showLoading = function(options) {
                __native_print('[Loading] ' + (options.title || ''));
            };
            wx.hideLoading = function() {};
        "#)?;
        
        Ok(())
    }
    
    /// 初始化 UI API
    fn init_ui(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        
        rt.eval(r#"
            // UI 相关 API
            wx.showToast = function(options) {
                __native_show_toast(options.title, options.icon || 'success', options.duration || 1500);
                options.success && options.success();
            };
            
            wx.hideToast = function() {
                __native_hide_toast();
            };
            
            wx.showLoading = function(options) {
                __native_show_loading(options.title);
                options.success && options.success();
            };
            
            wx.hideLoading = function() {
                __native_hide_loading();
            };
            
            wx.showModal = function(options) {
                __native_show_modal(
                    options.title || '',
                    options.content || '',
                    options.showCancel !== false,
                    options.confirmText || '确定',
                    options.cancelText || '取消'
                );
            };
            
            wx.showActionSheet = function(options) {
                __native_show_action_sheet(JSON.stringify(options.itemList));
            };
            
            // 获取系统信息
            wx.getSystemInfoSync = function() {
                return {
                    platform: 'mini-app',
                    version: '1.0.0',
                    SDKVersion: '1.0.0',
                    windowWidth: __native_get_window_width(),
                    windowHeight: __native_get_window_height(),
                    pixelRatio: 2
                };
            };
            
            wx.getSystemInfo = function(options) {
                var info = wx.getSystemInfoSync();
                options.success && options.success(info);
                options.complete && options.complete();
            };
            
            // Canvas API
            wx.createCanvasContext = function(canvasId) {
                return {
                    _canvasId: canvasId,
                    _commands: [],
                    
                    setFillStyle: function(color) {
                        this._commands.push({ type: 'setFillStyle', color: color });
                    },
                    setStrokeStyle: function(color) {
                        this._commands.push({ type: 'setStrokeStyle', color: color });
                    },
                    setLineWidth: function(width) {
                        this._commands.push({ type: 'setLineWidth', width: width });
                    },
                    fillRect: function(x, y, w, h) {
                        this._commands.push({ type: 'fillRect', x: x, y: y, w: w, h: h });
                    },
                    strokeRect: function(x, y, w, h) {
                        this._commands.push({ type: 'strokeRect', x: x, y: y, w: w, h: h });
                    },
                    arc: function(x, y, r, start, end) {
                        this._commands.push({ type: 'arc', x: x, y: y, r: r, start: start, end: end });
                    },
                    fill: function() {
                        this._commands.push({ type: 'fill' });
                    },
                    stroke: function() {
                        this._commands.push({ type: 'stroke' });
                    },
                    beginPath: function() {
                        this._commands.push({ type: 'beginPath' });
                    },
                    closePath: function() {
                        this._commands.push({ type: 'closePath' });
                    },
                    moveTo: function(x, y) {
                        this._commands.push({ type: 'moveTo', x: x, y: y });
                    },
                    lineTo: function(x, y) {
                        this._commands.push({ type: 'lineTo', x: x, y: y });
                    },
                    draw: function(reserve, callback) {
                        __native_canvas_draw(this._canvasId, JSON.stringify(this._commands));
                        if (!reserve) {
                            this._commands = [];
                        }
                        callback && callback();
                    }
                };
            };
        "#)?;
        
        Ok(())
    }
}
