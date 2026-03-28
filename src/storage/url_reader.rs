//! 通用 URL 内容读取器
//!
//! 支持 HTTP/HTTPS URL 的流式读取，可自定义请求头和请求体

use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use reqwest::Client;
use tokio::io::{AsyncRead, AsyncSeek, ReadBuf, SeekFrom};
use tokio::task::JoinHandle;

use super::model::FileContent;

const CONTENT_LENGTH: &str = "content-length";
const RANGE: &str = "range";

/// 通用 URL 内容读取器
pub struct UrlReader {
    url: String,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
    method: reqwest::Method,
    size: Option<u64>,
    hash: crate::storage::model::Hash,
    offset: u64,
    buffer: Vec<u8>,
    buffer_pos: usize,
    client: Client,
    chunk_size: u64,
    read_handle: Option<JoinHandle<Result<Vec<u8>, String>>>,
    eof: bool,
}

impl UrlReader {
    pub fn builder(url: impl Into<String>) -> UrlReaderBuilder {
        UrlReaderBuilder::new(url)
    }

    pub fn new(
        url: String,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
        method: reqwest::Method,
        size: Option<u64>,
        hash: crate::storage::model::Hash,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            url,
            headers,
            body,
            method,
            size,
            hash,
            offset: 0,
            buffer: Vec::new(),
            buffer_pos: 0,
            client,
            chunk_size: 64 * 1024, // 64KB
            read_handle: None,
            eof: false,
        }
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_hash(mut self, hash: crate::storage::model::Hash) -> Self {
        self.hash = hash;
        self
    }

    fn start_fetch(&mut self, cx: &mut Context<'_>) {
        if self.read_handle.is_some() || self.eof {
            return;
        }

        let offset = self.offset;
        let chunk_size = self.chunk_size;
        let url = self.url.clone();
        let headers = self.headers.clone();
        let method = self.method.clone();
        let body = self.body.clone();
        let size = self.size;
        let client = self.client.clone();

        let handle = tokio::spawn(async move {
            if size.is_some() && offset >= size.unwrap() {
                return Ok(Vec::new());
            }

            let end = size.map(|s| std::cmp::min(offset + chunk_size - 1, s - 1));

            let mut request = client.request(method, &url);
            for (key, value) in &headers {
                request = request.header(key, value);
            }
            if let Some(end) = end {
                request = request.header(RANGE, format!("bytes={}-{}", offset, end));
            } else if offset > 0 {
                request = request.header(RANGE, format!("bytes={}-", offset));
            }
            if let Some(ref body) = body {
                request = request.body(body.clone());
            }

            let response = request.send().await.map_err(|e| e.to_string())?;
            if !response.status().is_success() {
                if response.status().as_u16() == 416 {
                    return Ok(Vec::new());
                }
                return Err(format!("HTTP error: {}", response.status()));
            }

            let bytes = response.bytes().await.map_err(|e| e.to_string())?;
            Ok(bytes.to_vec())
        });

        self.read_handle = Some(handle);
        cx.waker().wake_by_ref();
    }
}

impl AsyncRead for UrlReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        // 如果缓冲区有数据，先返回
        if self.buffer_pos < self.buffer.len() {
            let available = &self.buffer[self.buffer_pos..];
            let to_copy = std::cmp::min(available.len(), buf.remaining());
            buf.put_slice(&available[..to_copy]);
            self.buffer_pos += to_copy;
            self.offset += to_copy as u64;
            return Poll::Ready(Ok(()));
        }

        // 缓冲区已用完，清空
        self.buffer.clear();
        self.buffer_pos = 0;

        // 检查是否到达末尾
        if self.eof {
            return Poll::Ready(Ok(()));
        }

        // 检查是否有正在进行的读取任务
        if let Some(handle) = &mut self.read_handle {
            match Pin::new(handle).poll(cx) {
                Poll::Ready(result) => {
                    self.read_handle = None;
                    match result {
                        Ok(Ok(data)) => {
                            if data.is_empty() {
                                self.eof = true;
                                return Poll::Ready(Ok(()));
                            }
                            self.buffer = data;
                            self.buffer_pos = 0;
                            cx.waker().wake_by_ref();
                            return Poll::Pending;
                        }
                        Ok(Err(e)) => {
                            return Poll::Ready(Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e,
                            )));
                        }
                        Err(e) => {
                            return Poll::Ready(Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!("Task join error: {}", e),
                            )));
                        }
                    }
                }
                Poll::Pending => return Poll::Pending,
            }
        }

        // 启动新的读取任务
        self.start_fetch(cx);
        Poll::Pending
    }
}

impl AsyncSeek for UrlReader {
    fn start_seek(mut self: Pin<&mut Self>, position: SeekFrom) -> std::io::Result<()> {
        let new_offset = match position {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => {
                if let Some(size) = self.size {
                    (size as i64 + offset) as u64
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "无法从末尾 seek，需要先获取文件大小",
                    ));
                }
            }
            SeekFrom::Current(offset) => (self.offset as i64 + offset) as u64,
        };
        self.offset = new_offset;
        self.buffer.clear();
        self.buffer_pos = 0;
        self.eof = false;
        // 取消正在进行的读取
        self.read_handle = None;
        Ok(())
    }

    fn poll_complete(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<u64>> {
        Poll::Ready(Ok(self.offset))
    }
}

impl FileContent for UrlReader {
    fn size(&self) -> Option<u64> {
        self.size
    }

    fn hash(&self) -> crate::storage::model::Hash {
        self.hash.clone()
    }
}

/// URL 读取器构建器
pub struct UrlReaderBuilder {
    url: String,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
    method: reqwest::Method,
    size: Option<u64>,
    hash: crate::storage::model::Hash,
}

impl UrlReaderBuilder {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            headers: HashMap::new(),
            body: None,
            method: reqwest::Method::GET,
            size: None,
            hash: crate::storage::model::Hash::Empty,
        }
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers.extend(headers);
        self
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    pub fn method(mut self, method: reqwest::Method) -> Self {
        self.method = method;
        self
    }

    pub fn size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn hash(mut self, hash: crate::storage::model::Hash) -> Self {
        self.hash = hash;
        self
    }

    pub fn build(self) -> UrlReader {
        UrlReader::new(
            self.url,
            self.headers,
            self.body,
            self.method,
            self.size,
            self.hash,
        )
    }

    /// 先发送 HEAD 请求获取文件大小，然后构建读取器
    pub async fn build_with_size(self) -> Result<UrlReader, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        let mut request = client.request(self.method.clone(), &self.url);
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        let response = request.send().await.map_err(|e| e.to_string())?;
        let size = response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        Ok(UrlReader::new(
            self.url,
            self.headers,
            self.body,
            self.method,
            size,
            self.hash,
        ))
    }
}
