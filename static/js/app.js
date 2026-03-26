/**
 * 主应用入口
 * 文件管理前端应用
 */

// 全局变量
let fileManager = null;
let uploadManager = null;
let currentView = localStorage.getItem("rlist_view") || "list";
let previewFilePath = "";
let contextMenuTarget = null;
let selectedPathForAction = null;
let isPublicStorageMode = false; // 当前存储模式：false=私有，true=公开
let currentStoragePath = "/"; // 当前存储路径

// 路径历史记录
let pathHistory = [];
let pathHistoryIndex = -1;

// 初始化
document.addEventListener("DOMContentLoaded", () => {
  // 从 URL 参数获取 path 参数
  const urlParams = new URLSearchParams(window.location.search);
  const pathParam = urlParams.get("path");

  // 初始化文件管理器
  fileManager = new FileManager({
    onFilesLoaded: renderFiles,
    onError: (msg) => showToast(msg, "error"),
  });

  // 初始化上传管理器
  uploadManager = new UploadManager({
    onTaskProgress: updateUploadProgress,
    onAllCompleted: onUploadAllCompleted,
  });

  // 初始化主题
  initTheme();

  // 检查登录状态
  checkAuthAndLoad();

  // 绑定搜索事件
  const searchInput = document.getElementById("searchInput");
  if (searchInput) {
    searchInput.addEventListener("input", (e) => {
      handleSearch(e.target.value);
    });
  }

  // 点击关闭上下文菜单
  document.addEventListener("click", (e) => {
    if (!e.target.closest(".context-menu")) {
      hideContextMenu();
    }
  });

  // 监听浏览器前进/后退
  window.addEventListener("popstate", (e) => {
    if (e.state && e.state.path !== undefined) {
      // 从历史记录导航
      if (isPublicStorageMode) {
        currentStoragePath = e.state.path;
      } else {
        fileManager.currentPath = e.state.path;
      }

      // 更新路径历史索引
      const historyIndex = pathHistory.indexOf(e.state.path);
      if (historyIndex !== -1) {
        pathHistoryIndex = historyIndex;
      }

      // 更新 UI
      updateBreadcrumb();
      loadCurrentStorageFiles();
      updateNavButtons();
    }
  });

  // 路径输入框回车确认
  const pathInput = document.getElementById("pathInput");
  if (pathInput) {
    pathInput.addEventListener("keypress", (e) => {
      if (e.key === "Enter") {
        confirmPathInput();
      }
    });
  }

  // 全局错误处理 - 捕获未处理的点击事件异常
  window.addEventListener("error", (e) => {
    console.error("全局错误:", e.error);
    // 不阻止默认行为，但记录错误以便调试
  });

  // 捕获未处理的 Promise rejection
  window.addEventListener("unhandledrejection", (e) => {
    console.error("未处理的 Promise rejection:", e.reason);
    // 不阻止默认行为，但记录错误以便调试
  });
});

/**
 * 初始化主题
 */
function initTheme() {
  const currentTheme = localStorage.getItem("rlist_theme") || "light";
  if (currentTheme === "dark") {
    document.documentElement.setAttribute("data-theme", "dark");
    const themeIcon = document.getElementById("themeIcon");
    if (themeIcon) themeIcon.className = "ti ti-sun";
  }
}

/**
 * 主题切换
 */
function toggleTheme() {
  const isDark = document.documentElement.getAttribute("data-theme") === "dark";
  const themeIcon = document.getElementById("themeIcon");
  if (isDark) {
    document.documentElement.removeAttribute("data-theme");
    if (themeIcon) themeIcon.className = "ti ti-moon";
    localStorage.setItem("rlist_theme", "light");
  } else {
    document.documentElement.setAttribute("data-theme", "dark");
    if (themeIcon) themeIcon.className = "ti ti-sun";
    localStorage.setItem("rlist_theme", "dark");
  }
}

/**
 * 检查认证并加载
 */
async function checkAuthAndLoad() {
  const authToken = localStorage.getItem("rlist_auth_token");
  const currentUser = localStorage.getItem("rlist_current_user");

  if (authToken) {
    const isAuth = await checkAuth();
    if (isAuth) {
      showMainInterface(currentUser);
    } else {
      showLoginInterface();
    }
  } else {
    showLoginInterface();
  }
}

/**
 * 显示主界面
 */
function showMainInterface(username) {
  document.getElementById("authContainer").style.display = "none";
  document.getElementById("mainContainer").style.display = "block";
  document.getElementById("userInfo").style.display = "flex";
  document.getElementById("usernameDisplay").textContent = username;
  // 显示存储切换开关
  const storageSwitch = document.getElementById("storageSwitch");
  if (storageSwitch) {
    storageSwitch.style.display = "flex";
  }

  // 从 URL 参数获取 path 参数
  const urlParams = new URLSearchParams(window.location.search);
  const pathParam = urlParams.get("path");
  if (pathParam) {
    const decodedPath = decodeURIComponent(pathParam);
    if (isPublicStorageMode) {
      currentStoragePath = decodedPath;
    } else {
      fileManager.currentPath = decodedPath;
    }
    // 初始化路径历史
    pathHistory = [decodedPath];
    pathHistoryIndex = 0;
  } else {
    // 默认初始化路径历史
    pathHistory = ["/"];
    pathHistoryIndex = 0;
  }

  setView(currentView);
  loadCurrentStorageFiles();
  updateNavButtons();
}

/**
 * 显示登录界面
 */
function showLoginInterface() {
  document.getElementById("authContainer").style.display = "flex";
  document.getElementById("mainContainer").style.display = "none";
}

// ==================== 认证相关函数 ====================

/**
 * 处理登录
 */
async function handleLogin() {
  try {
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

    const result = await login(username, password);
    if (result.success) {
      authMessage.textContent = "登录成功，正在跳转...";
      authMessage.className = "success";
      setTimeout(() => {
        showMainInterface(username);
      }, 1000);
    } else {
      authMessage.textContent = result.message;
      authMessage.className = "error";
    }
  } catch (error) {
    console.error("登录失败:", error);
    const authMessage = document.getElementById("authMessage");
    authMessage.textContent = "登录失败：" + error.message;
    authMessage.className = "error";
  }
}

// ==================== 公开存储相关函数 ====================

/**
 * 切换存储模式（公开/私有）
 * @param {boolean} isPublic - 是否为公开模式
 */
function toggleStorageMode(isPublic) {
  isPublicStorageMode = isPublic;
  currentStoragePath = "/"; // 切换时重置路径
  loadCurrentStorageFiles();
}

/**
 * 加载公开存储文件列表
 */
async function loadPublicStorageFiles() {
  const fileList = document.getElementById("fileList");
  if (!fileList) return;

  try {
    fileList.innerHTML =
      '<div class="loading"><div class="spinner"></div></div>';

    const response = await fetch("/obs/list", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path: currentStoragePath }),
    });

    const result = await response.json();

    if (result.code === 200 && result.data) {
      const rawItems = result.data.items || [];
      const filesData = rawItems.map((item) => {
        const isDir = item.File === undefined;
        return {
          name: item.File?.name || item.Directory?.name || "unknown",
          path: currentStoragePath.endsWith("/")
            ? currentStoragePath +
              (item.File?.name || item.Directory?.name || "")
            : currentStoragePath +
              "/" +
              (item.File?.name || item.Directory?.name || ""),
          size: item.File?.size || 0,
          file_type: isDir ? "dir" : "file",
          modified: item.File?.modified_at || item.Directory?.modified_at,
        };
      });
      fileManager.filesData = filesData;
      fileManager.selectedFiles.clear();
      renderFiles(filesData);
      updateBreadcrumb();
      updateDeleteButton();
    } else {
      fileList.innerHTML =
        '<div class="empty-state"><i class="ti ti-folder-x"></i><p>加载失败</p></div>';
      showToast("加载文件列表失败：" + result.message, "error");
    }
  } catch (error) {
    console.error("加载公开存储文件列表失败:", error);
    fileList.innerHTML =
      '<div class="empty-state"><i class="ti ti-wifi-off"></i><p>无法连接到服务器</p></div>';
    showToast("网络错误：" + error.message, "error");
  }
}

/**
 * 显示公开存储列表
 */
async function showPublicStorage() {
  try {
    const modal = document.getElementById("publicStorageModal");
    const content = document.getElementById("publicStorageContent");

    modal.style.display = "flex";
    content.innerHTML =
      '<div class="loading"><div class="spinner"></div></div>';

    // 获取公开存储列表（从 /api/admin/storage/list 获取）
    const response = await fetch("/api/admin/storage/list", {
      headers: getAuthHeaders(),
    });

    if (response.status === 403) {
      // 非管理员用户，显示提示
      content.innerHTML = `
        <div class="empty-state">
          <i class="ti ti-lock"></i>
          <p>只有管理员可以查看存储列表</p>
          <p style="margin-top: 8px; font-size: 13px; color: var(--text-secondary);">您可以直接使用顶部的公开/私有存储切换开关</p>
        </div>
      `;
      return;
    }

    if (response.ok) {
      const result = await response.json();
      if (result.code === 200 && result.data) {
        const storages = result.data;
        // 适配新的响应格式 { public: [...], private: [...] }
        const publicStorages = storages.public || [];
        if (publicStorages.length === 0) {
          content.innerHTML = `
            <div class="empty-state">
              <i class="ti ti-folder-open"></i>
              <p>暂无公开存储</p>
            </div>
          `;
        } else {
          content.innerHTML = `
            <div class="storage-list">
              ${publicStorages
                .map(
                  (s) => `
                <div class="storage-item" onclick="openPublicStorage('${escapeHtml(s.path)}', '${escapeHtml(s.name)}')">
                  <i class="ti ti-world"></i>
                  <div class="storage-item-info">
                    <div class="storage-item-name">${escapeHtml(s.name)}</div>
                    <div class="storage-item-path">${escapeHtml(s.path)}</div>
                  </div>
                  <i class="ti ti-chevron-right"></i>
                </div>
              `,
                )
                .join("")}
            </div>
          `;
        }
      } else {
        content.innerHTML = `<div class="empty-state"><i class="ti ti-alert-circle"></i><p>获取公开存储列表失败</p></div>`;
      }
    } else {
      content.innerHTML = `<div class="empty-state"><i class="ti ti-alert-circle"></i><p>获取公开存储列表失败</p></div>`;
    }
  } catch (error) {
    console.error("显示公开存储列表失败:", error);
    const content = document.getElementById("publicStorageContent");
    content.innerHTML = `<div class="empty-state"><i class="ti ti-alert-circle"></i><p>网络错误：${escapeHtml(error.message)}</p></div>`;
  }
}

/**
 * 隐藏公开存储模态框
 */
function hidePublicStorageModal() {
  try {
    document.getElementById("publicStorageModal").style.display = "none";
  } catch (error) {
    console.error("隐藏公开存储模态框失败:", error);
  }
}

/**
 * 打开公开存储（切换到公开模式并加载）
 * @param {string} path - 存储路径
 * @param {string} name - 存储名称
 */
function openPublicStorage(path, name) {
  try {
    // 切换到公开存储模式
    isPublicStorageMode = true;
    currentStoragePath = path;
    // 更新开关状态
    const switchToggle = document.getElementById("storageSwitchToggle");
    if (switchToggle) {
      switchToggle.checked = true;
    }
    hidePublicStorageModal();
    loadCurrentStorageFiles();
    showToast(`已切换到公开存储：${name}`, "success");
  } catch (error) {
    console.error("打开公开存储失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

// ==================== 文件列表相关函数 ====================

/**
 * 导航到指定路径
 * @param {string} path - 路径
 * @param {boolean} addToHistory - 是否添加到历史记录
 */
function navigateTo(path, addToHistory = true) {
  try {
    const targetPath = path || "/";

    if (isPublicStorageMode) {
      currentStoragePath = targetPath;
    } else {
      fileManager.navigateTo(targetPath);
    }

    // 更新 URL 地址栏（使用 history.pushState）
    if (addToHistory) {
      // 添加到路径历史
      pathHistory = pathHistory.slice(0, pathHistoryIndex + 1);
      pathHistory.push(targetPath);
      pathHistoryIndex++;

      // 更新浏览器历史记录
      const newUrl = buildUrlWithPath(targetPath);
      history.pushState({ path: targetPath }, "", newUrl);
    }

    // 更新面包屑和文件列表
    updateBreadcrumb();
    loadCurrentStorageFiles();

    // 更新导航按钮状态
    updateNavButtons();
  } catch (error) {
    console.error("导航失败:", error);
    showToast("导航失败：" + error.message, "error");
  }
}

/**
 * 构建带路径参数的 URL
 * @param {string} path - 路径
 * @returns {string} URL
 */
function buildUrlWithPath(path) {
  const url = new URL(window.location.href);

  // 设置 path 参数
  if (path !== "/") {
    url.searchParams.set("path", encodeURIComponent(path));
  } else {
    url.searchParams.delete("path");
  }

  return url.toString();
}

/**
 * 更新导航按钮状态
 */
function updateNavButtons() {
  const backBtn = document.getElementById("backBtn");
  const forwardBtn = document.getElementById("forwardBtn");

  if (backBtn) {
    backBtn.disabled = pathHistoryIndex <= 0;
  }
  if (forwardBtn) {
    forwardBtn.disabled = pathHistoryIndex >= pathHistory.length - 1;
  }
}

/**
 * 后退
 */
function goBack() {
  try {
    if (pathHistoryIndex > 0) {
      pathHistoryIndex--;
      const path = pathHistory[pathHistoryIndex];
      history.back();
    }
  } catch (error) {
    console.error("后退失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 前进
 */
function goForward() {
  try {
    if (pathHistoryIndex < pathHistory.length - 1) {
      pathHistoryIndex++;
      const path = pathHistory[pathHistoryIndex];
      history.forward();
    }
  } catch (error) {
    console.error("前进失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 切换路径输入框显示
 */
function togglePathInput() {
  try {
    const breadcrumb = document.getElementById("breadcrumb");
    const pathInputWrapper = document.getElementById("pathInputWrapper");
    const pathInput = document.getElementById("pathInput");

    if (breadcrumb.style.display === "none") {
      // 显示面包屑
      breadcrumb.style.display = "flex";
      pathInputWrapper.style.display = "none";
    } else {
      // 显示输入框
      breadcrumb.style.display = "none";
      pathInputWrapper.style.display = "flex";
      const currentPath = isPublicStorageMode
        ? currentStoragePath
        : fileManager?.currentPath || "/";
      pathInput.value = currentPath;
      pathInput.focus();
      pathInput.select();
    }
  } catch (error) {
    console.error("切换路径输入框失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 确认路径输入
 */
function confirmPathInput() {
  try {
    const pathInput = document.getElementById("pathInput");
    const newPath = pathInput.value.trim();

    if (newPath) {
      // 规范化路径
      const normalizedPath = normalizePath(newPath);
      navigateTo(normalizedPath);
    }

    // 切换回面包屑显示
    togglePathInput();
  } catch (error) {
    console.error("确认路径输入失败:", error);
    showToast("路径无效：" + error.message, "error");
  }
}

/**
 * 取消路径输入
 */
function cancelPathInput() {
  try {
    togglePathInput();
  } catch (error) {
    console.error("取消路径输入失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 规范化路径
 * @param {string} path - 路径
 * @returns {string} 规范化后的路径
 */
function normalizePath(path) {
  // 确保路径以 / 开头
  if (!path.startsWith("/")) {
    path = "/" + path;
  }

  // 处理 .. 和 .
  const parts = path.split("/").filter((p) => p && p !== ".");
  const result = [];

  for (const part of parts) {
    if (part === "..") {
      if (result.length > 0) {
        result.pop();
      }
    } else {
      result.push(part);
    }
  }

  return "/" + result.join("/");
}

/**
 * 加载当前存储模式的文件列表
 */
function loadCurrentStorageFiles() {
  try {
    if (isPublicStorageMode) {
      loadPublicStorageFiles();
    } else {
      fileManager.loadFiles(fileManager.currentPath);
    }
  } catch (error) {
    console.error("加载文件列表失败:", error);
    showToast("加载文件列表失败：" + error.message, "error");
  }
}

/**
 * 刷新文件列表
 */
function refresh() {
  try {
    loadCurrentStorageFiles();
    showToast("刷新成功", "success");
  } catch (error) {
    console.error("刷新失败:", error);
    showToast("刷新失败：" + error.message, "error");
  }
}

/**
 * 搜索文件
 * @param {string} query - 搜索词
 */
function handleSearch(query) {
  try {
    if (!fileManager) return;

    if (isPublicStorageMode) {
      // 公开存储模式下使用本地数据搜索
      const filtered = (fileManager.filesData || []).filter((f) =>
        f.name.toLowerCase().includes(query.toLowerCase()),
      );
      renderFiles(filtered);
    } else {
      const filtered = fileManager.search(query);
      renderFiles(filtered);
    }
  } catch (error) {
    console.error("搜索文件失败:", error);
    showToast("搜索失败：" + error.message, "error");
  }
}

/**
 * 更新面包屑
 */
function updateBreadcrumb() {
  try {
    const breadcrumb = document.getElementById("breadcrumb");
    if (!breadcrumb) return;

    const currentPath = isPublicStorageMode
      ? currentStoragePath
      : fileManager?.currentPath || "/";
    const parts = currentPath.split("/").filter((p) => p);
    let html =
      '<a href="#" onclick="navigateTo(\'/\'); return false;" class="breadcrumb-item">' +
      '<i class="ti ti-home"></i>' +
      "<span>首页</span>" +
      "</a>";

    let path = "";
    parts.forEach((part, index) => {
      path += "/" + part;
      const isLast = index === parts.length - 1;

      // 添加分隔符
      html += '<span class="breadcrumb-separator">/</span>';

      // 最后一项（当前路径）添加 active 类
      if (isLast) {
        html += `<a href="#" onclick="navigateTo('${escapeHtml(path)}'); return false;" class="breadcrumb-item active">
        <span>${escapeHtml(part)}</span>
      </a>`;
      } else {
        html += `<a href="#" onclick="navigateTo('${escapeHtml(path)}'); return false;" class="breadcrumb-item">
        <span>${escapeHtml(part)}</span>
      </a>`;
      }
    });

    breadcrumb.innerHTML = html;
  } catch (error) {
    console.error("更新面包屑失败:", error);
    showToast("更新面包屑失败：" + error.message, "error");
  }
}

/**
 * 渲染文件列表
 * @param {Array} files - 文件列表
 */
function renderFiles(files) {
  const fileList = document.getElementById("fileList");
  if (!fileList) return;

  try {
    if (!files || files.length === 0) {
      fileList.innerHTML =
        '<div class="empty-state"><i class="ti ti-folder-open"></i><p>此目录为空</p></div>';
      updateDeleteButton();
      return;
    }

    // 目录排在前面
    const dirs = files.filter((f) => f.file_type === "dir");
    const fileItems = files.filter((f) => f.file_type === "file");
    const sortedFiles = [...dirs, ...fileItems];

    // 公开存储模式下隐藏复选框
    const showCheckbox = !isPublicStorageMode;

    fileList.innerHTML = `
    <div class="file-list-header">
        ${showCheckbox ? '<div><input type="checkbox" class="checkbox" onchange="toggleSelectAll(this.checked)"></div>' : ""}
        <div>名称</div>
        <div>大小</div>
        <div>修改日期</div>
        <div>操作</div>
    </div>
    ${sortedFiles
      .map(
        (file) => `
        <div class="file-item ${fileManager.selectedFiles.has(file.path) ? "selected" : ""}"
             data-path="${escapeHtml(file.path)}"
             data-type="${file.file_type}"
             oncontextmenu="showContextMenu(event, '${escapeHtml(file.path)}', '${file.file_type}')">
            ${
              showCheckbox
                ? `
            <div>
                <input type="checkbox" class="checkbox"
                       ${fileManager.selectedFiles.has(file.path) ? "checked" : ""}
                       onchange="toggleSelection('${escapeHtml(file.path)}', this.checked, event)">
            </div>
            `
                : ""
            }
            <div class="file-main" onclick="handleFileClick('${escapeHtml(file.path)}', '${file.file_type}', event)">
                <div class="file-icon ${file.file_type === "dir" ? "folder" : ""}">${file.file_type === "dir" ? '<i class="ti ti-folder"></i>' : getFileIcon(file.name)}</div>
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
                    <button class="action-btn" onclick="enterFolderWithAnimation('${escapeHtml(file.path)}')" title="打开">
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
    updateDeleteButton();
  } catch (error) {
    console.error("渲染文件列表失败:", error);
    fileList.innerHTML = `
      <div class="empty-state">
        <i class="ti ti-alert-circle"></i>
        <p>渲染文件列表失败</p>
        <p style="margin-top: 8px; color: var(--text-secondary); font-size: 12px;">${escapeHtml(error.message)}</p>
        <button class="btn btn-primary" onclick="loadCurrentStorageFiles()" style="margin-top: 16px;">
          <i class="ti ti-refresh"></i> 重试
        </button>
      </div>
    `;
    showToast("渲染文件列表失败：" + error.message, "error");
  }
}

/**
 * 进入文件夹
 * @param {string} path - 路径
 */
function enterFolder(path) {
  try {
    navigateTo(path);
  } catch (error) {
    console.error("进入文件夹失败:", error);
    showToast("进入文件夹失败：" + error.message, "error");
  }
}

/**
 * 双击处理
 * @param {string} path - 路径
 * @param {string} type - 类型
 */
function handleDoubleClick(path, type) {
  try {
    if (type === "dir") {
      enterFolder(path);
    } else {
      previewFile(path, path.split("/").pop());
    }
  } catch (error) {
    console.error("双击处理失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 单击处理 - 文件夹直接进入，文件选中
 * @param {string} path - 路径
 * @param {string} type - 类型
 * @param {Event} event - 事件对象
 */
function handleFileClick(path, type, event) {
  try {
    // 如果点击了复选框或操作按钮，不触发此处理
    if (
      event.target.closest(".checkbox") ||
      event.target.closest(".file-actions")
    ) {
      return;
    }

    if (type === "dir") {
      // 文件夹：进入并添加动画
      enterFolderWithAnimation(path);
    } else {
      // 文件：切换选中状态
      if (!isPublicStorageMode) {
        const isSelected = fileManager.selectedFiles.has(path);
        toggleSelection(path, !isSelected, event);
      }
    }
  } catch (error) {
    console.error("处理文件点击失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 进入文件夹并添加动画效果
 * @param {string} path - 路径
 */
function enterFolderWithAnimation(path) {
  try {
    const fileList = document.getElementById("fileList");
    if (!fileList) return;

    // 添加进入动画类
    fileList.classList.add("nav-entering");

    // 执行导航（添加到历史记录）
    navigateTo(path);

    // 动画结束后移除类
    setTimeout(() => {
      fileList.classList.remove("nav-entering");
    }, 400);
  } catch (error) {
    console.error("进入文件夹失败:", error);
    showToast("进入文件夹失败：" + error.message, "error");
  }
}

/**
 * 导航到上一级并添加动画效果
 */
function navigateUpWithAnimation() {
  const fileList = document.getElementById("fileList");
  if (!fileList) return;

  // 添加离开动画类
  fileList.classList.add("nav-leave");

  setTimeout(() => {
    navigateTo("..");
    updateBreadcrumb();
    fileList.classList.remove("nav-leave");
  }, 200);
}

// ==================== 选择操作 ====================

/**
 * 切换选择状态
 * @param {string} path - 路径
 * @param {boolean} checked - 是否选中
 * @param {Event} event - 事件对象
 */
function toggleSelection(path, checked, event) {
  try {
    // 阻止事件冒泡，防止触发文件项点击
    if (event) {
      event.stopPropagation();
    }

    if (isPublicStorageMode) {
      showToast("公开存储模式下不支持选择操作", "error");
      return;
    }
    fileManager.toggleSelection(path, checked);
    updateDeleteButton();
    renderFiles(fileManager.filesData);
  } catch (error) {
    console.error("切换选择状态失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 全选/取消全选
 * @param {boolean} checked - 是否全选
 */
function toggleSelectAll(checked) {
  try {
    if (isPublicStorageMode) {
      showToast("公开存储模式下不支持选择操作", "error");
      return;
    }
    fileManager.toggleSelectAll(checked);
    updateDeleteButton();
    renderFiles(fileManager.filesData);
  } catch (error) {
    console.error("全选/取消全选失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 更新删除按钮状态
 */
function updateDeleteButton() {
  const deleteBtn = document.getElementById("deleteBtn");
  if (isPublicStorageMode) {
    // 公开存储模式下隐藏删除按钮
    deleteBtn.style.display = "none";
    return;
  }
  const count = fileManager.getSelectedCount();
  deleteBtn.style.display = count > 0 ? "flex" : "none";
  if (count > 0) {
    deleteBtn.innerHTML = `<i class="ti ti-trash"></i> 删除选中 (${count})`;
  }
}

/**
 * 删除选中的文件
 */
async function deleteSelected() {
  try {
    if (isPublicStorageMode) {
      showToast("公开存储模式下不支持删除操作", "error");
      return;
    }
    const count = fileManager.getSelectedCount();
    if (count === 0) return;

    // 显示删除确认弹窗
    showDeleteConfirmModal(count);
  } catch (error) {
    console.error("删除选中的文件失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 显示删除确认弹窗
 * @param {number} count - 删除数量
 */
function showDeleteConfirmModal(count) {
  try {
    const modal = document.getElementById("deleteConfirmModal");
    const message = document.getElementById("deleteMessage");
    message.textContent = `确定要删除选中的 ${count} 个项目吗？`;
    modal.style.display = "flex";
  } catch (error) {
    console.error("显示删除确认弹窗失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 隐藏删除确认弹窗
 */
function hideDeleteConfirmModal() {
  try {
    document.getElementById("deleteConfirmModal").style.display = "none";
  } catch (error) {
    console.error("隐藏删除确认弹窗失败:", error);
  }
}

/**
 * 处理删除弹窗背景点击
 */
function handleDeleteModalOverlayClick(event) {
  try {
    if (event.target === event.currentTarget) {
      hideDeleteConfirmModal();
    }
  } catch (error) {
    console.error("处理删除弹窗背景点击失败:", error);
  }
}

/**
 * 确认删除
 */
async function confirmDelete() {
  hideDeleteConfirmModal();

  try {
    // 检查是否有回调函数
    if (window.deleteCallback) {
      const callback = window.deleteCallback;
      window.deleteCallback = null;
      await callback();
    } else {
      // 默认的批量删除逻辑
      const result = await fileManager.deleteSelected();
      showToast(
        `已删除 ${result.count}/${result.total} 个项目`,
        result.count === result.total ? "success" : "error",
      );
    }
  } catch (error) {
    console.error("删除失败:", error);
    showToast("删除失败：" + error.message, "error");
  }
}

// ==================== 文件夹操作 ====================

/**
 * 显示新建文件夹模态框
 */
function showNewFolderModal() {
  try {
    if (isPublicStorageMode) {
      showToast("公开存储模式下不支持创建文件夹", "error");
      return;
    }
    document.getElementById("newFolderModal").style.display = "flex";
    document.getElementById("folderNameInput").value = "";
    document.getElementById("folderNameInput").focus();
  } catch (error) {
    console.error("显示新建文件夹模态框失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 隐藏新建文件夹模态框
 */
function hideNewFolderModal() {
  document.getElementById("newFolderModal").style.display = "none";
}

/**
 * 显示重命名模态框
 * @param {string} path - 路径
 * @param {string} name - 名称
 */
function showRenameModal(path, name) {
  try {
    selectedPathForAction = path;
    document.getElementById("renameModal").style.display = "flex";
    document.getElementById("renameInput").value = name;
    document.getElementById("renameInput").focus();
    document.getElementById("renameInput").select();
  } catch (error) {
    console.error("显示重命名模态框失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 隐藏重命名模态框
 */
function hideRenameModal() {
  try {
    document.getElementById("renameModal").style.display = "none";
  } catch (error) {
    console.error("隐藏重命名模态框失败:", error);
  }
}

/**
 * 确认重命名
 */
async function confirmRename() {
  try {
    const newName = document.getElementById("renameInput").value.trim();
    if (!newName) {
      showToast("请输入新名称", "error");
      return;
    }

    const result = await fileManager.rename(selectedPathForAction, newName);
    if (result.success) {
      hideRenameModal();
      showToast("重命名成功", "success");
    } else {
      showToast("重命名失败：" + result.message, "error");
    }
  } catch (error) {
    console.error("重命名失败:", error);
    showToast("重命名失败：" + error.message, "error");
  }
}

// ==================== 预览和下载 ====================

// 预览相关全局变量
let previewZoom = 1;
let previewRotation = 0;
let previewPdfDoc = null;
let previewFileType = "";
let previewFileName = "";
let previewXhr = null; // 当前预览下载的 XHR 对象

/**
 * 预览文件
 * @param {string} path - 路径
 * @param {string} name - 文件名
 */
async function previewFile(path, name) {
  try {
    const ext = name.split(".").pop().toLowerCase();
    const previewContent = document.getElementById("previewContent");
    const previewFileNameEl = document.getElementById("previewFileName");
    previewFilePath = path;
    previewZoom = 1;
    previewRotation = 0;
    previewPdfDoc = null;
    previewFileName = name;

    // 更新文件名显示
    if (previewFileNameEl) {
      previewFileNameEl.textContent = name;
    }

    // 重置缩放显示
    const zoomLevelEl = document.getElementById("zoomLevel");
    if (zoomLevelEl) {
      zoomLevelEl.textContent = "100%";
    }

    const imageExts = [
      "jpg",
      "jpeg",
      "png",
      "gif",
      "svg",
      "webp",
      "bmp",
      "ico",
    ];
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

    // 公开模式使用公开下载端点
    let url;
    if (isPublicStorageMode) {
      url = `${window.location.origin}/obs/download?path=${encodeURIComponent(path)}`;
    } else {
      const result = await fileManager.getPreviewUrl(path);
      if (!result.success) {
        showToast("获取文件失败：" + result.message, "error");
        return;
      }
      url = result.url;
    }

    // 显示加载进度条
    previewContent.innerHTML = `
    <div class="preview-loading">
      <div class="spinner"></div>
      <p class="loading-text">正在加载文件...</p>
      <div class="progress-bar"><div class="progress" style="width: 0%"></div></div>
      <p class="loading-percent">0%</p>
    </div>`;

    if (imageExts.includes(ext)) {
      previewFileType = "image";
      await loadImageWithProgress(url, previewContent, name);
    } else if (videoExts.includes(ext)) {
      previewFileType = "video";
      await loadVideoWithProgress(url, previewContent);
    } else if (audioExts.includes(ext)) {
      previewFileType = "audio";
      await loadAudioWithProgress(url, previewContent);
    } else if (textExts.includes(ext)) {
      previewFileType = "text";
      await loadTextWithProgress(url, previewContent);
    } else if (docExts.includes(ext)) {
      previewFileType = "pdf";
      await renderPdfPreview(url, previewContent);
    } else {
      previewFileType = "other";
      previewContent.innerHTML = `<div class="preview-placeholder"><i class="ti ti-file"></i><p>此文件类型不支持预览</p><p style="margin-top:8px;color:var(--text-secondary)">您可以下载文件后查看</p></div>`;
    }

    document.getElementById("previewModal").style.display = "flex";
  } catch (error) {
    console.error("预览文件失败:", error);
    const previewContent = document.getElementById("previewContent");
    previewContent.innerHTML = `<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>预览失败</p><p style="margin-top:8px;color:var(--text-secondary)">${escapeHtml(error.message)}</p></div>`;
    showToast("预览失败：" + error.message, "error");
  }
}

/**
 * 使用进度条加载图片
 * @param {string} url - 图片 URL
 * @param {HTMLElement} container - 容器元素
 * @param {string} alt - 图片描述
 */
function loadImageWithProgress(url, container, alt) {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    previewXhr = xhr;

    xhr.open("GET", url, true);
    xhr.responseType = "blob";

    xhr.setRequestHeader(
      "AUTH-JWT-TOKEN",
      localStorage.getItem("rlist_auth_token") || "",
    );

    xhr.onprogress = function (e) {
      if (e.lengthComputable) {
        const percent = Math.round((e.loaded / e.total) * 100);
        const progressBar = container.querySelector(".progress");
        const percentText = container.querySelector(".loading-percent");
        const loadingText = container.querySelector(".loading-text");
        if (progressBar) progressBar.style.width = `${percent}%`;
        if (percentText) percentText.textContent = `${percent}%`;
        if (loadingText) loadingText.textContent = `正在加载图片...`;
      }
    };

    xhr.onload = function () {
      if (xhr.status === 200) {
        const blob = xhr.response;
        const objectUrl = URL.createObjectURL(blob);
        container.innerHTML = `<img src="${objectUrl}" alt="${escapeHtml(alt)}" onload="onImageLoad()" onerror="onPreviewError()">`;
        resolve();
      } else {
        container.innerHTML = `<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>加载失败</p></div>`;
        reject(new Error("加载失败"));
      }
      previewXhr = null;
    };

    xhr.onerror = function () {
      container.innerHTML = `<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>网络错误</p></div>`;
      reject(new Error("网络错误"));
      previewXhr = null;
    };

    xhr.send();
  });
}

/**
 * 使用进度条加载视频
 * @param {string} url - 视频 URL
 * @param {HTMLElement} container - 容器元素
 */
function loadVideoWithProgress(url, container) {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    previewXhr = xhr;

    xhr.open("GET", url, true);
    xhr.responseType = "blob";

    xhr.setRequestHeader(
      "AUTH-JWT-TOKEN",
      localStorage.getItem("rlist_auth_token") || "",
    );

    xhr.onprogress = function (e) {
      if (e.lengthComputable) {
        const percent = Math.round((e.loaded / e.total) * 100);
        const progressBar = container.querySelector(".progress");
        const percentText = container.querySelector(".loading-percent");
        const loadingText = container.querySelector(".loading-text");
        if (progressBar) progressBar.style.width = `${percent}%`;
        if (percentText) percentText.textContent = `${percent}%`;
        if (loadingText) loadingText.textContent = `正在加载视频...`;
      }
    };

    xhr.onload = function () {
      if (xhr.status === 200) {
        const blob = xhr.response;
        const objectUrl = URL.createObjectURL(blob);
        container.innerHTML = `<video controls src="${objectUrl}" onloadedmetadata="onMediaLoad()"></video>`;
        resolve();
      } else {
        container.innerHTML = `<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>加载失败</p></div>`;
        reject(new Error("加载失败"));
      }
      previewXhr = null;
    };

    xhr.onerror = function () {
      container.innerHTML = `<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>网络错误</p></div>`;
      reject(new Error("网络错误"));
      previewXhr = null;
    };

    xhr.send();
  });
}

/**
 * 使用进度条加载音频
 * @param {string} url - 音频 URL
 * @param {HTMLElement} container - 容器元素
 */
function loadAudioWithProgress(url, container) {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    previewXhr = xhr;

    xhr.open("GET", url, true);
    xhr.responseType = "blob";

    xhr.setRequestHeader(
      "AUTH-JWT-TOKEN",
      localStorage.getItem("rlist_auth_token") || "",
    );

    xhr.onprogress = function (e) {
      if (e.lengthComputable) {
        const percent = Math.round((e.loaded / e.total) * 100);
        const progressBar = container.querySelector(".progress");
        const percentText = container.querySelector(".loading-percent");
        const loadingText = container.querySelector(".loading-text");
        if (progressBar) progressBar.style.width = `${percent}%`;
        if (percentText) percentText.textContent = `${percent}%`;
        if (loadingText) loadingText.textContent = `正在加载音频...`;
      }
    };

    xhr.onload = function () {
      if (xhr.status === 200) {
        const blob = xhr.response;
        const objectUrl = URL.createObjectURL(blob);
        container.innerHTML = `<audio controls src="${objectUrl}"></audio>`;
        resolve();
      } else {
        container.innerHTML = `<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>加载失败</p></div>`;
        reject(new Error("加载失败"));
      }
      previewXhr = null;
    };

    xhr.onerror = function () {
      container.innerHTML = `<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>网络错误</p></div>`;
      reject(new Error("网络错误"));
      previewXhr = null;
    };

    xhr.send();
  });
}

/**
 * 使用进度条加载文本
 * @param {string} url - 文本 URL
 * @param {HTMLElement} container - 容器元素
 */
async function loadTextWithProgress(url, container) {
  try {
    const xhr = new XMLHttpRequest();
    previewXhr = xhr;

    xhr.open("GET", url, true);

    xhr.setRequestHeader(
      "AUTH-JWT-TOKEN",
      localStorage.getItem("rlist_auth_token") || "",
    );

    xhr.onprogress = function (e) {
      if (e.lengthComputable) {
        const percent = Math.round((e.loaded / e.total) * 100);
        const progressBar = container.querySelector(".progress");
        const percentText = container.querySelector(".loading-percent");
        const loadingText = container.querySelector(".loading-text");
        if (progressBar) progressBar.style.width = `${percent}%`;
        if (percentText) percentText.textContent = `${percent}%`;
        if (loadingText) loadingText.textContent = `正在加载文本...`;
      } else {
        const loadingText = container.querySelector(".loading-text");
        if (loadingText) loadingText.textContent = `正在加载文本...`;
      }
    };

    xhr.onload = function () {
      if (xhr.status === 200) {
        const content = xhr.responseText;
        container.innerHTML = `<pre>${escapeHtml(content.substring(0, 100000))}</pre>`;
      } else {
        container.innerHTML = `<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>加载失败</p></div>`;
      }
      previewXhr = null;
    };

    xhr.onerror = function () {
      container.innerHTML = `<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>网络错误</p></div>`;
      previewXhr = null;
    };

    xhr.send();
  } catch (e) {
    container.innerHTML = `<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>无法加载文本内容</p></div>`;
  }
}

/**
 * 图片加载完成
 */
function onImageLoad() {
  // 可以在这里调整初始缩放
}

/**
 * 媒体加载完成
 */
function onMediaLoad() {
  // 媒体文件加载完成回调
}

/**
 * 预览错误处理
 */
function onPreviewError() {
  const previewContent = document.getElementById("previewContent");
  previewContent.innerHTML = `<div class="preview-placeholder">
    <i class="ti ti-alert-circle"></i>
    <p>加载失败</p>
  </div>`;
  showToast("预览加载失败", "error");
}

/**
 * 放大
 */
function zoomIn() {
  if (previewFileType === "pdf") {
    previewZoom += 0.25;
    updateZoomDisplay();
    if (previewPdfDoc) {
      rerenderPdf();
    }
  } else if (previewFileType === "image") {
    previewZoom += 0.25;
    updateZoomDisplay();
    const img = document.querySelector(".preview-content img");
    if (img) {
      img.style.transform = `scale(${previewZoom})`;
    }
  }
}

/**
 * 缩小
 */
function zoomOut() {
  if (previewZoom > 0.5) {
    if (previewFileType === "pdf") {
      previewZoom -= 0.25;
      updateZoomDisplay();
      if (previewPdfDoc) {
        rerenderPdf();
      }
    } else if (previewFileType === "image") {
      previewZoom -= 0.25;
      updateZoomDisplay();
      const img = document.querySelector(".preview-content img");
      if (img) {
        img.style.transform = `scale(${previewZoom})`;
      }
    }
  }
}

/**
 * 旋转预览
 */
function rotatePreview() {
  previewRotation = (previewRotation + 90) % 360;
  if (previewFileType === "pdf" && previewPdfDoc) {
    rerenderPdf();
  } else if (previewFileType === "image") {
    const img = document.querySelector(".preview-content img");
    if (img) {
      img.style.transform = `rotate(${previewRotation}deg) scale(${previewZoom})`;
    }
  }
}

/**
 * 更新缩放显示
 */
function updateZoomDisplay() {
  const zoomLevelEl = document.getElementById("zoomLevel");
  if (zoomLevelEl) {
    zoomLevelEl.textContent = `${Math.round(previewZoom * 100)}%`;
  }
}

/**
 * 重新渲染 PDF
 */
async function rerenderPdf() {
  if (!previewPdfDoc) return;

  const container = document.getElementById("pdfPageContainer");
  if (!container) return;

  container.innerHTML =
    '<div class="preview-placeholder"><i class="ti ti-loader"></i><p>渲染中...</p></div>';

  for (let pageNum = 1; pageNum <= previewPdfDoc.numPages; pageNum++) {
    const pageDiv = document.createElement("div");
    pageDiv.className = "pdf-page";
    pageDiv.style.marginBottom = "16px";

    const canvas = document.createElement("canvas");
    canvas.className = "pdf-canvas";
    canvas.id = `pdf-page-${pageNum}`;
    pageDiv.appendChild(canvas);
    container.appendChild(pageDiv);

    const page = await previewPdfDoc.getPage(pageNum);
    const viewport = page.getViewport({
      scale: previewZoom,
      rotation: previewRotation,
    });

    canvas.height = viewport.height;
    canvas.width = viewport.width;

    await page.render({
      canvasContext: canvas.getContext("2d"),
      viewport: viewport,
    }).promise;
  }
}

/**
 * 渲染 PDF 预览
 * @param {string} url - PDF URL
 * @param {HTMLElement} container - 容器元素
 */
async function renderPdfPreview(url, container) {
  // 检查 PDF.js 是否已加载
  if (typeof pdfjsLib === "undefined") {
    // PDF.js 还在加载中，显示加载进度
    container.innerHTML = `
      <div class="pdf-loading">
        <div class="spinner"></div>
        <p class="loading-text">正在加载 PDF 组件...</p>
        <div class="progress-bar"><div class="progress" style="width: 30%"></div></div>
      </div>`;

    // 等待 PDF.js 加载
    await waitForPdfJs();

    if (typeof pdfjsLib === "undefined") {
      container.innerHTML = `<div class="preview-placeholder">
        <i class="ti ti-alert-circle"></i>
        <p>PDF 预览功能加载失败</p>
        <p style="margin-top:8px;color:var(--text-secondary)">请检查网络连接</p>
      </div>`;
      return;
    }
  }

  // 显示加载进度
  container.innerHTML = `
    <div class="pdf-loading">
      <div class="spinner"></div>
      <p class="loading-text">正在加载 PDF 文件...</p>
      <div class="progress-bar"><div class="progress" style="width: 0%"></div></div>
      <p class="loading-percent">0%</p>
    </div>`;

  try {
    const loadingTask = pdfjsLib.getDocument({
      url: url,
      // 配置加载参数
      cMapUrl: "https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/cmaps/",
      cMapPacked: true,
    });

    // 监听加载进度
    loadingTask.onProgress = function (progress) {
      const percent = Math.round((progress.loaded / progress.total) * 100);
      const progressBar = container.querySelector(".progress");
      const percentText = container.querySelector(".loading-percent");
      if (progressBar) progressBar.style.width = `${percent}%`;
      if (percentText) percentText.textContent = `${percent}%`;
    };

    previewPdfDoc = await loadingTask.promise;

    container.innerHTML =
      '<div class="pdf-page-container" id="pdfPageContainer"></div>';
    const pageContainer = document.getElementById("pdfPageContainer");

    // 渲染所有页面
    for (let pageNum = 1; pageNum <= previewPdfDoc.numPages; pageNum++) {
      const pageDiv = document.createElement("div");
      pageDiv.className = "pdf-page";
      pageDiv.style.marginBottom = "16px";

      const canvas = document.createElement("canvas");
      canvas.className = "pdf-canvas";
      canvas.id = `pdf-page-${pageNum}`;
      pageDiv.appendChild(canvas);
      pageContainer.appendChild(pageDiv);

      const page = await previewPdfDoc.getPage(pageNum);
      const viewport = page.getViewport({
        scale: previewZoom,
        rotation: previewRotation,
      });

      canvas.height = viewport.height;
      canvas.width = viewport.width;

      await page.render({
        canvasContext: canvas.getContext("2d"),
        viewport: viewport,
      }).promise;
    }

    // 更新页面计数
    const info = document.createElement("div");
    info.style.color = "var(--text-secondary)";
    info.style.fontSize = "12px";
    info.style.marginTop = "8px";
    info.textContent = `共 ${previewPdfDoc.numPages} 页`;
    pageContainer.appendChild(info);
  } catch (error) {
    console.error("PDF 渲染失败:", error);
    container.innerHTML = `<div class="preview-placeholder">
      <i class="ti ti-alert-circle"></i>
      <p>PDF 加载失败</p>
      <p style="margin-top:8px;color:var(--text-secondary)">${error.message}</p>
    </div>`;
  }
}

/**
 * 等待 PDF.js 加载完成
 * @returns {Promise<void>}
 */
function waitForPdfJs() {
  return new Promise((resolve) => {
    let attempts = 0;
    const maxAttempts = 50; // 最多等待 5 秒

    const checkInterval = setInterval(() => {
      if (typeof pdfjsLib !== "undefined") {
        clearInterval(checkInterval);
        resolve();
      } else {
        attempts++;
        if (attempts >= maxAttempts) {
          clearInterval(checkInterval);
          resolve(); // 超时也返回
        }
      }
    }, 100);
  });
}

/**
 * 隐藏预览模态框
 */
function hidePreviewModal() {
  try {
    // 取消正在进行的下载
    if (previewXhr) {
      previewXhr.abort();
      previewXhr = null;
    }

    document.getElementById("previewModal").style.display = "none";
    document.getElementById("previewContent").innerHTML = "";
    previewPdfDoc = null;
    previewZoom = 1;
    previewRotation = 0;
  } catch (error) {
    console.error("隐藏预览模态框失败:", error);
  }
}

/**
 * 从预览下载
 */
function downloadFromPreview() {
  if (previewFilePath) {
    downloadFile(previewFilePath);
  }
}

/**
 * 下载文件
 * @param {string} path - 路径
 */
async function downloadFile(path) {
  try {
    if (isPublicStorageMode) {
      // 公开模式使用公开下载端点
      const url = `${window.location.origin}/obs/download?path=${encodeURIComponent(path)}`;
      window.open(url, "_blank");
      showToast("下载已开始", "success");
    } else {
      await fileManager.downloadFile(path);
    }
  } catch (error) {
    console.error("下载文件失败:", error);
    showToast("下载失败：" + error.message, "error");
  }
}

// ==================== 上传 ====================

/**
 * 处理文件上传
 * @param {FileList} files - 文件列表
 */
async function handleUploadFiles(files) {
  try {
    if (!files || files.length === 0) return;

    if (isPublicStorageMode) {
      showToast("公开存储模式下不支持上传文件", "error");
      return;
    }

    uploadManager.setCurrentPath(fileManager.currentPath);

    for (const file of files) {
      uploadManager.addFile(file);
    }

    showUploadProgressModal();
    await uploadManager.uploadAll();
  } catch (error) {
    console.error("处理文件上传失败:", error);
    showToast("上传失败：" + error.message, "error");
  }
}

/**
 * 更新上传进度显示
 * @param {UploadTask} task - 上传任务
 */
function updateUploadProgress(task) {
  const content = document.getElementById("uploadProgressContent");
  const fileId = `upload-${task.file.name.replace(/[^a-zA-Z0-9]/g, "-")}`;

  let item = document.getElementById(fileId);
  if (!item) {
    item = document.createElement("div");
    item.className = "upload-item";
    item.id = fileId;
    item.innerHTML = `
      <div class="upload-item-name">${escapeHtml(task.file.name)}</div>
      <div class="upload-item-status">${task.message || "等待中..."}</div>
      <div class="upload-item-progress">
        <div class="progress" style="width: ${task.progress}%"></div>
      </div>
    `;
    content.appendChild(item);
  } else {
    const statusEl = item.querySelector(".upload-item-status");
    const progressEl = item.querySelector(".upload-item-progress .progress");
    if (statusEl) statusEl.textContent = task.message || "等待中...";
    if (progressEl) progressEl.style.width = `${task.progress}%`;
  }

  content.scrollTop = content.scrollHeight;
}

/**
 * 所有上传完成
 * @param {Array} tasks - 任务列表
 */
function onUploadAllCompleted(tasks) {
  const success = tasks.filter((t) => t.status === "completed").length;
  const failed = tasks.filter((t) => t.status === "error").length;
  showToast(
    `上传完成：成功 ${success}, 失败 ${failed}`,
    success > 0 ? "success" : "error",
  );
  fileManager.refresh();
}

/**
 * 显示上传进度模态框
 */
function showUploadProgressModal() {
  try {
    const modal = document.getElementById("uploadProgressModal");
    const content = document.getElementById("uploadProgressContent");
    content.innerHTML = "";
    modal.style.display = "flex";
  } catch (error) {
    console.error("显示上传进度模态框失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 隐藏上传进度模态框
 */
function hideUploadProgressModal() {
  try {
    const modal = document.getElementById("uploadProgressModal");
    modal.style.display = "none";
  } catch (error) {
    console.error("隐藏上传进度模态框失败:", error);
  }
}

// ==================== 复制/移动 ====================

/**
 * 显示复制/移动模态框
 * @param {string} path - 路径
 */
function showCopyMoveModal(path) {
  try {
    selectedPathForAction = path;
    document.getElementById("copyMoveModal").dataset.path = path;
    document.getElementById("copyMoveModal").style.display = "flex";
    document.getElementById("targetPathInput").value = "";
    document.getElementById("pathSelectorStatus").textContent =
      "点击输入框选择路径";
  } catch (error) {
    console.error("显示复制/移动模态框失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 隐藏复制/移动模态框
 */
function hideCopyMoveModal() {
  document.getElementById("copyMoveModal").style.display = "none";
}

/**
 * 显示路径选择器
 */
function showPathSelector() {
  try {
    document.getElementById("pathSelectorModal").style.display = "flex";
    loadPathSelector("/");
  } catch (error) {
    console.error("显示路径选择器失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 隐藏路径选择器
 */
function hidePathSelector() {
  try {
    document.getElementById("pathSelectorModal").style.display = "none";
  } catch (error) {
    console.error("隐藏路径选择器失败:", error);
  }
}

/**
 * 处理模态框背景点击
 */
function handleModalOverlayClick(event) {
  try {
    if (event.target === event.currentTarget) {
      hidePathSelector();
    }
  } catch (error) {
    console.error("处理模态框背景点击失败:", error);
  }
}

/**
 * 加载路径选择器
 * @param {string} path - 路径
 */
async function loadPathSelector(path) {
  const content = document.getElementById("pathSelectorContent");
  console.log("加载路径选择器，路径:", path);
  content.innerHTML = '<div class="loading"><div class="spinner"></div></div>';

  try {
    const response = await fetch(
      `${fileManager.apiBase}/fs/list?path=${encodeURIComponent(path)}`,
      { headers: fileManager.getAuthHeaders() },
    );

    if (response.status === 401) {
      showToast("认证失败，请重新登录", "error");
      logout();
      return;
    }

    if (response.status === 403) {
      showToast("权限不足，无法查看目录", "error");
      return;
    }

    const result = await response.json();
    console.log("路径选择器 API 返回:", result);

    if (result.code === 200 && result.data) {
      const rawItems = result.data.items || [];
      console.log("原始项目:", rawItems);

      // 使用 Directory 属性判断是否为目录
      const dirs = rawItems
        .filter((item) => item.Directory !== undefined)
        .map((item) => ({
          name: item.Directory.name || "unknown",
          path: path.endsWith("/")
            ? path + (item.Directory.name || "")
            : path + "/" + (item.Directory.name || ""),
        }));

      console.log("目录列表:", dirs);

      const currentDirItem = {
        name: path === "/" ? "根目录" : path.split("/").pop() || "当前目录",
        path: path,
      };

      // 构建返回上级目录的项（如果不是根目录）
      let parentDirHtml = "";
      if (path !== "/") {
        const parentPath = path.substring(0, path.lastIndexOf("/")) || "/";
        parentDirHtml = `
          <div class="file-item" data-action="parent" data-path="${escapeHtml(parentPath)}">
              <div class="file-main">
                  <div class="file-icon"><i class="ti ti-arrow-up"></i></div>
                  <div class="file-name">.. (返回上级)</div>
              </div>
          </div>
        `;
      }

      // 构建目录列表 HTML（添加选择按钮）
      let dirsHtml = "";
      if (dirs.length === 0) {
        dirsHtml = `
          <div class="empty-state" style="padding: 20px; text-align: center; color: var(--text-secondary);">
              <i class="ti ti-folder-off" style="font-size: 32px;"></i>
              <p style="margin-top: 8px;">此目录为空</p>
          </div>
        `;
      } else {
        dirsHtml = dirs
          .map(
            (dir) => `
              <div class="file-item" data-action="dir" data-path="${escapeHtml(dir.path)}">
                  <div class="file-main" style="flex: 1; cursor: pointer;">
                      <div class="file-icon"><i class="ti ti-folder"></i></div>
                      <div class="file-name">${escapeHtml(dir.name)}</div>
                  </div>
                  <button class="action-btn-sm" onclick="event.stopPropagation(); selectPath('${escapeHtml(dir.path)}')" style="margin-right: 8px;" title="选择此文件夹">
                      <i class="ti ti-check"></i> 选择
                  </button>
              </div>
            `,
          )
          .join("");
      }

      content.innerHTML = `
        ${parentDirHtml}
        <div class="file-item" data-action="current" data-path="${escapeHtml(currentDirItem.path)}" style="cursor: pointer;">
            <div class="file-main">
                <div class="file-icon"><i class="ti ti-check"></i></div>
                <div class="file-name">${escapeHtml(currentDirItem.name)} (当前)</div>
            </div>
        </div>
        ${dirsHtml}
      `;

      // 绑定点击事件
      const fileItems = content.querySelectorAll(".file-item");
      console.log("绑定的文件项数量:", fileItems.length);

      fileItems.forEach((item, index) => {
        const action = item.getAttribute("data-action");
        const targetPath = item.getAttribute("data-path");
        console.log(`项目 ${index}:`, { action, targetPath });

        item.addEventListener("click", (e) => {
          e.preventDefault();
          e.stopPropagation();

          const itemAction = item.getAttribute("data-action");
          const itemPath = item.getAttribute("data-path");

          console.log("点击路径选择器项目:", itemAction, itemPath);

          if (itemAction === "parent" && itemPath) {
            // 返回上级目录
            console.log("返回上级目录:", itemPath);
            loadPathSelector(itemPath);
          } else if (itemAction === "dir" && itemPath) {
            // 进入子目录 - 同时更新目标路径输入框（添加末尾 / 以便后续自动添加文件名）
            console.log("进入子目录:", itemPath);
            const normalizedPath = itemPath.endsWith("/")
              ? itemPath
              : itemPath + "/";
            document.getElementById("targetPathInput").value = normalizedPath;
            loadPathSelector(itemPath);
          } else if (itemAction === "current" && itemPath) {
            // 选择当前目录
            console.log("选择当前目录:", itemPath);
            selectPath(itemPath);
          }
        });
      });

      document.getElementById("pathSelectorStatus").textContent =
        `当前路径：${path}`;
    } else {
      content.innerHTML = '<div class="empty-state"><p>加载失败</p></div>';
    }
  } catch (error) {
    console.error("加载路径选择器失败:", error);
    content.innerHTML = '<div class="empty-state"><p>加载失败</p></div>';
    showToast("网络错误：" + error.message, "error");
  }
}

/**
 * 选择路径
 * @param {string} path - 路径
 */
function selectPath(path) {
  try {
    // 为目录路径添加末尾的 /，以便后续自动添加文件名
    const normalizedPath = path.endsWith("/") ? path : path + "/";
    document.getElementById("targetPathInput").value = normalizedPath;
    hidePathSelector();
  } catch (error) {
    console.error("选择路径失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 确认路径选择
 */
function confirmPathSelection() {
  try {
    // 使用输入框中的当前值（已经在点击文件夹时更新）
    let path = document.getElementById("targetPathInput").value;
    if (path) {
      // 为目录路径添加末尾的 /，以便后续自动添加文件名
      if (!path.endsWith("/")) {
        path = path + "/";
        document.getElementById("targetPathInput").value = path;
      }
      hidePathSelector();
    } else {
      showToast("请先选择路径", "error");
    }
  } catch (error) {
    console.error("确认路径选择失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 确认复制/移动
 */
async function confirmCopyMove() {
  try {
    let targetPath = document.getElementById("targetPathInput").value.trim();
    const type = document.getElementById("copyMoveType").value;

    if (!targetPath) {
      showToast("请输入目标路径", "error");
      return;
    }

    // 如果目标路径是目录（以/结尾或是根目录），则自动添加源文件/文件夹名
    const srcName =
      selectedPathForAction.split("/").pop() || selectedPathForAction;
    if (targetPath === "/" || targetPath.endsWith("/")) {
      // 规范化路径：确保只有一个斜杠分隔
      const baseDir =
        targetPath === "/"
          ? ""
          : targetPath.endsWith("/")
            ? targetPath.slice(0, -1)
            : targetPath;
      targetPath = baseDir + "/" + srcName;
    }

    const result = await fileManager.copyOrMove(
      selectedPathForAction,
      targetPath,
      type,
    );
    if (result.success) {
      hideCopyMoveModal();
      showToast(`${type === "copy" ? "复制" : "移动"}成功`, "success");
    } else {
      showToast(
        `${type === "copy" ? "复制" : "移动"}失败：` + result.message,
        "error",
      );
    }
  } catch (error) {
    console.error("复制/移动失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

// ==================== 右键菜单 ====================

/**
 * 显示右键菜单
 * @param {Event} event - 事件
 * @param {string} path - 路径
 * @param {string} type - 类型
 */
function showContextMenu(event, path, type) {
  try {
    event.preventDefault();
    event.stopPropagation();
    contextMenuTarget = { path, type };

    const menu = document.getElementById("contextMenu");
    const actions =
      type === "dir"
        ? `
        <div class="context-menu-item" onclick="hideContextMenu(); enterFolder('${escapeHtml(path)}')">
            <i class="ti ti-folder-open"></i> 打开
        </div>
        <div class="context-menu-item" onclick="hideContextMenu(); showCopyMoveModal('${escapeHtml(path)}')">
            <i class="ti ti-copy"></i> 复制/移动
        </div>
      `
        : `
        <div class="context-menu-item" onclick="hideContextMenu(); previewFile('${escapeHtml(path)}', '${escapeHtml(path.split("/").pop())}')">
            <i class="ti ti-eye"></i> 预览
        </div>
        <div class="context-menu-item" onclick="hideContextMenu(); downloadFile('${escapeHtml(path)}')">
            <i class="ti ti-download"></i> 下载
        </div>
        <div class="context-menu-item" onclick="hideContextMenu(); showCopyMoveModal('${escapeHtml(path)}')">
            <i class="ti ti-copy"></i> 复制/移动
        </div>
        <div class="context-menu-item" onclick="hideContextMenu(); copyShareUrl('${escapeHtml(path)}')">
            <i class="ti ti-link"></i> 复制分享链接
        </div>
      `;

    menu.innerHTML = `
    ${actions}
    <div class="context-menu-divider"></div>
    <div class="context-menu-item" onclick="hideContextMenu(); showRenameModal('${escapeHtml(path)}', '${escapeHtml(path.split("/").pop())}')">
        <i class="ti ti-edit"></i> 重命名
    </div>
    <div class="context-menu-item" onclick="hideContextMenu(); deleteFile('${escapeHtml(path)}')">
        <i class="ti ti-trash"></i> 删除
    </div>
    <div class="context-menu-divider"></div>
    <div class="context-menu-item" onclick="hideContextMenu(); copyPath('${escapeHtml(path)}')">
        <i class="ti ti-link"></i> 复制路径
    </div>
  `;

    menu.style.display = "block";
    menu.style.left = event.clientX + "px";
    menu.style.top = event.clientY + "px";
  } catch (error) {
    console.error("显示右键菜单失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 为文件显示右键菜单
 * @param {string} path - 路径
 * @param {string} type - 类型
 */
function showContextMenuForFile(path, type) {
  try {
    const menu = document.getElementById("contextMenu");
    menu.style.display = "none";
    contextMenuTarget = { path, type };

    const rect = event.target.getBoundingClientRect();
    setTimeout(() => {
      menu.style.display = "block";
      menu.style.left = rect.left + "px";
      menu.style.top = rect.bottom + 8 + "px";
    }, 0);
  } catch (error) {
    console.error("显示右键菜单失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 隐藏右键菜单
 */
function hideContextMenu() {
  try {
    document.getElementById("contextMenu").style.display = "none";
  } catch (error) {
    console.error("隐藏右键菜单失败:", error);
  }
}

/**
 * 复制路径
 * @param {string} path - 路径
 */
async function copyPath(path) {
  try {
    await fileManager.copyPath(path);
    hideContextMenu();
  } catch (error) {
    console.error("复制路径失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 复制分享链接
 * @param {string} path - 路径
 */
async function copyShareUrl(path) {
  try {
    if (isPublicStorageMode) {
      // 公开模式使用公开下载端点
      const url = `${window.location.origin}/obs/download?path=${encodeURIComponent(path)}`;
      const success = await copyToClipboard(url);
      if (success) {
        showToast("分享链接已复制到剪贴板", "success");
      } else {
        showToast("复制失败", "error");
      }
    } else {
      await fileManager.copyShareUrl(path);
    }
    hideContextMenu();
  } catch (error) {
    console.error("复制分享链接失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 删除文件
 * @param {string} path - 路径
 */
async function deleteFile(path) {
  try {
    // 使用自定义删除确认弹窗
    showDeleteConfirmModalWithCallback(path, async () => {
      try {
        const result = await fileManager.remove(path);
        if (result.success) {
          showToast("删除成功", "success");
        } else {
          showToast("删除失败：" + result.message, "error");
        }
        hideContextMenu();
      } catch (error) {
        console.error("删除文件失败:", error);
        showToast("删除失败：" + error.message, "error");
      }
    });
  } catch (error) {
    console.error("删除文件失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 显示删除确认弹窗（带回调）
 * @param {string} path - 文件路径（用于显示）
 * @param {Function} callback - 确认后的回调函数
 */
function showDeleteConfirmModalWithCallback(path, callback) {
  try {
    const modal = document.getElementById("deleteConfirmModal");
    const message = document.getElementById("deleteMessage");
    const fileName = path.split("/").pop();
    message.textContent = `确定要删除 "${fileName}" 吗？`;

    // 保存回调函数
    window.deleteCallback = callback;

    modal.style.display = "flex";
  } catch (error) {
    console.error("显示删除确认弹窗失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

// ==================== 其他功能 ====================

/**
 * 打开管理后台（独立页面）
 */
function openAdminPanel() {
  try {
    // 先检查认证状态
    const authToken = localStorage.getItem("rlist_auth_token");
    if (!authToken) {
      showToast("请先登录", "error");
      return;
    }

    // 使用 location.href 直接跳转，避免被浏览器拦截
    window.location.href = "/admin.html";
  } catch (error) {
    console.error("打开管理后台失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

// ==================== 键盘事件 ====================

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
