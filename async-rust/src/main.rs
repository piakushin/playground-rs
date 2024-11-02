use std::{
    future::Future,
    io,
    net::{TcpListener, TcpStream},
    pin::Pin,
    time::Duration,
};

mod async_io;
mod foo;
mod join_all;
mod runtime;
mod sleep;
mod spawn;
mod timeout;

type DynFuture = Pin<Box<dyn Future<Output = ()> + Send + Sync>>;

async fn one_response(mut socket: TcpStream, n: u64) -> io::Result<()> {
    let start_msg = format!("reqwest start: {n}\n");
    async_io::write_all(start_msg.as_bytes(), &mut socket).await?;
    sleep::sleep(Duration::from_secs(1)).await;
    let end_msg = format!("reqwest end: {n}\n");
    async_io::write_all(end_msg.as_bytes(), &mut socket).await?;
    Ok(())
}

async fn server_main(mut listener: TcpListener) -> io::Result<()> {
    let mut n = 100;
    loop {
        let (socket, _) = async_io::accept(&mut listener).await?;
        spawn::spawn(async move { one_response(socket, n).await.unwrap() });
        n += 1;
    }
}

async fn client_main() -> io::Result<()> {
    let mut stream = TcpStream::connect("0.0.0.0:8000")?;
    // Important flag prevents IO from blocking on reads/writes.
    stream.set_nonblocking(true)?;
    async_io::print_all(&mut stream).await
}

async fn async_main() {
    let mut handles: Vec<_> = (11..=20).map(|n| spawn::spawn(foo::foo(n))).collect();

    let mut futures = Vec::new();
    for n in 1..=10 {
        futures.push(foo::foo(n));
    }

    // Run a custom joined custom futures.
    // Each run for 1 sec but we implemented async runtime and all of them complete simultaniously.
    handles.push(spawn::spawn(join_all::join_all(futures)));

    for handle in handles {
        handle.await;
    }

    // Run async server.
    let listener = TcpListener::bind("0.0.0.0:8000").unwrap();
    // Important flag prevents IO from blocking on reads/writes.
    listener.set_nonblocking(true).unwrap();
    spawn::spawn(async { server_main(listener).await.unwrap() });

    // And run several clients printing server response.
    let client_handles: Vec<_> = (100..150).map(|_| spawn::spawn(client_main())).collect();
    for handle in client_handles {
        handle.await.unwrap();
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let main_task = Box::pin(async_main());

    runtime::enter(main_task)
}
