// 全局变量
const API_BASE = "/api";
let currentPath = "/";
let selectedFiles = new Set();
let adminKey = localStorage.getItem("rlist_admin_key") || "";
let filesData = [];
let currentView = localStorage.getItem("rlist_view") || "list";
let currentTheme = localStorage.getItem("rlist_theme") || "light";
let previewFilePath = "";
let contextMenuTarget = null;
let authToken = localStorage.getItem("rlist_auth_token") || "";
let currentUser = localStorage.getItem("rlist_current_user") || "";

// 初始化
document.addEventListener("DOMContentLoaded", () => {
  // 初始化主题
  if (currentTheme === "dark") {
    document.documentElement.setAttribute("data-theme", "dark");
    document.getElementById("themeIcon").className = "ti ti-sun";
  }

  if (adminKey) {
    document.getElementById("adminKeyInput").value = adminKey;
  }

  // 检查登录状态
  checkAuth();

  // 点击关闭上下文菜单
  document.addEventListener("click", (e) => {
    if (!e.target.closest(".context-menu")) {
      hideContextMenu();
    }
  });
});

// ==================== 认证相关函数 ====================

// 检查认证状态
async function checkAuth() {
  if (authToken) {
    // 先验证 token 是否有效
    try {
      const response = await fetch(`${API_BASE}/me`, {
        headers: getAuthHeaders(),
      });

      // 401: Token 无效或过期，需要重新登录
      if (response.status === 401) {
        console.log("Token 已过期或无效");
        clearAuth();
        showLogin();
        return;
      }

      // 403: 权限不足，但 token 有效，不需要重新登录
      if (response.status === 403) {
        console.log("权限不足");
        showToast("权限不足，无法执行此操作", "error");
        return;
      }

      if (!response.ok) {
        throw new Error("Token 无效");
      }

      const result = await response.json();

      if (result.code === 200) {
        // Token 有效，显示主界面
        document.getElementById("authContainer").style.display = "none";
        document.getElementById("mainContainer").style.display = "block";
        document.getElementById("userInfo").style.display = "flex";
        document.getElementById("usernameDisplay").textContent = currentUser;
        setView(currentView);
        loadFiles();
      } else {
        // Token 无效，清除认证信息
        clearAuth();
        showLogin();
      }
    } catch (error) {
      console.log("Token 验证失败，清除旧 token");
      clearAuth();
      showLogin();
    }
  } else {
    // 未登录，显示登录界面
    showLogin();
  }
}

// 显示登录界面
function showLogin() {
  document.getElementById("authContainer").style.display = "flex";
  document.getElementById("mainContainer").style.display = "none";
}

// 清除认证信息
function clearAuth() {
  authToken = "";
  currentUser = "";
  localStorage.removeItem("rlist_auth_token");
  localStorage.removeItem("rlist_current_user");
}

// 切换认证标签页
function showAuthTab(tab) {
  const loginTab = document.getElementById("loginTab");
  const registerTab = document.getElementById("registerTab");
  const loginForm = document.getElementById("loginForm");
  const registerForm = document.getElementById("registerForm");
  const authMessage = document.getElementById("authMessage");

  authMessage.textContent = "";
  authMessage.className = "";

  if (tab === "login") {
    loginTab.classList.add("active");
    registerTab.classList.remove("active");
    loginForm.style.display = "block";
    registerForm.style.display = "none";
  } else {
    loginTab.classList.remove("active");
    registerTab.classList.add("active");
    loginForm.style.display = "none";
    registerForm.style.display = "block";
  }
}

// 登录
async function login() {
  const username = document.getElementById("loginUsername").value.trim();
  const password = document.getElementById("loginPassword").value;
  const authMessage = document.getElementById("authMessage");

  if (!username || !password) {
    authMessage.textContent = "请输入用户名和密码";
    authMessage.className = "error";
    return;
  }

  authMessage.textContent = "登录中...";
  authMessage.className = "";

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
      // 保存 token 和用户信息
      authToken = result.data.token;
      currentUser = username;
      localStorage.setItem("rlist_auth_token", authToken);
      localStorage.setItem("rlist_current_user", currentUser);

      authMessage.textContent = "登录成功，正在跳转...";
      authMessage.className = "success";

      setTimeout(() => {
        checkAuth();
      }, 1000);
    } else {
      authMessage.textContent = result.message || "登录失败";
      authMessage.className = "error";
    }
  } catch (error) {
    console.error("登录错误:", error);
    authMessage.textContent =
      "网络错误：" + error.message + "，请确认服务器是否正常运行";
    authMessage.className = "error";
  }
}

// 注册
async function register() {
  const username = document.getElementById("registerUsername").value.trim();
  const password = document.getElementById("registerPassword").value;
  const passwordConfirm = document.getElementById(
    "registerPasswordConfirm",
  ).value;
  const authMessage = document.getElementById("authMessage");

  if (!username || !password) {
    authMessage.textContent = "请输入用户名和密码";
    authMessage.className = "error";
    return;
  }

  if (password.length < 6) {
    authMessage.textContent = "密码长度至少为 6 位";
    authMessage.className = "error";
    return;
  }

  if (password !== passwordConfirm) {
    authMessage.textContent = "两次输入的密码不一致";
    authMessage.className = "error";
    return;
  }

  try {
    const response = await fetch(`${API_BASE}/register`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ username, password }),
    });

    const result = await response.json();
    if (result.code === 200) {
      authMessage.textContent = "注册成功，请登录";
      authMessage.className = "success";

      // 清空注册表单
      document.getElementById("registerUsername").value = "";
      document.getElementById("registerPassword").value = "";
      document.getElementById("registerPasswordConfirm").value = "";

      // 切换到登录标签页
      setTimeout(() => {
        showAuthTab("login");
      }, 1500);
    } else {
      authMessage.textContent = result.message || "注册失败";
      authMessage.className = "error";
    }
  } catch (error) {
    authMessage.textContent = "网络错误：" + error.message;
    authMessage.className = "error";
  }
}

// 退出登录
function logout() {
  // 清除本地存储
  authToken = "";
  currentUser = "";
  localStorage.removeItem("rlist_auth_token");
  localStorage.removeItem("rlist_current_user");

  // 重新加载页面
  location.reload();
}

// 获取认证头
function getAuthHeaders() {
  const headers = {};
  if (authToken) {
    headers["AUTH-JWT-TOKEN"] = authToken;
  }
  return headers;
}

// 统一的 API 请求辅助函数
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
      // 如果响应不是 JSON 格式，返回通用错误
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

// 主题切换
function toggleTheme() {
  const isDark = document.documentElement.getAttribute("data-theme") === "dark";
  if (isDark) {
    document.documentElement.removeAttribute("data-theme");
    document.getElementById("themeIcon").className = "ti ti-moon";
    currentTheme = "light";
  } else {
    document.documentElement.setAttribute("data-theme", "dark");
    document.getElementById("themeIcon").className = "ti ti-sun";
    currentTheme = "dark";
  }
  localStorage.setItem("rlist_theme", currentTheme);
}

// 视图切换
function setView(view) {
  currentView = view;
  localStorage.setItem("rlist_view", view);
  const fileList = document.getElementById("fileList");
  const listBtn = document.getElementById("listViewBtn");
  const gridBtn = document.getElementById("gridViewBtn");

  if (view === "grid") {
    fileList.classList.add("grid-view");
    listBtn.classList.remove("active");
    gridBtn.classList.add("active");
  } else {
    fileList.classList.remove("grid-view");
    listBtn.classList.add("active");
    gridBtn.classList.remove("active");
  }
}

// 加载文件列表
async function loadFiles() {
  const fileList = document.getElementById("fileList");
  fileList.innerHTML = '<div class="loading"><div class="spinner"></div></div>';

  try {
    const response = await fetch(
      `${API_BASE}/fs/list?path=${encodeURIComponent(currentPath)}`,
      { headers: getAuthHeaders() },
    );

    // 401: 未认证，需要重新登录
    if (response.status === 401) {
      showToast("认证失败，请重新登录", "error");
      logout();
      return;
    }

    // 403: 权限不足
    if (response.status === 403) {
      showToast("权限不足，无法查看文件列表", "error");
      return;
    }

    const result = await response.json();

    if (result.code === 200 && result.data) {
      filesData = result.data.content || [];
      renderFiles(filesData);
      updateBreadcrumb();
    } else if (result.code === 401) {
      showToast("认证失败，请重新登录", "error");
      logout();
    } else if (result.code === 403) {
      showToast("权限不足，无法执行此操作", "error");
    } else {
      fileList.innerHTML =
        '<div class="empty-state"><i class="ti ti-folder-x"></i><p>加载失败</p></div>';
      showToast("加载文件列表失败：" + result.message, "error");
    }
  } catch (error) {
    fileList.innerHTML =
      '<div class="empty-state"><i class="ti ti-wifi-off"></i><p>无法连接到服务器</p></div>';
    showToast("网络错误：" + error.message, "error");
  }
}

// 渲染文件列表
function renderFiles(files) {
  const fileList = document.getElementById("fileList");

  if (!files || files.length === 0) {
    fileList.innerHTML =
      '<div class="empty-state"><i class="ti ti-folder-open"></i><p>此目录为空</p></div>';
    return;
  }

  // 目录排在前面
  const dirs = files.filter((f) => f.file_type === "dir");
  const fileItems = files.filter((f) => f.file_type === "file");
  const sortedFiles = [...dirs, ...fileItems];

  fileList.innerHTML = `
                <div class="file-list-header">
                    <div><input type="checkbox" class="checkbox" onchange="toggleSelectAll(this.checked)"></div>
                    <div>名称</div>
                    <div>大小</div>
                    <div>修改日期</div>
                    <div>操作</div>
                </div>
                ${sortedFiles
                  .map(
                    (file) => `
                    <div class="file-item ${selectedFiles.has(file.path) ? "selected" : ""}"
                         data-path="${escapeHtml(file.path)}"
                         data-type="${file.file_type}"
                         oncontextmenu="showContextMenu(event, '${escapeHtml(file.path)}', '${file.file_type}')">
                        <div>
                            <input type="checkbox" class="checkbox"
                                   ${selectedFiles.has(file.path) ? "checked" : ""}
                                   onchange="toggleSelection('${escapeHtml(file.path)}', this.checked)">
                        </div>
                        <div class="file-main" ondblclick="handleDoubleClick('${escapeHtml(file.path)}', '${file.file_type}')">
                            <div class="file-icon">${file.file_type === "dir" ? '<i class="ti ti-folder"></i>' : getFileIcon(file.name)}</div>
                            <div>
                                <div class="file-name">${escapeHtml(file.name)}</div>
                                <div class="file-meta">${file.file_type === "file" ? formatSize(file.size) : "文件夹"}</div>
                            </div>
                        </div>
                        <div class="file-size">${file.file_type === "file" ? formatSize(file.size) : ""}</div>
                        <div class="file-date">${file.modified ? formatDate(file.modified) : ""}</div>
                        <div class="file-actions">
                            ${
                              file.file_type === "dir"
                                ? `
                                <button class="action-btn" onclick="enterFolder('${escapeHtml(file.path)}')" title="打开">
                                    <i class="ti ti-folder-open"></i>
                                </button>
                            `
                                : `
                                <button class="action-btn" onclick="previewFile('${escapeHtml(file.path)}', '${escapeHtml(file.name)}')" title="预览">
                                    <i class="ti ti-eye"></i>
                                </button>
                                <button class="action-btn" onclick="downloadFile('${escapeHtml(file.path)}')" title="下载">
                                    <i class="ti ti-download"></i>
                                </button>
                            `
                            }
                            <button class="action-btn" onclick="showContextMenuForFile('${escapeHtml(file.path)}', '${file.file_type}')" title="更多">
                                <i class="ti ti-dots"></i>
                            </button>
                        </div>
                    </div>
                `,
                  )
                  .join("")}
            `;
}

// 更新面包屑导航
function updateBreadcrumb() {
  const breadcrumb = document.getElementById("breadcrumb");
  const parts = currentPath.split("/").filter((p) => p);
  let html =
    '<a href="#" onclick="navigateTo(\'/\'); return false;"><i class="ti ti-home"></i></a>';

  let path = "";
  parts.forEach((part) => {
    path += "/" + part;
    html += ` <span class="separator">/</span> <a href="#" onclick="navigateTo('${escapeHtml(path)}'); return false;">${escapeHtml(part)}</a>`;
  });

  breadcrumb.innerHTML = html;
}

// 导航到指定路径
function navigateTo(path) {
  currentPath = path || "/";
  selectedFiles.clear();
  updateDeleteButton();
  loadFiles();
}

// 进入文件夹
function enterFolder(path) {
  navigateTo(path);
}

// 双击处理
function handleDoubleClick(path, type) {
  if (type === "dir") {
    enterFolder(path);
  } else {
    previewFile(path, path.split("/").pop());
  }
}

// 刷新
function refresh() {
  loadFiles();
}

// 搜索
function handleSearch(query) {
  if (!query) {
    renderFiles(filesData);
    return;
  }
  const filtered = filesData.filter((f) =>
    f.name.toLowerCase().includes(query.toLowerCase()),
  );
  renderFiles(filtered);
}

// 全选/取消全选
function toggleSelectAll(checked) {
  if (checked) {
    filesData.forEach((f) => selectedFiles.add(f.path));
  } else {
    selectedFiles.clear();
  }
  renderFiles(filesData);
  updateDeleteButton();
}

// 显示新建文件夹模态框
function showNewFolderModal() {
  document.getElementById("newFolderModal").style.display = "flex";
  document.getElementById("folderNameInput").value = "";
  document.getElementById("folderNameInput").focus();
}

// 隐藏新建文件夹模态框
function hideNewFolderModal() {
  document.getElementById("newFolderModal").style.display = "none";
}

// 创建文件夹
async function createFolder() {
  const name = document.getElementById("folderNameInput").value.trim();
  if (!name) {
    showToast("请输入文件夹名称", "error");
    return;
  }

  const path = currentPath.endsWith("/")
    ? currentPath + name
    : currentPath + "/" + name;

  try {
    const result = await apiRequest("/fs/mkdir", {
      method: "POST",
      body: JSON.stringify({ path }),
    });

    if (result.code === 200) {
      hideNewFolderModal();
      showToast("文件夹创建成功", "success");
      loadFiles();
    } else if (result.code !== 401) {
      showToast("创建失败：" + result.message, "error");
    }
  } catch (error) {
    showToast("网络错误：" + error.message, "error");
  }
}

// 显示重命名模态框
function showRenameModal(path, name) {
  document.getElementById("renameModal").dataset.path = path;
  document.getElementById("renameModal").style.display = "flex";
  document.getElementById("renameInput").value = name;
  document.getElementById("renameInput").focus();
  document.getElementById("renameInput").select();
}

// 隐藏重命名模态框
function hideRenameModal() {
  document.getElementById("renameModal").style.display = "none";
}

// 确认重命名
async function confirmRename() {
  const path = document.getElementById("renameModal").dataset.path;
  const newName = document.getElementById("renameInput").value.trim();

  if (!newName) {
    showToast("请输入新名称", "error");
    return;
  }

  try {
    const result = await apiRequest("/fs/rename", {
      method: "POST",
      body: JSON.stringify({
        src_path: path,
        new_name: newName,
      }),
    });

    if (result.code === 200) {
      hideRenameModal();
      showToast("重命名成功", "success");
      loadFiles();
    } else if (result.code !== 401) {
      showToast("重命名失败：" + result.message, "error");
    }
  } catch (error) {
    showToast("网络错误：" + error.message, "error");
  }
}

// 预览文件
async function previewFile(path, name) {
  const ext = name.split(".").pop().toLowerCase();
  const previewContent = document.getElementById("previewContent");
  previewFilePath = path;

  // 检查是否为可预览的文件类型
  const imageExts = ["jpg", "jpeg", "png", "gif", "svg", "webp", "bmp", "ico"];
  const videoExts = ["mp4", "webm", "ogg", "mov", "avi", "mkv"];
  const audioExts = ["mp3", "wav", "flac", "aac", "ogg"];
  const textExts = [
    "txt",
    "md",
    "log",
    "json",
    "xml",
    "yaml",
    "yml",
    "js",
    "ts",
    "py",
    "java",
    "c",
    "cpp",
    "go",
    "rs",
    "html",
    "css",
    "sh",
  ];
  const docExts = ["pdf"];

  try {
    const response = await fetch(
      `${API_BASE}/fs/get?path=${encodeURIComponent(path)}`,
      { headers: getAuthHeaders() },
    );

    // 401: 未认证，需要重新登录
    if (response.status === 401) {
      showToast("认证失败，请重新登录", "error");
      logout();
      return;
    }

    // 403: 权限不足
    if (response.status === 403) {
      showToast("权限不足，无法预览文件", "error");
      return;
    }

    const result = await response.json();

    if (result.code !== 200 || !result.data) {
      showToast("获取文件失败：" + result.message, "error");
      return;
    }

    const url = result.data.url;
    previewContent.innerHTML = "";

    if (imageExts.includes(ext)) {
      previewContent.innerHTML = `<img src="${url}" alt="${escapeHtml(name)}">`;
    } else if (videoExts.includes(ext)) {
      previewContent.innerHTML = `<video controls src="${url}"></video>`;
    } else if (audioExts.includes(ext)) {
      previewContent.innerHTML = `<audio controls src="${url}"></audio>`;
    } else if (textExts.includes(ext)) {
      // 获取文件内容进行预览
      try {
        const contentResponse = await fetch(url, { headers: getAuthHeaders() });
        const content = await contentResponse.text();
        previewContent.innerHTML = `<pre>${escapeHtml(content.substring(0, 50000))}</pre>`;
      } catch (e) {
        previewContent.innerHTML = `<p>无法加载文本内容</p>`;
      }
    } else if (docExts.includes(ext)) {
      previewContent.innerHTML = `<iframe src="${url}" style="width:100%;height:500px;border:none;"></iframe>`;
    } else {
      previewContent.innerHTML = `<p>此文件类型不支持预览</p><p style="margin-top:8px;color:var(--text-secondary)">您可以下载文件后查看</p>`;
    }

    document.getElementById("previewModal").style.display = "flex";
  } catch (error) {
    showToast("预览失败：" + error.message, "error");
  }
}

// 隐藏预览模态框
function hidePreviewModal() {
  document.getElementById("previewModal").style.display = "none";
  document.getElementById("previewContent").innerHTML = "";
}

// 从预览下载
function downloadFromPreview() {
  if (previewFilePath) {
    downloadFile(previewFilePath);
  }
}

// 下载文件
async function downloadFile(path) {
  try {
    const response = await fetch(
      `${API_BASE}/fs/get?path=${encodeURIComponent(path)}`,
      { headers: getAuthHeaders() },
    );

    // 401: 未认证，需要重新登录
    if (response.status === 401) {
      showToast("认证失败，请重新登录", "error");
      logout();
      return;
    }

    // 403: 权限不足
    if (response.status === 403) {
      showToast("权限不足，无法下载文件", "error");
      return;
    }

    const result = await response.json();

    if (result.code === 200 && result.data) {
      window.open(result.data.url, "_blank");
      showToast("下载已开始", "success");
    } else if (result.code === 401) {
      showToast("认证失败，请重新登录", "error");
      logout();
    } else if (result.code === 403) {
      showToast("权限不足，无法执行此操作", "error");
    } else {
      showToast("获取下载链接失败：" + result.message, "error");
    }
  } catch (error) {
    showToast("网络错误：" + error.message, "error");
  }
}

// 上传文件
async function handleUpload(files) {
  if (files.length === 0) return;

  // 显示上传进度模态框
  showUploadProgressModal();

  for (const file of files) {
    const path = currentPath.endsWith("/")
      ? currentPath + file.name
      : currentPath + "/" + file.name;

    try {
      // 计算文件 SHA256 hash
      const hash = await calculateFileHash(file);

      // 先获取上传信息（POST 方法，参数放在 body 中）
      const uploadInfoResp = await fetch(`${API_BASE}/fs/upload-info`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          ...getAuthHeaders(),
        },
        body: JSON.stringify({
          path: path,
          size: file.size,
          hash: hash,
        }),
      });

      // 401: 未认证，需要重新登录
      if (uploadInfoResp.status === 401) {
        showToast("认证失败，请重新登录", "error");
        logout();
        break;
      }

      // 403: 权限不足
      if (uploadInfoResp.status === 403) {
        showToast("权限不足，无法上传文件", "error");
        break;
      }

      const uploadInfoResult = await uploadInfoResp.json();

      if (uploadInfoResult.code === 401) {
        showToast("认证失败，请重新登录", "error");
        logout();
        break;
      }

      if (uploadInfoResult.code === 403) {
        showToast("权限不足，无法执行此操作", "error");
        break;
      }

      if (uploadInfoResult.code !== 200) {
        showToast(
          `${file.name} 获取上传信息失败：${uploadInfoResult.message}`,
          "error",
        );
        updateUploadProgress(file.name, 0, "获取上传信息失败");
        continue;
      }

      let uploadSuccess = false;

      // 判断上传模式
      if (uploadInfoResult.data && uploadInfoResult.data.mode === "direct") {
        // Direct 模式：先尝试直接上传到存储端
        try {
          uploadSuccess = await uploadToDirect(
            file,
            uploadInfoResult.data,
            path,
            (progress) =>
              updateUploadProgress(file.name, progress, "Direct 模式上传中..."),
          );
        } catch (directError) {
          // Direct 模式抛出异常，捕获后 fallback 到 Relay 模式
          console.warn(`${file.name} Direct 模式异常：`, directError.message);
          uploadSuccess = false;
        }
      }

      // Direct 模式失败或返回 false，fallback 到 Relay 模式
      if (!uploadSuccess) {
        showToast(`${file.name} Direct 模式失败，尝试 Relay 模式...`, "info");
        await uploadToRelay(file, path, hash, (progress) =>
          updateUploadProgress(file.name, progress, "Relay 模式上传中..."),
        );
        // Relay 模式成功后标记为成功
        uploadSuccess = true;
      }

      if (uploadSuccess) {
        updateUploadProgress(file.name, 100, "上传完成");
      }
    } catch (error) {
      showToast(`${file.name} 上传错误：${error.message}`, "error");
      updateUploadProgress(file.name, 0, `错误：${error.message}`);
    }
  }

  loadFiles();
}

// 计算文件 SHA256 hash
async function calculateFileHash(file) {
  const buffer = await file.arrayBuffer();
  const hashBuffer = await crypto.subtle.digest("SHA-256", buffer);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  const hashHex = hashArray
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
  return hashHex;
}

// Direct 模式上传（返回是否成功）
async function uploadToDirect(file, uploadInfo, path, onProgress) {
  const { upload_url, method, form_fields, headers, complete_url } = uploadInfo;

  console.log(`Direct 模式上传：${file.name}, URL: ${upload_url}`);

  // 检查是否是秒传（特殊 URL 标记）
  if (upload_url === "about:blank") {
    showToast(`${file.name} 上传成功（秒传）`, "success");
    if (onProgress) onProgress(100);
    return true;
  }

  try {
    // 判断是否有 form_fields，决定上传方式
    if (form_fields && Object.keys(form_fields).length > 0) {
      // 需要附加表单字段的上传（如 S3 等）
      const formData = new FormData();

      // 添加表单字段
      Object.entries(form_fields).forEach(([key, value]) => {
        formData.append(key, value);
      });

      // 添加文件
      formData.append("file", file);

      const fetchOptions = {
        method: method || "POST",
        body: formData,
      };

      // 添加请求头（如果有）
      if (headers) {
        fetchOptions.headers = headers;
      }

      // 使用 xhr 来跟踪上传进度
      const uploaded = await uploadWithProgress(
        upload_url,
        fetchOptions,
        file.size,
        onProgress,
      );

      if (uploaded) {
        // 上传成功，调用 complete_url（如果有）
        if (complete_url) {
          await callCompleteUrl(complete_url, file, form_fields, path);
        }
        showToast(`${file.name} 上传成功`, "success");
        return true;
      } else {
        throw new Error("上传失败");
      }
    } else {
      // 直接上传文件内容（如 mcloud，使用 PUT 方法）
      // 使用流式上传，确保 Content-Type 正确
      const uploadHeaders = headers ? { ...headers } : {};
      // 确保设置正确的 Content-Type
      if (!uploadHeaders["Content-Type"]) {
        uploadHeaders["Content-Type"] = "application/octet-stream";
      }

      const fetchOptions = {
        method: method || "PUT",
        headers: uploadHeaders,
      };

      console.log(
        `Direct 模式：使用 ${method || "PUT"} 方法上传到 ${upload_url}`,
      );

      // 使用 xhr 来跟踪上传进度
      const uploaded = await uploadWithProgress(
        upload_url,
        fetchOptions,
        file.size,
        onProgress,
        file,
      );

      if (uploaded) {
        // 上传成功，调用 complete_url（如果有）
        if (complete_url) {
          await callCompleteUrl(complete_url, file, form_fields, path);
        }
        showToast(`${file.name} 上传成功`, "success");
        return true;
      } else {
        throw new Error("上传失败");
      }
    }
  } catch (error) {
    // Direct 模式失败，抛出异常让外层捕获并触发 fallback
    console.warn(`${file.name} Direct 上传失败：`, error.message);
    throw error;
  }
}

// 调用 complete_url 完成上传
async function callCompleteUrl(completeUrl, file, formFields, originalPath) {
  try {
    // 解析 complete_url 中的参数获取 upload_id 和 file_id
    const url = new URL(completeUrl, window.location.origin);
    const params = new URLSearchParams(url.search);

    // 从 URL 参数或 form_fields 中获取 file_id 和 upload_id
    // 对于 mcloud，这些信息在 URL 参数中
    const fileId =
      params.get("file_id") ||
      (formFields && formFields.fileId) ||
      params.get("fileId") ||
      "";
    const uploadId =
      params.get("upload_id") ||
      (formFields && formFields.uploadId) ||
      params.get("uploadId") ||
      "";

    // 计算文件 hash
    const contentHash = await calculateFileHash(file);

    // 调用 complete 接口 - 使用完整 URL
    // 使用原始完整路径（包含存储前缀），而不是后端返回的路径
    const completeParams = new URLSearchParams({
      path: originalPath || "", // 使用前端传入的原始路径
      upload_id: uploadId,
      file_id: fileId,
      content_hash: contentHash,
    });

    // 构建完整 URL（包括 origin）
    const fullUrl = `${window.location.origin}${url.pathname}?${completeParams.toString()}`;

    const response = await fetch(fullUrl, {
      method: "POST",
    });

    if (!response.ok) {
      console.warn(`调用 complete 接口失败：`, await response.text());
    } else {
      console.log(`调用 complete 接口成功`);
    }
  } catch (error) {
    console.warn(`调用 complete_url 失败：`, error.message);
  }
}

// Relay 模式上传（通过服务器中转）
async function uploadToRelay(file, path, hash, onProgress) {
  // 使用 xhr 来跟踪上传进度
  const timeoutMs = Math.max(10 * 60 * 1000, file.size * 2); // 至少 10 分钟，或根据文件大小调整

  try {
    const uploaded = await uploadWithProgress(
      `${API_BASE}/fs/upload?path=${encodeURIComponent(path)}&size=${file.size}&hash=${hash}`,
      {
        method: "PUT",
      },
      file.size,
      onProgress,
      file,
      timeoutMs,
    );

    if (!uploaded) {
      throw new Error("上传失败");
    }

    showToast(`${file.name} 上传成功（Relay 模式）`, "success");
    return true;
  } catch (error) {
    console.error(`Relay 模式上传失败:`, error);
    if (
      error.message.includes("Failed to fetch") ||
      error.message.includes("NetworkError")
    ) {
      throw new Error("网络连接失败，请检查服务器是否正常运行");
    }
    throw error;
  }
}

// 使用 XHR 进行带进度的上传
function uploadWithProgress(
  url,
  fetchOptions,
  fileSize,
  onProgress,
  file,
  timeoutMs = 10 * 60 * 1000,
) {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open(fetchOptions.method || "PUT", url, true);

    // 设置请求头
    // 注意：当使用 FormData 时，不要手动设置 Content-Type，让浏览器自动设置 boundary
    const isFormData = fetchOptions.body instanceof FormData;
    if (fetchOptions.headers && !isFormData) {
      Object.entries(fetchOptions.headers).forEach(([key, value]) => {
        xhr.setRequestHeader(key, value);
      });
    }

    // 超时处理
    const timeoutId = setTimeout(() => {
      xhr.abort();
      reject(new Error("上传超时，文件较大，请重试"));
    }, timeoutMs);

    // 上传进度监听
    xhr.upload.addEventListener("progress", (event) => {
      if (event.lengthComputable && onProgress) {
        const percent = Math.round((event.loaded / event.total) * 100);
        onProgress(percent);
      }
    });

    // 完成处理
    xhr.addEventListener("load", () => {
      clearTimeout(timeoutId);
      if (xhr.status >= 200 && xhr.status < 400) {
        if (onProgress) onProgress(100);
        resolve(true);
      } else {
        let errorText = xhr.statusText;
        try {
          const result = JSON.parse(xhr.responseText);
          errorText = result.message || result.error || xhr.statusText;
        } catch (e) {
          errorText = xhr.responseText || xhr.statusText;
        }
        reject(new Error(errorText));
      }
    });

    // 错误处理
    xhr.addEventListener("error", () => {
      clearTimeout(timeoutId);
      reject(new Error("网络错误，请检查连接"));
    });

    // 发送数据
    if (isFormData) {
      // 使用 FormData（用于 S3 等需要表单字段的场景）
      xhr.send(fetchOptions.body);
    } else if (file) {
      // 直接发送文件对象（用于 Relay 模式和 mcloud Direct 模式）
      xhr.send(file);
    } else if (fetchOptions.body) {
      xhr.send(fetchOptions.body);
    } else {
      xhr.send();
    }
  });
}

// 显示上传进度模态框
function showUploadProgressModal() {
  const modal = document.getElementById("uploadProgressModal");
  const content = document.getElementById("uploadProgressContent");
  content.innerHTML = "";
  modal.style.display = "flex";
}

// 隐藏上传进度模态框
function hideUploadProgressModal() {
  const modal = document.getElementById("uploadProgressModal");
  modal.style.display = "none";
}

// 更新上传进度
function updateUploadProgress(fileName, progress, status) {
  const content = document.getElementById("uploadProgressContent");
  let item = document.getElementById(
    `upload-${fileName.replace(/[^a-zA-Z0-9]/g, "-")}`,
  );

  if (!item) {
    item = document.createElement("div");
    item.className = "upload-item";
    item.id = `upload-${fileName.replace(/[^a-zA-Z0-9]/g, "-")}`;
    item.innerHTML = `
                        <div class="upload-item-name">${escapeHtml(fileName)}</div>
                        <div class="upload-item-status">${status || "等待中..."}</div>
                        <div class="upload-item-progress">
                            <div class="progress" style="width: ${progress}%"></div>
                        </div>
                    `;
    content.appendChild(item);
  } else {
    const statusEl = item.querySelector(".upload-item-status");
    const progressEl = item.querySelector(".upload-item-progress .progress");
    if (statusEl) statusEl.textContent = status || "等待中...";
    if (progressEl) progressEl.style.width = `${progress}%`;
  }

  // 自动滚动到底部
  content.scrollTop = content.scrollHeight;
}

// 删除文件
async function deleteFile(path) {
  if (!confirm(`确定要删除 "${path}" 吗？`)) return;

  try {
    const result = await apiRequest("/fs/remove", {
      method: "POST",
      body: JSON.stringify({ path }),
    });

    if (result.code === 200) {
      showToast("删除成功", "success");
      loadFiles();
    } else if (result.code !== 401) {
      showToast("删除失败：" + result.message, "error");
    }
  } catch (error) {
    showToast("网络错误：" + error.message, "error");
  }
}

// 选择文件
function toggleSelection(path, checked) {
  if (checked) {
    selectedFiles.add(path);
  } else {
    selectedFiles.delete(path);
  }
  updateDeleteButton();
}

// 更新删除按钮状态
function updateDeleteButton() {
  const deleteBtn = document.getElementById("deleteBtn");
  deleteBtn.style.display = selectedFiles.size > 0 ? "flex" : "none";
  deleteBtn.innerHTML = `<i class="ti ti-trash"></i> 删除选中 (${selectedFiles.size})`;
}

// 删除选中的文件
async function deleteSelected() {
  if (selectedFiles.size === 0) return;

  if (!confirm(`确定要删除选中的 ${selectedFiles.size} 个项目吗？`)) return;

  const paths = Array.from(selectedFiles);
  let successCount = 0;

  for (const path of paths) {
    try {
      const response = await fetch(`${API_BASE}/fs/remove`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path }),
      });

      // 401: 未认证，需要重新登录
      if (response.status === 401) {
        showToast("认证失败，请重新登录", "error");
        logout();
        return;
      }

      // 403: 权限不足
      if (response.status === 403) {
        showToast("权限不足，无法删除文件", "error");
        return;
      }

      const result = await response.json();
      if (result.code === 200) successCount++;
    } catch (error) {
      console.error("删除失败:", error);
    }
  }

  selectedFiles.clear();
  updateDeleteButton();
  loadFiles();
  showToast(
    `已删除 ${successCount}/${paths.length} 个项目`,
    successCount === paths.length ? "success" : "error",
  );
}

// 设置管理员密钥
function setAdminKey() {
  adminKey = document.getElementById("adminKeyInput").value.trim();
  if (adminKey) {
    localStorage.setItem("rlist_admin_key", adminKey);
    showToast("管理员密钥已保存", "success");
  }
}

// 切换管理面板
function toggleAdminPanel() {
  const panel = document.getElementById("adminPanel");
  panel.style.display = panel.style.display === "none" ? "flex" : "none";
}

// 显示提示
function showToast(message, type = "info") {
  const toast = document.createElement("div");
  toast.className = `toast ${type}`;
  const icon =
    type === "success"
      ? "ti ti-check"
      : type === "error"
        ? "ti ti-alert-circle"
        : "ti ti-info-circle";
  toast.innerHTML = `<i class="${icon}"></i><span>${message}</span>`;
  document.body.appendChild(toast);

  setTimeout(() => {
    toast.remove();
  }, 3000);
}

// 辅助函数
function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

function getFileIcon(name) {
  const ext = name.split(".").pop().toLowerCase();
  const iconMap = {
    pdf: "ti-file-text",
    doc: "ti-file-text",
    docx: "ti-file-text",
    xls: "ti-file-spreadsheet",
    xlsx: "ti-file-spreadsheet",
    ppt: "ti-file-presentation",
    pptx: "ti-file-presentation",
    jpg: "ti-file-image",
    jpeg: "ti-file-image",
    png: "ti-file-image",
    gif: "ti-file-image",
    svg: "ti-file-image",
    webp: "ti-file-image",
    mp3: "ti-file-music",
    wav: "ti-file-music",
    flac: "ti-file-music",
    mp4: "ti-file-video",
    avi: "ti-file-video",
    mkv: "ti-file-video",
    mov: "ti-file-video",
    zip: "ti-file-zip",
    rar: "ti-file-zip",
    "7z": "ti-file-zip",
    tar: "ti-file-zip",
    gz: "ti-file-zip",
    txt: "ti-file-text",
    md: "ti-file-text",
    log: "ti-file-text",
    js: "ti-file-code",
    ts: "ti-file-code",
    py: "ti-file-code",
    java: "ti-file-code",
    cpp: "ti-file-code",
    c: "ti-file-code",
    go: "ti-file-code",
    rs: "ti-file-code",
    html: "ti-file-code",
    css: "ti-file-code",
    json: "ti-file-code",
    xml: "ti-file-code",
    yaml: "ti-file-code",
    yml: "ti-file-code",
  };
  const icon = iconMap[ext] || "ti-file";
  return `<i class="ti ${icon}"></i>`;
}

function formatSize(bytes) {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
}

function formatDate(dateStr) {
  try {
    return new Date(dateStr).toLocaleDateString("zh-CN");
  } catch {
    return dateStr;
  }
}

// 右键菜单
function showContextMenu(event, path, type) {
  event.preventDefault();
  event.stopPropagation();
  contextMenuTarget = { path, type };

  const menu = document.getElementById("contextMenu");
  const actions =
    type === "dir"
      ? `
                <div class="context-menu-item" onclick="enterFolder('${escapeHtml(path)}')">
                    <i class="ti ti-folder-open"></i> 打开
                </div>
                <div class="context-menu-item" onclick="showCopyMoveModal('${escapeHtml(path)}')">
                    <i class="ti ti-copy"></i> 复制/移动
                </div>
            `
      : `
                <div class="context-menu-item" onclick="previewFile('${escapeHtml(path)}', '${escapeHtml(path.split("/").pop())}')">
                    <i class="ti ti-eye"></i> 预览
                </div>
                <div class="context-menu-item" onclick="downloadFile('${escapeHtml(path)}')">
                    <i class="ti ti-download"></i> 下载
                </div>
                <div class="context-menu-item" onclick="showCopyMoveModal('${escapeHtml(path)}')">
                    <i class="ti ti-copy"></i> 复制/移动
                </div>
            `;

  menu.innerHTML = `
                ${actions}
                <div class="context-menu-divider"></div>
                <div class="context-menu-item" onclick="showRenameModal('${escapeHtml(path)}', '${escapeHtml(path.split("/").pop())}')">
                    <i class="ti ti-edit"></i> 重命名
                </div>
                <div class="context-menu-item" onclick="deleteFile('${escapeHtml(path)}')">
                    <i class="ti ti-trash"></i> 删除
                </div>
                <div class="context-menu-divider"></div>
                <div class="context-menu-item" onclick="copyPath('${escapeHtml(path)}')">
                    <i class="ti ti-link"></i> 复制路径
                </div>
            `;

  menu.style.display = "block";
  menu.style.left = event.clientX + "px";
  menu.style.top = event.clientY + "px";
}

function showContextMenuForFile(path, type) {
  const menu = document.getElementById("contextMenu");
  menu.style.display = "none";
  contextMenuTarget = { path, type };

  const rect = event.target.getBoundingClientRect();
  setTimeout(() => {
    menu.style.display = "block";
    menu.style.left = rect.left + "px";
    menu.style.top = rect.bottom + 8 + "px";
  }, 0);
}

function hideContextMenu() {
  document.getElementById("contextMenu").style.display = "none";
}

function copyPath(path) {
  navigator.clipboard
    .writeText(path)
    .then(() => {
      showToast("路径已复制到剪贴板", "success");
    })
    .catch(() => {
      showToast("复制失败", "error");
    });
  hideContextMenu();
}

// 复制/移动模态框
function showCopyMoveModal(path) {
  document.getElementById("copyMoveModal").dataset.path = path;
  document.getElementById("copyMoveModal").style.display = "flex";
  document.getElementById("targetPathInput").value = "";
  document.getElementById("pathSelectorStatus").textContent =
    "点击输入框选择路径";
}

function hideCopyMoveModal() {
  document.getElementById("copyMoveModal").style.display = "none";
}

// 路径选择器
let selectedPath = "";

function showPathSelector() {
  document.getElementById("pathSelectorModal").style.display = "flex";
  loadPathSelector("/");
}

function hidePathSelector() {
  document.getElementById("pathSelectorModal").style.display = "none";
}

async function loadPathSelector(path) {
  const content = document.getElementById("pathSelectorContent");
  content.innerHTML = '<div class="loading"><div class="spinner"></div></div>';

  try {
    const response = await fetch(
      `${API_BASE}/fs/parent-dirs?path=${encodeURIComponent(path)}`,
    );

    // 401: 未认证，需要重新登录
    if (response.status === 401) {
      showToast("认证失败，请重新登录", "error");
      logout();
      return;
    }

    // 403: 权限不足
    if (response.status === 403) {
      showToast("权限不足，无法查看目录", "error");
      return;
    }

    const result = await response.json();

    if (result.code === 200 && result.data) {
      const dirs = result.data;
      if (dirs.length === 0) {
        content.innerHTML =
          '<div class="empty-state"><i class="ti ti-folder-x"></i><p>无可用目录</p></div>';
        return;
      }

      content.innerHTML = dirs
        .map(
          (dir) => `
                            <div class="file-item" onclick="selectPath('${escapeHtml(dir.path)}')" style="cursor: pointer;">
                                <div class="file-main">
                                    <div class="file-icon"><i class="ti ti-folder"></i></div>
                                    <div class="file-name">${escapeHtml(dir.name)}</div>
                                </div>
                            </div>
                        `,
        )
        .join("");

      // 更新当前选择
      selectedPath = path;
      document.getElementById("pathSelectorStatus").textContent =
        `当前路径：${path}`;
    } else {
      content.innerHTML = '<div class="empty-state"><p>加载失败</p></div>';
    }
  } catch (error) {
    content.innerHTML = '<div class="empty-state"><p>加载失败</p></div>';
    showToast("网络错误：" + error.message, "error");
  }
}

function selectPath(path) {
  selectedPath = path;
  document.getElementById("targetPathInput").value = path;
  hidePathSelector();
}

function confirmPathSelection() {
  if (selectedPath) {
    document.getElementById("targetPathInput").value = selectedPath;
  }
  hidePathSelector();
}

async function confirmCopyMove() {
  const path = document.getElementById("copyMoveModal").dataset.path;
  const targetPath = document.getElementById("targetPathInput").value.trim();
  const type = document.getElementById("copyMoveType").value;

  if (!targetPath) {
    showToast("请输入目标路径", "error");
    return;
  }

  const apiEndpoint = type === "copy" ? "/api/fs/copy" : "/api/fs/move";

  try {
    const response = await fetch(apiEndpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        src_path: path,
        dst_path: targetPath,
      }),
    });

    // 401: 未认证，需要重新登录
    if (response.status === 401) {
      showToast("认证失败，请重新登录", "error");
      logout();
      return;
    }

    // 403: 权限不足
    if (response.status === 403) {
      showToast("权限不足，无法执行此操作", "error");
      return;
    }

    const result = await response.json();
    if (result.code === 200) {
      hideCopyMoveModal();
      showToast(`${type === "copy" ? "复制" : "移动"}成功`, "success");
      loadFiles();
    } else {
      showToast(
        `${type === "copy" ? "复制" : "移动"}失败：` + result.message,
        "error",
      );
    }
  } catch (error) {
    showToast("网络错误：" + error.message, "error");
  }
}

// 回车键处理
document.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    if (document.getElementById("newFolderModal").style.display === "flex") {
      createFolder();
    } else if (
      document.getElementById("renameModal").style.display === "flex"
    ) {
      confirmRename();
    } else if (
      document.getElementById("copyMoveModal").style.display === "flex"
    ) {
      confirmCopyMove();
    } else if (
      document.getElementById("pathSelectorModal").style.display === "flex"
    ) {
      confirmPathSelection();
    }
  } else if (e.key === "Escape") {
    hideNewFolderModal();
    hideRenameModal();
    hidePreviewModal();
    hideCopyMoveModal();
    hidePathSelector();
    hideContextMenu();
  }
});
