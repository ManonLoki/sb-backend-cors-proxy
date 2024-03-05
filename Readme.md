# 傻逼后端反向代理

此工具专门解决部分脑残后端要求前端开发者自行解决跨域问题的脑残想法

编译环境  rust >= 1.76


# 构建方式

```
cargo build --release
```

# 使用方式
```
/// 启动
./sb-backend-cors-proxy --backend-host=你要代理的后端域名 如 http(s)://xxx.com

/// Api请求
http://127.0.0.1:4000/你的api地址
```

