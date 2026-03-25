/**
 * API 请求模块
 * 提供统一的 API 请求接口
 */

const API_BASE = "/api";

/**
 * 获取认证头
 * @returns {Object} - 认证头对象
 */
function getAuthHeaders() {
  const headers = {};
  const authToken = localStorage.getItem("rlist_auth_token");
  if (authToken) {
    headers["AUTH-JWT-TOKEN"] = authToken;
  }
  return headers;
}

/**
 * 使用 Web Crypto API 计算 SHA512 哈希
 * @param {Uint8Array} data - 输入数据
 * @returns {Promise<string>} - 十六进制哈希字符串
 */
async function sha512(data) {
  const hashBuffer = await crypto.subtle.digest("SHA-512", data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");
}

/**
 * 将字符串转为 Uint8Array
 * @param {string} str - 字符串
 * @returns {Uint8Array} - 字节数组
 */
function stringToBytes(str) {
  return new TextEncoder().encode(str);
}

/**
 * 将数字转为 big-endian 8 字节数组
 * @param {bigint} num - 数字
 * @returns {Uint8Array} - 8 字节数组
 */
function bigIntToBigEndianBytes(num) {
  const bytes = new Uint8Array(8);
  for (let i = 7; i >= 0; i--) {
    bytes[i] = Number(num & 0xffn);
    num = num >> 8n;
  }
  return bytes;
}

/**
 * 合并多个 Uint8Array
 * @param  {...Uint8Array} arrays - 字节数组列表
 * @returns {Uint8Array} - 合并后的字节数组
 */
function mergeBytes(...arrays) {
  const totalLength = arrays.reduce((sum, arr) => sum + arr.length, 0);
  const result = new Uint8Array(totalLength);
  let offset = 0;
  for (const arr of arrays) {
    result.set(arr, offset);
    offset += arr.length;
  }
  return result;
}

/**
 * 获取 Challenge
 * @returns {Promise<Object>} - Challenge 结果
 */
async function getChallenge() {
  try {
    const response = await fetch(`${API_BASE}/challenge`, {
      method: "GET",
      headers: { "Content-Type": "application/json" },
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    // 手动解析 JSON，避免大整数精度丢失
    const text = await response.text();
    // 用正则提取 salt 的原始字符串值
    const saltMatch = text.match(/"salt"\s*:\s*(\d+)/);
    if (!saltMatch) {
      return {
        success: false,
        message: "获取 Challenge 失败：无法解析 salt",
      };
    }
    const salt = BigInt(saltMatch[1]);

    const result = JSON.parse(text);
    if (result.code === 200 && result.data) {
      return { success: true, salt: salt };
    } else {
      return {
        success: false,
        message: result.message || "获取 Challenge 失败",
      };
    }
  } catch (error) {
    return {
      success: false,
      message: "网络错误：" + error.message,
    };
  }
}

/**
 * 将权限对象转换为位掩码
 * 后端定义：read=bit0, download=bit1, upload=bit2, delete=bit3, move_obj=bit4, copy=bit5, create_dir=bit6, list=bit7
 * @param {Object} permissions - 权限对象
 * @returns {number} - 位掩码
 */
function permissionsToBits(permissions) {
  if (!permissions) return 0;
  let bits = 0;
  if (permissions.read) bits |= 1 << 0;
  if (permissions.download) bits |= 1 << 1;
  if (permissions.upload) bits |= 1 << 2;
  if (permissions.delete) bits |= 1 << 3;
  if (permissions.move_obj) bits |= 1 << 4;
  if (permissions.copy) bits |= 1 << 5;
  if (permissions.create_dir) bits |= 1 << 6;
  if (permissions.list) bits |= 1 << 7;
  return bits;
}

/**
 * 寻找满足难度要求的 nonce
 * 使用批量计算优化，每批处理多个 nonce
 * @param {bigint} salt - Salt 值
 * @param {number} timestamp - 时间戳
 * @param {string} username - 用户名
 * @param {string} password - 密码
 * @param {number} difficulty - 难度（前导 0 数量）
 * @param {Object} [permissions] - 权限对象（仅注册时需要）
 * @param {number} [batchSize=10000] - 每批计算的 nonce 数量
 * @returns {Promise<{nonce: string, claim: string}>}
 */
async function findValidNonce(
  salt,
  timestamp,
  username,
  password,
  difficulty = 4,
  permissions = null,
  batchSize = 10000,
) {
  const usernameBytes = stringToBytes(username);
  const passwordBytes = stringToBytes(password);
  const timestampBytes = bigIntToBigEndianBytes(BigInt(timestamp));
  const saltHex = salt.toString(16);
  const saltHexBytes = stringToBytes(saltHex);
  const targetZeros = "0".repeat(difficulty);

  // 注册时需要添加 permissions bits，登录时不需要
  const hasPermissions = permissions !== null && permissions !== undefined;
  const permBits = hasPermissions ? permissionsToBits(permissions) : 0;
  const permBytes = hasPermissions
    ? new Uint8Array([permBits])
    : new Uint8Array(0);

  let nonce = 0;
  while (true) {
    for (let i = 0; i < batchSize; i++) {
      const nonceStr = (nonce + i).toString();
      const nonceBytes = stringToBytes(nonceStr);

      const payload = mergeBytes(
        permBytes,
        nonceBytes,
        usernameBytes,
        passwordBytes,
        timestampBytes,
        saltHexBytes,
      );

      const hash = await sha512(payload);
      if (hash.startsWith(targetZeros)) {
        return { nonce: nonceStr, claim: hash };
      }
    }
    nonce += batchSize;
    // 每批让出主线程，避免阻塞 UI
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
}

/**
 * 统一的 API 请求方法
 * @param {string} endpoint - API 端点
 * @param {Object} options - fetch 选项
 * @returns {Promise<Object>} - 响应结果
 */
async function apiRequest(endpoint, options = {}) {
  const authHeaders = getAuthHeaders();

  const mergedOptions = {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...authHeaders,
      ...(options.headers || {}),
    },
  };

  try {
    const response = await fetch(`${API_BASE}${endpoint}`, mergedOptions);

    // 401: 未认证/Token 无效，需要重新登录
    if (response.status === 401) {
      showToast("认证失败，请重新登录", "error");
      logout();
      return { code: 401, message: "认证失败" };
    }

    // 403: 已认证但权限不足
    if (response.status === 403) {
      showToast("权限不足，无法执行此操作", "error");
      return { code: 403, message: "权限不足" };
    }

    // 尝试解析 JSON 响应
    let result;
    try {
      result = await response.json();
    } catch (parseError) {
      return {
        code: response.status,
        message: `请求失败：HTTP ${response.status}`,
      };
    }

    // 检查响应中的错误码
    if (result.code === 401) {
      showToast("认证失败，请重新登录", "error");
      logout();
      return { code: 401, message: "认证失败" };
    }

    if (result.code === 403) {
      showToast("权限不足，无法执行此操作", "error");
      return { code: 403, message: "权限不足" };
    }

    return result;
  } catch (error) {
    throw new Error("网络错误：" + error.message);
  }
}

/**
 * 清除认证信息
 */
function clearAuth() {
  localStorage.removeItem("rlist_auth_token");
  localStorage.removeItem("rlist_current_user");
}

/**
 * 退出登录
 */
function logout() {
  clearAuth();
  location.reload();
}

/**
 * 检查认证状态
 * @returns {Promise<boolean>} - 是否已认证
 */
async function checkAuth() {
  const authToken = localStorage.getItem("rlist_auth_token");
  const currentUser = localStorage.getItem("rlist_current_user");

  if (authToken) {
    try {
      const response = await fetch(`${API_BASE}/me`, {
        headers: getAuthHeaders(),
      });

      if (response.status === 401 || response.status === 403) {
        clearAuth();
        return false;
      }

      if (!response.ok) {
        clearAuth();
        return false;
      }

      const result = await response.json();
      if (result.code === 200) {
        return true;
      } else {
        clearAuth();
        return false;
      }
    } catch {
      clearAuth();
      return false;
    }
  }
  return false;
}

/**
 * 登录
 * @param {string} username - 用户名
 * @param {string} password - 密码
 * @returns {Promise<Object>} - 登录结果
 */
async function login(username, password) {
  try {
    // 获取 challenge
    const challengeResult = await getChallenge();
    if (!challengeResult.success) {
      return { success: false, message: challengeResult.message };
    }
    const salt = challengeResult.salt;

    // 获取当前时间戳（秒）
    const timestamp = Math.floor(Date.now() / 1000);

    // 寻找满足难度要求的 nonce
    const { nonce, claim } = await findValidNonce(
      salt,
      timestamp,
      username,
      password,
      4,
    );

    const response = await fetch(`${API_BASE}/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        username,
        password,
        salt: salt.toString(),
        timestamp,
        nonce,
        claim,
      }),
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const result = await response.json();
    if (result.code === 200 && result.data) {
      localStorage.setItem("rlist_auth_token", result.data.token);
      localStorage.setItem("rlist_current_user", username);
      return { success: true, token: result.data.token };
    } else {
      return { success: false, message: result.message || "登录失败" };
    }
  } catch (error) {
    return {
      success: false,
      message: "网络错误：" + error.message,
    };
  }
}

/**
 * 注册
 * @param {string} username - 用户名
 * @param {string} password - 密码
 * @returns {Promise<Object>} - 注册结果
 */
async function register(username, password) {
  try {
    // 获取 challenge
    const challengeResult = await getChallenge();
    if (!challengeResult.success) {
      return { success: false, message: challengeResult.message };
    }
    const salt = challengeResult.salt;

    // 获取当前时间戳（秒）
    const timestamp = Math.floor(Date.now() / 1000);

    // 寻找满足难度要求的 nonce
    const { nonce, claim } = await findValidNonce(
      salt,
      timestamp,
      username,
      password,
      4,
    );

    const response = await fetch(`${API_BASE}/register`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        username,
        password,
        salt: salt.toString(),
        timestamp,
        nonce,
        claim,
      }),
    });

    const result = await response.json();
    if (result.code === 200) {
      return { success: true, message: "注册成功，请登录" };
    } else {
      return { success: false, message: result.message || "注册失败" };
    }
  } catch (error) {
    return {
      success: false,
      message: "网络错误：" + error.message,
    };
  }
}

/**
 * 检查当前用户是否为管理员
 * @returns {Promise<boolean>} - 是否为管理员
 */
async function checkIsAdmin() {
  try {
    const response = await fetch(`${API_BASE}/admin/storage/list`, {
      method: "GET",
      headers: getAuthHeaders(),
    });

    // 403 表示不是管理员
    if (response.status === 403) {
      return false;
    }

    // 200 表示是管理员
    if (response.status === 200) {
      return true;
    }

    return false;
  } catch {
    return false;
  }
}

/**
 * 列出所有用户
 * @returns {Promise<Object>} - 用户列表结果
 */
async function listUsers() {
  return await apiRequest("/admin/user/list", {
    method: "GET",
  });
}

/**
 * 添加用户（管理员注册）
 * @param {string} username - 用户名
 * @param {string} password - 密码
 * @param {Object} permissions - 权限对象
 * @returns {Promise<Object>} - 添加结果
 */
async function addUser(username, password, permissions) {
  try {
    // 获取 challenge
    const challengeResult = await getChallenge();
    if (!challengeResult.success) {
      return { success: false, message: challengeResult.message };
    }
    const salt = challengeResult.salt;

    // 获取当前时间戳（秒）
    const timestamp = Math.floor(Date.now() / 1000);

    // 寻找满足难度要求的 nonce（传入 permissions）
    const { nonce, claim } = await findValidNonce(
      salt,
      timestamp,
      username,
      password,
      4,
      permissions,
    );

    return await apiRequest("/admin/user/register", {
      method: "POST",
      body: JSON.stringify({
        username,
        password,
        salt: salt.toString(),
        timestamp,
        nonce,
        claim,
        permissions,
      }),
    });
  } catch (error) {
    return {
      success: false,
      message: "网络错误：" + error.message,
    };
  }
}

/**
 * 删除用户
 * @param {string} username - 用户名
 * @returns {Promise<Object>} - 删除结果
 */
async function removeUser(username) {
  return await apiRequest("/admin/user/remove", {
    method: "POST",
    body: JSON.stringify({
      user_name: username,
    }),
  });
}
