/**
 * 公开访问页面 JavaScript
 * 提供无需登录的文件浏览和下载功能
 */

// 全局变量
const API_BASE = "/obs";
let currentPath = "/";
let filesData = [];
let currentView = localStorage.getItem("rlist_public_view") || "list";
let previewFilePath = "";
let contextMenuTarget = null;

// 预览相关全局变量
let previewZoom = 1;
let previewRotation = 0;
let previewPdfDoc = null;
let previewFileType = "";

// 路径历史记录
let pathHistory = [];
let pathHistoryIndex = -1;
let isNavigatingHistory = false;

// 分页相关
let currentCursor = null; // 开始偏移量
let hasMorePages = false;
let isLoadingMore = false;
const PAGE_SIZE = 20;

// 初始化
document.addEventListener("DOMContentLoaded", () => {
  // 从 URL 参数获取 storage 和 path 参数
  const urlParams = new URLSearchParams(window.location.search);
  const storageParam = urlParams.get("storage");
  const pathParam = urlParams.get("path");

  // 如果有 storage 参数，更新当前路径并显示存储徽章
  if (storageParam) {
    currentPath = storageParam;
    // 显示存储徽章
    const storageBadge = document.getElementById("storageBadge");
    const storageBadgeName = document.getElementById("storageBadgeName");
    if (storageBadge && storageBadgeName) {
      storageBadge.style.display = "inline-flex";
      storageBadgeName.textContent = storageParam;
    }
  }

  // 如果有 path 参数，使用 path 参数
  if (pathParam) {
    currentPath = decodeURIComponent(pathParam);
  }

  // 初始化路径历史
  pathHistory = [currentPath];

  // 全局错误处理 - 捕获未处理的异常
  window.addEventListener("error", (e) => {
    console.error("全局错误:", e.error);
  });

  // 捕获未处理的 Promise rejection
  window.addEventListener("unhandledrejection", (e) => {
    console.error("未处理的 Promise rejection:", e.reason);
  });

  pathHistoryIndex = 0;

  // 初始化主题
  initTheme();

  // 加载文件列表
  loadFiles();

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
      currentPath = e.state.path;

      // 更新路径历史索引
      const historyIndex = pathHistory.indexOf(currentPath);
      if (historyIndex !== -1) {
        pathHistoryIndex = historyIndex;
      }

      // 更新 UI
      updateBreadcrumb();
      loadFiles(currentPath);
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
 * 视图切换
 * @param {string} view - 视图类型
 */
function setView(view) {
  currentView = view;
  localStorage.setItem("rlist_view", view);
  const fileList = document.getElementById("fileList");
  const listBtn = document.getElementById("listViewBtn");
  const gridBtn = document.getElementById("gridViewBtn");

  if (view === "grid") {
    fileList?.classList.add("grid-view");
    listBtn?.classList.remove("active");
    gridBtn?.classList.add("active");
  } else {
    fileList?.classList.remove("grid-view");
    listBtn?.classList.add("active");
    gridBtn?.classList.remove("active");
  }
}

/**
 * 加载文件列表（支持分页）
 * @param {string} path - 路径
 * @param {boolean} reset - 是否重置分页
 */
async function loadFiles(path = currentPath, reset = true) {
  currentPath = path;
  const fileList = document.getElementById("fileList");
  if (!fileList) return;

  // 重置分页状态
  if (reset) {
    currentCursor = null;
    hasMorePages = false;
    filesData = [];
    fileList.innerHTML =
      '<div class="loading"><div class="spinner"></div></div>';
  } else {
    // 加载更多时显示加载状态
    if (isLoadingMore) return;
    isLoadingMore = true;

    // 在列表末尾添加加载提示
    const loadingIndicator = fileList.querySelector(".loading-more");
    if (loadingIndicator) {
      loadingIndicator.style.display = "flex";
    } else {
      const indicator = document.createElement("div");
      indicator.className = "loading-more";
      indicator.innerHTML =
        '<div class="spinner spinner-small"></div><span>加载中...</span>';
      fileList.appendChild(indicator);
    }
  }

  try {
    const requestBody = {
      path: currentPath,
      per_page: PAGE_SIZE,
    };

    // 添加游标参数
    if (currentCursor !== null) {
      requestBody.cursor = currentCursor;
    }

    // 添加 Challenge 验证
    const requestWithChallenge = await buildPublicRequest(requestBody);

    const response = await fetch(`${API_BASE}/list`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(requestWithChallenge),
    });

    const result = await response.json();

    if (result.code === 200 && result.data) {
      const rawItems = result.data.items || [];
      const newItems = rawItems.map((item) => {
        const isDir = item.File === undefined;
        return {
          name: item.File?.name || item.Directory?.name || "unknown",
          path: currentPath.endsWith("/")
            ? currentPath + (item.File?.name || item.Directory?.name || "")
            : currentPath +
              "/" +
              (item.File?.name || item.Directory?.name || ""),
          size: item.File?.size || 0,
          file_type: isDir ? "dir" : "file",
          modified: item.File?.modified_at || item.Directory?.modified_at,
        };
      });

      // 更新分页状态
      currentCursor = result.data.next_cursor;
      hasMorePages = currentCursor !== null && currentCursor !== undefined;

      if (reset) {
        filesData = newItems;
        renderFiles(filesData);
      } else {
        filesData = [...filesData, ...newItems];
        renderFiles(filesData, false);
      }

      // 移除加载指示器
      const loadingIndicator = fileList.querySelector(".loading-more");
      if (loadingIndicator) {
        loadingIndicator.remove();
      }

      updateBreadcrumb();
      isLoadingMore = false;
    } else {
      if (reset) {
        fileList.innerHTML =
          '<div class="empty-state"><i class="ti ti-folder-x"></i><p>加载失败</p></div>';
      }
      showToast("加载文件列表失败：" + result.message, "error");
    }
  } catch (error) {
    if (reset) {
      fileList.innerHTML =
        '<div class="empty-state"><i class="ti ti-wifi-off"></i><p>无法连接到服务器</p></div>';
    }
    showToast("网络错误：" + error.message, "error");
    isLoadingMore = false;
  }
}

/**
 * 加载更多文件
 */
async function loadMoreFiles() {
  if (!hasMorePages || isLoadingMore) return;
  await loadFiles(currentPath, false);
}

/**
 * 渲染文件列表
 * @param {Array} files - 文件列表
 * @param {boolean} fullRender - 是否完全重新渲染
 */
function renderFiles(files, fullRender = true) {
  const fileList = document.getElementById("fileList");
  if (!fileList) return;

  if (!files || files.length === 0) {
    fileList.innerHTML =
      '<div class="empty-state"><i class="ti ti-folder-open"></i><p>此目录为空</p></div>';
    return;
  }

  // 目录排在前面
  const dirs = files.filter((f) => f.file_type === "dir");
  const fileItems = files.filter((f) => f.file_type === "file");
  const sortedFiles = [...dirs, ...fileItems];

  if (fullRender) {
    // 完全重新渲染
    fileList.innerHTML = `
      <div class="file-list-header">
          <div>名称</div>
          <div>大小</div>
          <div>修改日期</div>
          <div>操作</div>
      </div>
      ${sortedFiles
        .map(
          (file) => `
          <div class="file-item"
               data-path="${escapeHtml(file.path)}"
               data-type="${file.file_type}"
               oncontextmenu="showContextMenu(event, '${escapeHtml(file.path)}', '${file.file_type}')">
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

    // 添加滚动监听器
    setupScrollListener();
  } else {
    // 增量渲染 - 只追加新项
    const itemsHtml = sortedFiles
      .map(
        (file) => `
        <div class="file-item"
             data-path="${escapeHtml(file.path)}"
             data-type="${file.file_type}"
             oncontextmenu="showContextMenu(event, '${escapeHtml(file.path)}', '${file.file_type}')">
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
      .join("");

    // 追加到列表末尾（移除加载指示器后再追加）
    const loadingIndicator = fileList.querySelector(".loading-more");
    if (loadingIndicator) {
      loadingIndicator.insertAdjacentHTML("beforebegin", itemsHtml);
    } else {
      fileList.insertAdjacentHTML("beforeend", itemsHtml);
    }
  }
}

/**
 * 设置滚动监听器以加载更多文件
 */
function setupScrollListener() {
  const container = document.querySelector(".file-list-container");
  if (!container) return;

  // 移除旧的监听器
  container.removeEventListener("scroll", handleScroll);

  // 添加新的监听器
  container.addEventListener("scroll", handleScroll);
}

/**
 * 滚动处理函数
 */
function handleScroll() {
  const container = document.querySelector(".file-list-container");
  if (!container) return;

  const { scrollTop, scrollHeight, clientHeight } = container;

  // 当滚动到距离底部 100px 时加载更多
  if (scrollTop + clientHeight >= scrollHeight - 100) {
    loadMoreFiles();
  }
}

/**
 * 双击处理
 * @param {string} path - 路径
 * @param {string} type - 类型
 */
function handleDoubleClick(path, type) {
  if (type === "dir") {
    enterFolder(path);
  } else {
    previewFile(path, path.split("/").pop());
  }
}

/**
 * 单击处理 - 文件夹直接进入，文件选中
 * @param {string} path - 路径
 * @param {string} type - 类型
 * @param {Event} event - 事件对象
 */
function handleFileClick(path, type, event) {
  // 如果点击了操作按钮，不触发此处理
  if (event.target.closest(".file-actions")) {
    return;
  }

  if (type === "dir") {
    // 文件夹：进入并添加动画
    enterFolderWithAnimation(path);
  }
}

/**
 * 进入文件夹并添加动画效果
 * @param {string} path - 路径
 */
function enterFolderWithAnimation(path) {
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
}

/**
 * 更新面包屑导航
 */
function updateBreadcrumb() {
  const breadcrumb = document.getElementById("breadcrumb");
  if (!breadcrumb) return;

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
}

/**
 * 导航到指定路径
 * @param {string} path - 路径
 * @param {boolean} addToHistory - 是否添加到历史记录
 */
function navigateTo(path, addToHistory = true) {
  currentPath = path || "/";

  // 更新 URL 地址栏（使用 history.pushState）
  if (addToHistory) {
    // 添加到路径历史
    pathHistory = pathHistory.slice(0, pathHistoryIndex + 1);
    pathHistory.push(currentPath);
    pathHistoryIndex++;

    // 更新浏览器历史记录
    const newUrl = buildUrlWithPath(currentPath);
    history.pushState({ path: currentPath }, "", newUrl);
  }

  // 更新面包屑和文件列表
  updateBreadcrumb();
  loadFiles(currentPath);

  // 更新导航按钮状态
  updateNavButtons();
}

/**
 * 构建带路径参数的 URL
 * @param {string} path - 路径
 * @returns {string} URL
 */
function buildUrlWithPath(path) {
  const url = new URL(window.location.href);
  const storageParam = document.getElementById("storageBadgeName")?.textContent;

  // 如果有 storage，保留 storage 参数
  if (storageParam) {
    url.searchParams.set("storage", storageParam);
  }

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
  if (pathHistoryIndex > 0) {
    pathHistoryIndex--;
    const path = pathHistory[pathHistoryIndex];
    currentPath = path;
    history.back();
  }
}

/**
 * 前进
 */
function goForward() {
  if (pathHistoryIndex < pathHistory.length - 1) {
    pathHistoryIndex++;
    const path = pathHistory[pathHistoryIndex];
    currentPath = path;
    history.forward();
  }
}

/**
 * 切换路径输入框显示
 */
function togglePathInput() {
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
    pathInput.value = currentPath;
    pathInput.focus();
    pathInput.select();
  }
}

/**
 * 确认路径输入
 */
function confirmPathInput() {
  const pathInput = document.getElementById("pathInput");
  const newPath = pathInput.value.trim();

  if (newPath) {
    // 规范化路径
    const normalizedPath = normalizePath(newPath);
    navigateTo(normalizedPath);
  }

  // 切换回面包屑显示
  togglePathInput();
}

/**
 * 取消路径输入
 */
function cancelPathInput() {
  togglePathInput();
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
 * 进入文件夹
 * @param {string} path - 路径
 */
function enterFolder(path) {
  navigateTo(path);
}

/**
 * 刷新文件列表
 */
function refresh() {
  loadFiles(currentPath);
}

/**
 * 搜索文件
 * @param {string} query - 搜索词
 */
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

/**
 * 预览文件
 * @param {string} path - 路径
 * @param {string} name - 文件名
 */
async function previewFile(path, name) {
  const ext = name.split(".").pop().toLowerCase();
  const previewContent = document.getElementById("previewContent");
  const previewFileNameEl = document.getElementById("previewFileName");
  previewFilePath = path;
  previewZoom = 1;
  previewRotation = 0;
  previewPdfDoc = null;

  // 更新文件名显示
  if (previewFileNameEl) {
    previewFileNameEl.textContent = name;
  }

  // 重置缩放显示
  const zoomLevelEl = document.getElementById("zoomLevel");
  if (zoomLevelEl) {
    zoomLevelEl.textContent = "100%";
  }

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

  // 获取预览 URL - 先通过 Challenge 验证获取下载链接
  previewContent.innerHTML =
    '<div class="preview-placeholder"><i class="ti ti-loader"></i><p>加载中...</p></div>';

  // 使用 buildPublicRequest 获取带 challenge 的下载链接
  let downloadUrl;
  try {
    const requestBody = { path };
    const requestWithChallenge = await buildPublicRequest(requestBody);
    const response = await fetch(`${API_BASE}/download`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(requestWithChallenge),
    });
    const result = await response.json();
    if (result.code === 200 && result.data && result.data.download_url) {
      downloadUrl = result.data.download_url;
    } else {
      // Fallback: 使用中继模式
      downloadUrl = `${API_BASE}/fs/download?path=${encodeURIComponent(path)}`;
    }
  } catch (e) {
    console.error("获取下载链接失败:", e);
    downloadUrl = `${API_BASE}/fs/download?path=${encodeURIComponent(path)}`;
  }

  const fullUrl = `${window.location.origin}${downloadUrl}`;

  if (imageExts.includes(ext)) {
    previewFileType = "image";
    previewContent.innerHTML = `<img src="${fullUrl}" alt="${escapeHtml(name)}" onload="onImageLoad()" onerror="onPreviewError()">`;
  } else if (videoExts.includes(ext)) {
    previewFileType = "video";
    previewContent.innerHTML = `<video controls src="${fullUrl}"></video>`;
  } else if (audioExts.includes(ext)) {
    previewFileType = "audio";
    previewContent.innerHTML = `<audio controls src="${fullUrl}"></audio>`;
  } else if (textExts.includes(ext)) {
    previewFileType = "text";
    try {
      const contentResponse = await fetch(fullUrl);
      const content = await contentResponse.text();
      previewContent.innerHTML = `<pre>${escapeHtml(content.substring(0, 100000))}</pre>`;
    } catch (e) {
      previewContent.innerHTML = `<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>无法加载文本内容</p></div>`;
    }
  } else if (docExts.includes(ext)) {
    previewFileType = "pdf";
    await renderPdfPreview(fullUrl, previewContent);
  } else {
    previewFileType = "other";
    previewContent.innerHTML = `<div class="preview-placeholder"><i class="ti ti-file"></i><p>此文件类型不支持预览</p><p style="margin-top:8px;color:var(--text-secondary)">您可以下载文件后查看</p></div>`;
  }

  document.getElementById("previewModal").style.display = "flex";
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
 * 图片加载完成
 */
function onImageLoad() {
  // 可以在这里调整初始缩放
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
 * 隐藏预览模态框
 */
function hidePreviewModal() {
  document.getElementById("previewModal").style.display = "none";
  document.getElementById("previewContent").innerHTML = "";
  previewPdfDoc = null;
  previewZoom = 1;
  previewRotation = 0;
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
    // 先尝试获取直链 - 需要 Challenge 验证
    const requestBody = { path };
    const requestWithChallenge = await buildPublicRequest(requestBody);

    const response = await fetch(`${API_BASE}/download`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(requestWithChallenge),
    });
    const result = await response.json();

    let downloadUrl;
    if (result.code === 200 && result.data && result.data.download_url) {
      // 使用存储驱动提供的直链
      downloadUrl = result.data.download_url;
    } else {
      showToast("获取下载链接失败：" + result.message, "error");
      return;
    }

    const url = `${window.location.origin}${downloadUrl}`;
    window.open(url, "_blank");
    showToast("下载已开始", "success");
  } catch (error) {
    console.error("下载失败:", error);
    showToast("下载失败：" + error.message, "error");
  }
}

/**
 * 复制分享链接（直链）
 * @param {string} path - 路径
 */
async function copyShareUrl(path) {
  try {
    // 先获取解析后的直链
    const response = await fetch(
      `${API_BASE}/fs/get?path=${encodeURIComponent(path)}`,
    );
    const result = await response.json();

    let url;
    if (result.code === 200 && result.data && result.data.url) {
      // 使用解析后的直链
      url = result.data.url;
      // 如果是相对路径，转换为绝对路径
      if (!url.startsWith("http")) {
        url = `${window.location.origin}${url}`;
      }
    } else {
      // 降级方案：使用中继模式
      url = `${window.location.origin}${API_BASE}/fs/download?path=${encodeURIComponent(path)}`;
    }

    const success = await copyToClipboard(url);
    if (success) {
      showToast("分享链接已复制到剪贴板", "success");
    } else {
      showToast("复制失败", "error");
    }
  } catch (error) {
    showToast("获取链接失败：" + error.message, "error");
  }
  hideContextMenu();
}

/**
 * 显示右键菜单
 * @param {Event} event - 事件
 * @param {string} path - 路径
 * @param {string} type - 类型
 */
function showContextMenu(event, path, type) {
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
      `
      : `
        <div class="context-menu-item" onclick="hideContextMenu(); previewFile('${escapeHtml(path)}', '${escapeHtml(path.split("/").pop())}')">
            <i class="ti ti-eye"></i> 预览
        </div>
        <div class="context-menu-item" onclick="hideContextMenu(); downloadFile('${escapeHtml(path)}')">
            <i class="ti ti-download"></i> 下载
        </div>
        <div class="context-menu-item" onclick="hideContextMenu(); copyShareUrl('${escapeHtml(path)}')">
            <i class="ti ti-link"></i> 复制分享链接
        </div>
      `;

  menu.innerHTML = `
    ${actions}
    <div class="context-menu-divider"></div>
    <div class="context-menu-item" onclick="hideContextMenu(); copyPath('${escapeHtml(path)}')">
        <i class="ti ti-link"></i> 复制路径
    </div>
  `;

  menu.style.display = "block";
  menu.style.left = event.clientX + "px";
  menu.style.top = event.clientY + "px";
}

/**
 * 为文件显示右键菜单
 * @param {string} path - 路径
 * @param {string} type - 类型
 */
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

/**
 * 隐藏右键菜单
 */
function hideContextMenu() {
  document.getElementById("contextMenu").style.display = "none";
}

/**
 * 复制路径
 * @param {string} path - 路径
 */
async function copyPath(path) {
  const success = await copyToClipboard(path);
  if (success) {
    showToast("路径已复制到剪贴板", "success");
  } else {
    showToast("复制失败", "error");
  }
  hideContextMenu();
}

// ==================== 键盘事件 ====================

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    hidePreviewModal();
    hideContextMenu();
  }
});
