use axum::{
    body::Body,
    extract::{Request, State},
    http::uri::Uri,
    response::{IntoResponse, Response},
    Router,
};
use clap::Parser;

use hyper::StatusCode;
// 使用hyper_tls支持Https请求
use hyper_tls::HttpsConnector;
use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioExecutor};
// 代理客户端
type Client = hyper_util::client::legacy::Client<hyper_tls::HttpsConnector<HttpConnector>, Body>;

/// 命令行
#[derive(Debug, Clone, Parser)]
#[command(
    about = r#"
    傻逼后端反向代理

    启动方式:
    sb_backend_proxy --host [你要代理的地址]
    访问方式:
    http://localhost:4000/你的api路径"#
)]
struct Cmd {
    /// 代理服务端口 默认4000
    #[arg(long, default_value = "4000")]
    server_port: u16,

    /// 要代理的目标地址 如 http://127.0.0.1:8888/
    #[arg(long)]
    host: String,

    /// 允许跨域的头 例如 Content-Type,Authorization等等 逗号分割
    #[arg(long)]
    allow_headers: Option<String>,
}

#[tokio::main]
async fn main() {
    // 处理命令行
    let cmd = Cmd::parse();
    // 处理日志
    tracing_subscriber::fmt::init();

    // 创建基于Hyper的代理客户端
    let client: Client =
        hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
            .build(HttpsConnector::new());
    // 设置处理函数 基于根地址
    let app = Router::new()
        .route("/:name/*path", axum::routing::any(handler_dynamic))
        .route("/:name", axum::routing::any(handler_static))
        .with_state((cmd.clone(), client));

    let address = format!("127.0.0.1:{}", cmd.server_port);
    // 监听地址
    let listener = tokio::net::TcpListener::bind(address.as_str())
        .await
        .unwrap();
    tracing::info!("傻逼后端反向代理正在监听端口 http://{}", address.as_str());
    tracing::info!("示例: 访问http://127.0.0.1:{}/你的api路径", cmd.server_port);

    // 监听
    axum::serve(listener, app).await.unwrap();
}

/// 处理动态路径
async fn handler_dynamic(
    state: State<(Cmd, Client)>,
    req: Request,
) -> Result<Response, impl IntoResponse> {
    handler(state, req).await
}
/// 处理根路径
async fn handler_static(
    state: State<(Cmd, Client)>,
    req: Request,
) -> Result<Response, impl IntoResponse> {
    handler(state, req).await
}

// 处理反向代理
async fn handler(
    State((cmd, client)): State<(Cmd, Client)>,
    mut req: Request,
) -> Result<Response, (StatusCode, String)> {
    // 如果是OPTIONS请求则直接返回
    if req.method() == hyper::Method::OPTIONS {
        tracing::info!("收到OPTIONS探路请求,伪造响应头");

        let mut response = Response::new(Body::empty());

        cors(&mut response, cmd.allow_headers.clone());

        Ok(response)
    } else {
        let path = req.uri().path();
        // 获取请求路径+Query 如果不满足则降级为Path
        let mut path_query = req
            .uri()
            .path_and_query()
            .map(|v| v.as_str())
            .unwrap_or(&path)
            .to_string();

        // 如果路径以/开头则去掉 / 由URL统一附加
        if path_query.starts_with("/") {
            // 扔掉第一位的/
            path_query = path_query[1..].to_string();
        }

        let uri = format!("{}/{}", cmd.host, path_query);

        tracing::info!("请求 {} [{}]", uri, req.method());

        *req.uri_mut() = Uri::try_from(uri).unwrap();

        let mut response = client
            .request(req)
            .await
            .map_err(|e| {
                tracing::error!("请求失败: {:?}", e);
                (StatusCode::BAD_REQUEST, e.to_string())
            })?
            .into_response();

        cors(&mut response, cmd.allow_headers.clone());

        Ok(response)
    }
}

/// Cors处理
fn cors(response: &mut Response, custom_headers: Option<String>) {
    // 追加跨域头
    response
        .headers_mut()
        .append("Access-Control-Allow-Origin", "*".parse().unwrap());
    response.headers_mut().append(
        "Access-Control-Request-Method",
        "GET,POST,PUT,DELETE,OPTIONS,HEAD,PATCH".parse().unwrap(),
    );
    response.headers_mut().append(
        "Access-Control-Allow-Headers",
        custom_headers
            .unwrap_or("Content-Type,Authorization".to_owned())
            .parse()
            .unwrap(),
    );
    response
        .headers_mut()
        .append("Access-Control-Allow-Credentials", "true".parse().unwrap());
}
