use std::io::ErrorKind;
use std::pin::Pin;
use std::task::{Context, Poll};
use ntex::web;
use tokio::io::{AsyncRead, ReadBuf};
use ntex::util::{Stream};

pub struct PayloadAsyncReader {
    payload: web::types::Payload,
    buffer: Vec<u8>,
}

impl PayloadAsyncReader {
    pub fn new(payload: web::types::Payload) -> Self {
        let buf = Vec::new();
        PayloadAsyncReader { payload, buffer: buf }
    }
}

impl AsyncRead for PayloadAsyncReader {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        if this.buffer.is_empty() {
            match futures::ready!(Pin::new(&mut this.payload).poll_next(cx)) {
                Some(Ok(data)) => {
                    // let v = data.to_vec();
                    // this.buffer.copy_from_slice(&v);
                    this.buffer = data.to_vec();
                }
                Some(Err(err)) => {
                    return std::task::Poll::Ready(Err(std::io::Error::new(ErrorKind::Other, err)))
                }
                None => {
                    return std::task::Poll::Ready(Ok(()))
                }
            }
        }
        // 将缓冲区中的数据写入到给定的缓冲区中
        let bytes_to_copy = this.buffer.len();
        buf.put_slice(&this.buffer[..bytes_to_copy]);
        // 从缓冲区中移除已复制的数据
        this.buffer.drain(..bytes_to_copy);
        std::task::Poll::Ready(Ok(()))
    }

}


impl futures::AsyncRead for PayloadAsyncReader {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        if this.buffer.is_empty() {
            match futures::ready!(Pin::new(&mut this.payload).poll_next(cx)) {
                Some(Ok(data)) => {
                    // let v = data.to_vec();
                    // this.buffer.copy_from_slice(&v);
                    this.buffer = data.to_vec();
                }
                Some(Err(err)) => {
                    return std::task::Poll::Ready(Err(std::io::Error::new(ErrorKind::Other, err)))
                }
                None => {
                    return std::task::Poll::Ready(Ok(0))
                }
            }
        }
        // 将缓冲区中的数据写入到给定的缓冲区中
        let bytes_to_copy = this.buffer.len().min(buf.len());
        buf[..bytes_to_copy].copy_from_slice(&this.buffer[..bytes_to_copy]);

        // 从缓冲区中移除已复制的数据
        this.buffer.drain(..bytes_to_copy);

        std::task::Poll::Ready(Ok(bytes_to_copy))
    }
}
