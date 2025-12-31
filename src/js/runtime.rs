//! QuickJS 运行时

use rquickjs::{Context, Runtime, Function, Value, Ctx, Result as JsResult};
use std::cell::RefCell;
use std::rc::Rc;

/// JS 运行时
pub struct JsRuntime {
    runtime: Runtime,
    context: Context,
}

impl JsRuntime {
    pub fn new() -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|e| e.to_string())?;
        let context = Context::full(&runtime).map_err(|e| e.to_string())?;
        
        Ok(Self { runtime, context })
    }
    
    /// 执行 JS 代码
    pub fn eval(&self, code: &str) -> Result<String, String> {
        self.context.with(|ctx| {
            let result: JsResult<Value> = ctx.eval(code);
            match result {
                Ok(val) => Ok(value_to_string(&val)),
                Err(e) => {
                    // 尝试获取更详细的错误信息
                    let err_msg = format!("{:?}", e);
                    Err(err_msg)
                }
            }
        })
    }
    
    /// 执行 JS 文件
    pub fn eval_file(&self, path: &str) -> Result<String, String> {
        let code = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        self.eval(&code)
    }
    
    /// 注册全局函数（简化版，使用闭包包装）
    pub fn register_function<F>(&self, name: &str, func: F) -> Result<(), String>
    where
        F: Fn(Vec<String>) -> String + 'static,
    {
        // 将函数存储在 Rc<RefCell> 中以便在闭包中使用
        let func = Rc::new(RefCell::new(func));
        let name_owned = name.to_string();
        
        self.context.with(|ctx| {
            let global = ctx.globals();
            let func_clone = func.clone();
            
            // 创建一个简单的 JS 函数
            let js_func = Function::new(ctx.clone(), move |_ctx: Ctx, args: Vec<Value>| -> JsResult<String> {
                let string_args: Vec<String> = args
                    .iter()
                    .map(|v| value_to_string(v))
                    .collect();
                
                let f = func_clone.borrow();
                let result = f(string_args);
                Ok(result)
            });
            
            match js_func {
                Ok(f) => global.set(&name_owned, f).map_err(|e| e.to_string()),
                Err(e) => Err(e.to_string()),
            }
        })
    }
    
    /// 调用 JS 函数
    pub fn call_function(&self, name: &str, args: &[&str]) -> Result<String, String> {
        // 构建调用代码
        let args_str = args
            .iter()
            .map(|s| format!("\"{}\"", s.replace("\"", "\\\"")))
            .collect::<Vec<_>>()
            .join(", ");
        
        let code = format!("{}({})", name, args_str);
        self.eval(&code)
    }
    
    /// 设置全局变量
    pub fn set_global(&self, name: &str, value: &str) -> Result<(), String> {
        self.context.with(|ctx| {
            let global = ctx.globals();
            global.set(name, value).map_err(|e| e.to_string())
        })
    }
    
    /// 获取全局变量
    pub fn get_global(&self, name: &str) -> Result<String, String> {
        self.context.with(|ctx| {
            let global = ctx.globals();
            let val: JsResult<Value> = global.get(name);
            match val {
                Ok(v) => Ok(value_to_string(&v)),
                Err(e) => Err(e.to_string()),
            }
        })
    }
}

/// 将 JS Value 转换为字符串
fn value_to_string(val: &Value) -> String {
    if val.is_undefined() {
        "undefined".to_string()
    } else if val.is_null() {
        "null".to_string()
    } else if let Some(s) = val.as_string() {
        s.to_string().unwrap_or_default()
    } else if let Some(n) = val.as_int() {
        n.to_string()
    } else if let Some(n) = val.as_float() {
        n.to_string()
    } else if let Some(b) = val.as_bool() {
        b.to_string()
    } else {
        "[object]".to_string()
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create JS runtime")
    }
}
