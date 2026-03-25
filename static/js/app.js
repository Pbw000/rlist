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

// 初始化
document.addEventListener("DOMContentLoaded", () => {
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
  setView(currentView);
  fileManager.loadFiles("/");
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
}

// ==================== 文件列表相关函数 ====================

/**
 * 导航到指定路径
 * @param {string} path - 路径
 */
function navigateTo(path) {
  fileManager.navigateTo(path);
  updateBreadcrumb();
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
  fileManager.refresh();
}

/**
 * 搜索文件
 * @param {string} query - 搜索词
 */
function handleSearch(query) {
  if (!fileManager) return;

  const filtered = fileManager.search(query);
  renderFiles(filtered);
}

/**
 * 更新面包屑
 */
function updateBreadcrumb() {
  const breadcrumb = document.getElementById("breadcrumb");
  const currentPath = fileManager?.currentPath || "/";
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

/**
 * 渲染文件列表
 * @param {Array} files - 文件列表
 */
function renderFiles(files) {
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
        <div class="file-item ${fileManager.selectedFiles.has(file.path) ? "selected" : ""}"
             data-path="${escapeHtml(file.path)}"
             data-type="${file.file_type}"
             oncontextmenu="showContextMenu(event, '${escapeHtml(file.path)}', '${file.file_type}')">
            <div>
                <input type="checkbox" class="checkbox"
                       ${fileManager.selectedFiles.has(file.path) ? "checked" : ""}
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

// ==================== 选择操作 ====================

/**
 * 切换选择状态
 * @param {string} path - 路径
 * @param {boolean} checked - 是否选中
 */
function toggleSelection(path, checked) {
  fileManager.toggleSelection(path, checked);
  updateDeleteButton();
  renderFiles(fileManager.filesData);
}

/**
 * 全选/取消全选
 * @param {boolean} checked - 是否全选
 */
function toggleSelectAll(checked) {
  fileManager.toggleSelectAll(checked);
  updateDeleteButton();
  renderFiles(fileManager.filesData);
}

/**
 * 更新删除按钮状态
 */
function updateDeleteButton() {
  const deleteBtn = document.getElementById("deleteBtn");
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
  const count = fileManager.getSelectedCount();
  if (count === 0) return;

  if (!confirm(`确定要删除选中的 ${count} 个项目吗？`)) return;

  const result = await fileManager.deleteSelected();
  showToast(
    `已删除 ${result.count}/${result.total} 个项目`,
    result.count === result.total ? "success" : "error",
  );
}

// ==================== 文件夹操作 ====================

/**
 * 显示新建文件夹模态框
 */
function showNewFolderModal() {
  document.getElementById("newFolderModal").style.display = "flex";
  document.getElementById("folderNameInput").value = "";
  document.getElementById("folderNameInput").focus();
}

/**
 * 隐藏新建文件夹模态框
 */
function hideNewFolderModal() {
  document.getElementById("newFolderModal").style.display = "none";
}

/**
 * 创建文件夹
 */
async function createFolder() {
  const name = document.getElementById("folderNameInput").value.trim();
  if (!name) {
    showToast("请输入文件夹名称", "error");
    return;
  }

  const result = await fileManager.createFolder(name);
  if (result.success) {
    hideNewFolderModal();
    showToast("文件夹创建成功", "success");
  } else {
    showToast("创建失败：" + result.message, "error");
  }
}

// ==================== 重命名 ====================

/**
 * 显示重命名模态框
 * @param {string} path - 路径
 * @param {string} name - 名称
 */
function showRenameModal(path, name) {
  selectedPathForAction = path;
  document.getElementById("renameModal").style.display = "flex";
  document.getElementById("renameInput").value = name;
  document.getElementById("renameInput").focus();
  document.getElementById("renameInput").select();
}

/**
 * 隐藏重命名模态框
 */
function hideRenameModal() {
  document.getElementById("renameModal").style.display = "none";
}

/**
 * 确认重命名
 */
async function confirmRename() {
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

  const result = await fileManager.getPreviewUrl(path);
  if (!result.success) {
    showToast("获取文件失败：" + result.message, "error");
    return;
  }

  const url = result.url;

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
  await fileManager.downloadFile(path);
}

// ==================== 上传 ====================

/**
 * 处理文件上传
 * @param {FileList} files - 文件列表
 */
async function handleUploadFiles(files) {
  if (!files || files.length === 0) return;

  uploadManager.setCurrentPath(fileManager.currentPath);

  for (const file of files) {
    uploadManager.addFile(file);
  }

  showUploadProgressModal();
  await uploadManager.uploadAll();
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
  const modal = document.getElementById("uploadProgressModal");
  const content = document.getElementById("uploadProgressContent");
  content.innerHTML = "";
  modal.style.display = "flex";
}

/**
 * 隐藏上传进度模态框
 */
function hideUploadProgressModal() {
  const modal = document.getElementById("uploadProgressModal");
  modal.style.display = "none";
}

// ==================== 复制/移动 ====================

/**
 * 显示复制/移动模态框
 * @param {string} path - 路径
 */
function showCopyMoveModal(path) {
  selectedPathForAction = path;
  document.getElementById("copyMoveModal").dataset.path = path;
  document.getElementById("copyMoveModal").style.display = "flex";
  document.getElementById("targetPathInput").value = "";
  document.getElementById("pathSelectorStatus").textContent =
    "点击输入框选择路径";
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
  document.getElementById("pathSelectorModal").style.display = "flex";
  loadPathSelector("/");
}

/**
 * 隐藏路径选择器
 */
function hidePathSelector() {
  document.getElementById("pathSelectorModal").style.display = "none";
}

/**
 * 加载路径选择器
 * @param {string} path - 路径
 */
async function loadPathSelector(path) {
  const content = document.getElementById("pathSelectorContent");
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

    if (result.code === 200 && result.data) {
      const rawItems = result.data.items || [];
      const dirs = rawItems
        .filter((item) => item.File === undefined)
        .map((item) => ({
          name: item.Directory?.name || "unknown",
          path: path.endsWith("/")
            ? path + (item.Directory?.name || "")
            : path + "/" + (item.Directory?.name || ""),
        }));

      const currentDirItem = {
        name: path === "/" ? "根目录" : path.split("/").pop() || "当前目录",
        path: path,
      };

      content.innerHTML = `
        <div class="file-item" onclick="selectPath('${escapeHtml(currentDirItem.path)}')" style="cursor: pointer; background: var(--selected-bg);">
            <div class="file-main">
                <div class="file-icon"><i class="ti ti-check"></i></div>
                <div class="file-name">${escapeHtml(currentDirItem.name)} (当前)</div>
            </div>
        </div>
        ${dirs
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
          .join("")}
      `;

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

/**
 * 选择路径
 * @param {string} path - 路径
 */
function selectPath(path) {
  document.getElementById("targetPathInput").value = path;
  hidePathSelector();
}

/**
 * 确认路径选择
 */
function confirmPathSelection() {
  const path = document.getElementById("targetPathInput").value;
  if (path) {
    document.getElementById("targetPathInput").value = path;
  }
  hidePathSelector();
}

/**
 * 确认复制/移动
 */
async function confirmCopyMove() {
  const targetPath = document.getElementById("targetPathInput").value.trim();
  const type = document.getElementById("copyMoveType").value;

  if (!targetPath) {
    showToast("请输入目标路径", "error");
    return;
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
}

// ==================== 右键菜单 ====================

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
        <div class="context-menu-item" onclick="copyShareUrl('${escapeHtml(path)}')">
            <i class="ti ti-link"></i> 复制分享链接
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
  await fileManager.copyPath(path);
  hideContextMenu();
}

/**
 * 复制分享链接
 * @param {string} path - 路径
 */
async function copyShareUrl(path) {
  await fileManager.copyShareUrl(path);
  hideContextMenu();
}

/**
 * 删除文件
 * @param {string} path - 路径
 */
async function deleteFile(path) {
  if (!confirm(`确定要删除 "${path}" 吗？`)) return;

  const result = await fileManager.remove(path);
  if (result.success) {
    showToast("删除成功", "success");
  } else {
    showToast("删除失败：" + result.message, "error");
  }
  hideContextMenu();
}

// ==================== 其他功能 ====================

/**
 * 打开管理后台（独立页面）
 */
function openAdminPanel() {
  // 先检查认证状态
  const authToken = localStorage.getItem("rlist_auth_token");
  if (!authToken) {
    showToast("请先登录", "error");
    return;
  }

  // 使用 location.href 直接跳转，避免被浏览器拦截
  window.location.href = "/admin.html";
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
