use crate::protocol::{HandshakeMethod, SOCKS_VERSION};
use bytes::{BufMut, BytesMut};
use std::io::{Error, ErrorKind, Result};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// SOCKS5 handshake response
///
/// ```plain
/// +-----+--------+
/// | VER | METHOD |
/// +-----+--------+
/// |  1  |   1    |
/// +-----+--------+
/// ```
#[derive(Clone, Debug)]
pub struct HandshakeResponse {
    pub method: HandshakeMethod,
}

impl HandshakeResponse {
    pub fn new(method: HandshakeMethod) -> Self {
        Self { method }
    }

    pub async fn read_from<R: AsyncRead + Unpin>(r: &mut R) -> Result<Self> {
        let ver = r.read_u8().await?;

        if ver != SOCKS_VERSION {
            return Err(Error::new(
                ErrorKind::Unsupported,
                format!("Unsupported SOCKS version {0:#x}", ver),
            ));
        }

        let method = HandshakeMethod::from(r.read_u8().await?);

        Ok(Self { method })
    }

    pub async fn write_to<W: AsyncWrite + Unpin>(&self, w: &mut W) -> Result<()> {
        let mut buf = BytesMut::with_capacity(self.serialized_len());
        self.write_to_buf(&mut buf);
        w.write_all(&buf).await
    }

    pub fn write_to_buf<B: BufMut>(&self, buf: &mut B) {
        buf.put_u8(SOCKS_VERSION);
        buf.put_u8(u8::from(self.method));
    }

    pub fn serialized_len(&self) -> usize {
        2
    }
}
