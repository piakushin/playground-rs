use std::{
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    task::Poll,
};

use crate::runtime::register_poll_fd;

pub async fn accept(listener: &mut TcpListener) -> io::Result<(TcpStream, SocketAddr)> {
    std::future::poll_fn(|context| match listener.accept() {
        Ok((stream, socket)) => {
            // Important flag prevents IO from blocking on reads/writes.
            stream.set_nonblocking(true)?;
            Poll::Ready(Ok((stream, socket)))
        }
        Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
            // Register poll fd for OS to wakeup this future when fd is ready to read from.
            register_poll_fd(context, listener, libc::POLLIN);
            Poll::Pending
        }
        Err(err) => Poll::Ready(Err(err)),
    })
    .await
}

pub async fn write_all(mut buf: &[u8], stream: &mut TcpStream) -> io::Result<()> {
    std::future::poll_fn(|context| {
        while !buf.is_empty() {
            match stream.write(buf) {
                Ok(0) => {
                    let err = io::Error::from(io::ErrorKind::WriteZero);
                    return Poll::Ready(Err(err));
                }
                Ok(n) => buf = &buf[n..],
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    // Register poll fd for OS to wakeup this future when fd is ready to write from.
                    register_poll_fd(context, stream, libc::POLLOUT);
                    return Poll::Pending;
                }
                Err(err) => return Poll::Ready(Err(err)),
            }
        }
        Poll::Ready(Ok(()))
    })
    .await
}

pub async fn print_all(stream: &mut TcpStream) -> io::Result<()> {
    std::future::poll_fn(|context| loop {
        let mut buf = [0; 1024];
        match stream.read(&mut buf) {
            Ok(0) => return Poll::Ready(Ok(())),
            Ok(n) => io::stdout().write_all(&buf[..n])?,
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                // Register poll fd for OS to wakeup this future when fd is ready to read from.
                register_poll_fd(context, stream, libc::POLLIN);
                return Poll::Pending;
            }
            Err(err) => return Poll::Ready(Err(err)),
        };
    })
    .await
}
