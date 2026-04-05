//! 资源句柄共用抽象。

use crate::Client;

/// 表示一个可以访问客户端句柄的资源对象。
pub trait Resource {
    /// 返回资源内部引用的客户端。
    fn client(&self) -> &Client;
}
