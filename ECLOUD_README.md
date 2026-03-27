# 天翼云盘 (ecloud) 驱动配置指南

## 获取 sessionKey 和 sessionSecret

天翼云盘 API 使用基于 HMAC-SHA1 的签名认证，需要 `sessionKey` 和 `sessionSecret` 两个参数。

### 方法一：从浏览器获取（推荐）

1. **登录天翼云盘**
   - 打开浏览器访问 https://cloud.189.cn
   - 使用账号密码登录

2. **打开开发者工具**
   - 按 `F12` 打开开发者工具
   - 切换到 `Network`（网络）标签

3. **捕获 API 请求**
   - 在云盘页面执行任何操作（如浏览文件夹）
   - 在 Network 列表中找到类似 `listFiles.action` 的请求

4. **获取 sessionKey**
   - 点击该请求
   - 查看 `Headers`（请求头）
   - 找到 `SessionKey` 字段，复制其值

5. **获取 sessionSecret**
   - 在 Network 列表中找到 `getSessionForPC.action` 请求（登录时调用）
   - 查看 `Response`（响应）
   - 在 JSON 响应中找到 `data.sessionSecret` 字段，复制其值

### 方法二：使用 Python 脚本获取

运行 `test_ecloud.py` 脚本，按照提示输入 sessionKey 和 sessionSecret 进行测试。

## 配置 storage.toml

获取到 sessionKey 和 sessionSecret 后，在 `storage.toml` 中添加：

```toml
[[private_registry.drivers]]
path = "ecloud"

[private_registry.drivers.config.Ecloud]
session_key = "你的 sessionKey"
session_secret = "你的 sessionSecret"
```

## 注意事项

1. **sessionKey 有效期**
   - sessionKey 有有效期，过期后需要重新获取
   - 如果 API 返回 401 未授权错误，说明 sessionKey 已过期

2. **签名算法**
   - 使用 HMAC-SHA1 签名
   - 签名字符串格式：`METHOD\nDATE\n`
   - 使用 sessionSecret 作为密钥
   - 结果进行 Base64 编码

3. **请求头要求**
   - `Date`: GMT 格式时间戳
   - `X-Request-ID`: 唯一请求 ID
   - `SessionKey`: 会话标识
   - `Signature`: HMAC-SHA1 签名

## API 端点

- 基础 URL: `https://api.cloud.189.cn`
- 上传 URL: `https://upload.cloud.189.cn`

## 支持的功能

- ✅ 文件列表浏览
- ✅ 文件下载
- ✅ 文件上传（支持秒传）
- ✅ 创建文件夹
- ✅ 删除文件/文件夹
- ✅ 重命名
- ✅ 复制/移动

## 故障排除

### 问题：API 返回 "date/signature is null"

**原因**: 缺少必要的签名头

**解决**: 确保配置了正确的 sessionKey 和 sessionSecret

### 问题：API 返回 "InvalidSessionKey"

**原因**: sessionKey 已过期

**解决**: 重新登录获取新的 sessionKey 和 sessionSecret

### 问题：上传失败

**原因**: 可能是网络问题或文件已存在

**解决**: 检查网络连接，确认文件名不冲突
