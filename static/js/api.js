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
 * 统一的 API 请求方法
 * @param {string} endpoint - API 端点
 * @param {Object} options - fetch 选项
 * @returns {Promise<Object>} - 响应结果
 */
async function apiRequest(endpoint, options = {}) {
  const defaultOptions = {
    headers: {
      "Content-Type": "application/json",
      ...getAuthHeaders(),
    },
  };

  const mergedOptions = {
    ...defaultOptions,
    ...options,
    headers: {
      ...defaultOptions.headers,
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
    const response = await fetch(`${API_BASE}/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ username, password }),
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
    const response = await fetch(`${API_BASE}/register`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ username, password }),
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
