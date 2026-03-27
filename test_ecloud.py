#!/usr/bin/env python3
"""
天翼云盘 sessionKey 和 sessionSecret 获取工具

使用方法:
1. 在浏览器中登录 https://cloud.189.cn
2. 按 F12 打开开发者工具
3. 切换到 Network 标签
4. 刷新页面或执行任何文件操作
5. 找到 API 请求（如 listFiles.action）
6. 查看请求头中的 sessionKey 和 Signature
7. 运行此脚本，输入 sessionKey 和 sessionSecret 进行测试
"""

import base64
import hashlib
import hmac
import json
import time
from datetime import datetime, timezone

import requests

# 配置（从浏览器获取）
SESSION_KEY = ""  # 从请求头 SessionKey 获取
SESSION_SECRET = ""  # sessionSecret 需要从登录响应获取

HEADERS = {
    "Accept": "application/json;charset=UTF-8",
    "Referer": "https://cloud.189.cn/",
    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
}

API_BASE = "https://api.cloud.189.cn"


def get_http_date_str():
    """获取 GMT 格式时间"""
    return datetime.now(timezone.utc).strftime("%a, %d %b %Y %H:%M:%S GMT")


def generate_signature(session_secret, method, date):
    """生成 HMAC-SHA1 签名"""
    secret = session_secret.encode("utf-8")
    string_to_sign = f"{method}\n{date}\n".encode("utf-8")

    signature = hmac.new(secret, string_to_sign, hashlib.sha1).digest()
    return base64.b64encode(signature).decode("utf-8")


def test_list_files(session_key, session_secret):
    """测试列出文件"""
    print("=== 测试列出根目录文件 ===")

    url = f"{API_BASE}/v2/listFiles.action"
    params = {
        "folderId": "-11",
        "orderBy": "lastOpTime",
        "descending": "true",
        "showHidden": "false",
        "pageNum": "1",
        "pageSize": "20",
    }

    timestamp = get_http_date_str()
    request_id = f"{int(time.time() * 1000):x}"
    signature = generate_signature(session_secret, "GET", timestamp)

    headers = HEADERS.copy()
    headers.update(
        {
            "Date": timestamp,
            "X-Request-ID": request_id,
            "SessionKey": session_key,
            "Signature": signature,
        }
    )

    try:
        resp = requests.get(url, headers=headers, params=params, timeout=30)
        print(f"Status: {resp.status_code}")
        data = resp.json()

        if data.get("fileListAO"):
            files = data["fileListAO"].get("fileList", [])
            folders = data["fileListAO"].get("folderList", [])
            print(f"\n✓ 成功！找到 {len(folders)} 个文件夹，{len(files)} 个文件")

            if folders:
                print("\n文件夹:")
                for f in folders[:5]:
                    print(f"  - {f.get('name')}")

            if files:
                print("\n文件:")
                for f in files[:5]:
                    print(f"  - {f.get('name')} ({f.get('size', 0)} bytes)")

            return True
        else:
            print(f"\n✗ 失败：{data.get('errorCode')} - {data.get('errorMsg')}")
            return False
    except Exception as e:
        print(f"Error: {e}")
        return False


def main():
    print("天翼云盘 sessionKey/sessionSecret 测试工具")
    print("=" * 50)

    # 如果 SESSION_KEY 和 SESSION_SECRET 为空，提示用户输入
    if not SESSION_KEY or not SESSION_SECRET:
        print("\n请在浏览器中获取 sessionKey 和 sessionSecret:")
        print("1. 登录 https://cloud.189.cn")
        print("2. 按 F12 打开开发者工具")
        print("3. 切换到 Network 标签")
        print("4. 执行任何文件操作（如浏览文件夹）")
        print("5. 找到 API 请求（如 listFiles.action）")
        print("6. 查看请求头:")
        print("   - SessionKey: 复制这个值")
        print("   - sessionSecret: 需要从登录响应 getSessionForPC.action 中获取")
        print("\n或者在代码中直接设置 SESSION_KEY 和 SESSION_SECRET 变量")
        return

    print(f"\n使用配置:")
    print(f"session_key: {SESSION_KEY[:20]}...")
    print(f"session_secret: {SESSION_SECRET[:20]}...")

    success = test_list_files(SESSION_KEY, SESSION_SECRET)

    if success:
        print("\n=== 配置信息 ===")
        print("在 storage.toml 中配置:")
        print(f"""
[[private_registry.drivers]]
path = "ecloud"

[private_registry.drivers.config.Ecloud]
session_key = "{SESSION_KEY}"
session_secret = "{SESSION_SECRET}"
""")
    else:
        print("\n认证失败，请检查 sessionKey 和 sessionSecret 是否正确")


if __name__ == "__main__":
    main()
