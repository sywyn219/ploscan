use {
    hyper::{
        Body,Client,Request,Response,Server,Uri,
        service::service_fn,

        rt::run,
    },
    futures::{
        compat::Future01CompatExt,
        future::{FutureExt, TryFutureExt},
    },
    std::net::SocketAddr,
};

async fn serve_req(_req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    println!("Got request at {:?}",_req.uri());
    let url_str = "http://www.rust-lang.org/en-US/";
    let url = url_str.parse::<Uri>().expect("failed to parse URL");
    let res = Client::new().get(url).compat().await;
    println!("request finished --returning response");
    res
    // Ok(Response::new(Body::from("hello, world!")))
}

async fn run_server(addr: SocketAddr) {
    println!("Listening on http://{}", addr);

    let serve_future = Server::bind(&addr)
        .serve(|| service_fn(|req|
            serve_req(req).boxed().compat()
        ));

    if let Err(e) = serve_future.compat().await {
        eprintln!("server error: {}", e);
    }
}

pub fn start() {
    let addr = SocketAddr::from(([127,0,0,1], 3000));
    let futures_03_future = run_server(addr);
    let futures_01_future =
        futures_03_future.unit_error().boxed().compat();
    run(futures_01_future);
}