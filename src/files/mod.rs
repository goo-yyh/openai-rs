//! 文件上传相关抽象。

use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use reqwest::multipart::Part;

use crate::error::{Error, Result};

/// 表示 Multipart 文本字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultipartField {
    /// 字段名称。
    pub name: String,
    /// 字段值。
    pub value: String,
}

/// 统一的文件输入类型别名。
pub type FileLike = UploadSource;

/// 表示一个可上传的文件来源。
#[derive(Clone)]
pub enum UploadSource {
    /// 直接由内存字节构成。
    Bytes {
        /// 文件字节。
        data: Bytes,
        /// 文件名。
        filename: String,
        /// 可选 MIME 类型。
        mime_type: Option<String>,
    },
    /// 由文件路径读取得到。
    Path {
        /// 原始路径。
        path: PathBuf,
        /// 文件字节。
        data: Bytes,
        /// 文件名。
        filename: String,
        /// 可选 MIME 类型。
        mime_type: Option<String>,
    },
    /// 由通用读取器读取得到。
    Reader {
        /// 文件字节。
        data: Bytes,
        /// 文件名。
        filename: String,
        /// 可选 MIME 类型。
        mime_type: Option<String>,
    },
}

impl UploadSource {
    /// 从文件路径创建上传源。
    ///
    /// # Errors
    ///
    /// 当文件不存在、无法读取或无法推导文件名时返回错误。
    pub fn from_path<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let data = std::fs::read(path)
            .map(Bytes::from)
            .map_err(|error| Error::InvalidConfig(format!("读取上传文件失败: {error}")))?;
        let filename = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| Error::InvalidConfig("无法从路径推导文件名".into()))?
            .to_owned();
        let mime_type = mime_guess::from_path(path).first_raw().map(str::to_owned);

        Ok(Self::Path {
            path: path.to_path_buf(),
            data,
            filename,
            mime_type,
        })
    }

    /// 从内存字节创建上传源。
    pub fn from_bytes<T, U>(bytes: T, filename: U) -> Self
    where
        T: Into<Bytes>,
        U: Into<String>,
    {
        Self::Bytes {
            data: bytes.into(),
            filename: filename.into(),
            mime_type: None,
        }
    }

    /// 从通用读取器读取字节并创建上传源。
    ///
    /// # Errors
    ///
    /// 当读取器读取失败时返回错误。
    pub fn from_reader<R, U>(mut reader: R, filename: U) -> Result<Self>
    where
        R: Read,
        U: Into<String>,
    {
        let mut buffer = Vec::new();
        reader
            .read_to_end(&mut buffer)
            .map_err(|error| Error::InvalidConfig(format!("读取上传流失败: {error}")))?;

        Ok(Self::Reader {
            data: Bytes::from(buffer),
            filename: filename.into(),
            mime_type: None,
        })
    }

    /// 覆盖 MIME 类型。
    pub fn with_mime_type<T>(mut self, mime_type: T) -> Self
    where
        T: Into<String>,
    {
        let mime_type = Some(mime_type.into());
        match &mut self {
            Self::Bytes {
                mime_type: target, ..
            }
            | Self::Path {
                mime_type: target, ..
            }
            | Self::Reader {
                mime_type: target, ..
            } => {
                *target = mime_type;
            }
        }
        self
    }

    /// 覆盖文件名。
    pub fn with_filename<T>(mut self, filename: T) -> Self
    where
        T: Into<String>,
    {
        let filename = filename.into();
        match &mut self {
            Self::Bytes {
                filename: target, ..
            }
            | Self::Path {
                filename: target, ..
            }
            | Self::Reader {
                filename: target, ..
            } => {
                *target = filename;
            }
        }
        self
    }

    /// 返回文件名。
    pub fn filename(&self) -> &str {
        match self {
            Self::Bytes { filename, .. }
            | Self::Path { filename, .. }
            | Self::Reader { filename, .. } => filename,
        }
    }

    /// 返回 MIME 类型。
    pub fn mime_type(&self) -> Option<&str> {
        match self {
            Self::Bytes { mime_type, .. }
            | Self::Path { mime_type, .. }
            | Self::Reader { mime_type, .. } => mime_type.as_deref(),
        }
    }

    /// 返回原始字节。
    pub fn bytes(&self) -> &Bytes {
        match self {
            Self::Bytes { data, .. } | Self::Path { data, .. } | Self::Reader { data, .. } => data,
        }
    }

    /// 把上传源转换为 `reqwest::multipart::Part`。
    ///
    /// # Errors
    ///
    /// 当 MIME 类型非法时返回错误。
    pub fn to_part(&self) -> Result<Part> {
        let mut part = Part::bytes(self.bytes().to_vec()).file_name(self.filename().to_owned());

        if let Some(mime_type) = self.mime_type() {
            part = part
                .mime_str(mime_type)
                .map_err(|error| Error::InvalidConfig(format!("非法 MIME 类型: {error}")))?;
        }

        Ok(part)
    }
}

impl fmt::Debug for UploadSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("UploadSource");
        debug.field("filename", &self.filename());
        debug.field("mime_type", &self.mime_type());
        debug.field("size", &self.bytes().len());

        if let Self::Path { path, .. } = self {
            debug.field("path", path);
        }

        debug.finish()
    }
}
