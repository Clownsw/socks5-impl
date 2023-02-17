use crate::protocol::{HandshakeMethod, SOCKS_VERSION};
use bytes::{BufMut, BytesMut};
use std::{
    io::{Error, ErrorKind, Result},
    mem::{self, ManuallyDrop},
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// SOCKS5 handshake request
///
/// ```plain
/// +-----+----------+----------+
/// | VER | NMETHODS | METHODS  |
/// +-----+----------+----------+
/// |  1  |    1     | 1 to 255 |
/// +-----+----------+----------|
/// ```
#[derive(Clone, Debug)]
pub struct HandshakeRequest {
    pub methods: Vec<HandshakeMethod>,
}

impl HandshakeRequest {
    pub fn new(methods: Vec<HandshakeMethod>) -> Self {
        Self { methods }
    }

    pub async fn from_stream<R: AsyncRead + Unpin>(r: &mut R) -> Result<Self> {
        let ver = r.read_u8().await?;

        if ver != SOCKS_VERSION {
            return Err(Error::new(
                ErrorKind::Unsupported,
                format!("Unsupported SOCKS version {0:#x}", ver),
            ));
        }

        let mlen = r.read_u8().await?;
        let mut methods = vec![0; mlen as usize];
        r.read_exact(&mut methods).await?;

        let methods = unsafe {
            let mut methods = ManuallyDrop::new(methods);

            Vec::from_raw_parts(
                methods.as_mut_ptr() as *mut HandshakeMethod,
                methods.len(),
                methods.capacity(),
            )
        };

        Ok(Self { methods })
    }

    pub async fn write_to<W: AsyncWrite + Unpin>(&self, w: &mut W) -> Result<()> {
        let mut buf = BytesMut::with_capacity(self.serialized_len());
        self.write_to_buf(&mut buf);
        w.write_all(&buf).await
    }

    pub fn write_to_buf<B: BufMut>(&self, buf: &mut B) {
        buf.put_u8(SOCKS_VERSION);
        buf.put_u8(self.methods.len() as u8);

        let methods = unsafe { mem::transmute(self.methods.as_slice()) };
        buf.put_slice(methods);
    }

    pub fn serialized_len(&self) -> usize {
        2 + self.methods.len()
    }
}
