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

// 初始化
document.addEventListener("DOMContentLoaded", () => {
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
});

/**
 * 初始化主题
 */
function initTheme() {
  const currentTheme = localStorage.getItem("rlist_public_theme") || "light";
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
    localStorage.setItem("rlist_public_theme", "light");
  } else {
    document.documentElement.setAttribute("data-theme", "dark");
    if (themeIcon) themeIcon.className = "ti ti-sun";
    localStorage.setItem("rlist_public_theme", "dark");
  }
}

/**
 * 视图切换
 * @param {string} view - 视图类型
 */
function setView(view) {
  currentView = view;
  localStorage.setItem("rlist_public_view", view);
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
 * 加载文件列表
 * @param {string} path - 路径
 */
async function loadFiles(path = currentPath) {
  currentPath = path;
  const fileList = document.getElementById("fileList");
  if (!fileList) return;

  fileList.innerHTML = '<div class="loading"><div class="spinner"></div></div>';

  try {
    const response = await fetch(`${API_BASE}/list`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path: currentPath }),
    });

    const result = await response.json();

    if (result.code === 200 && result.data) {
      const rawItems = result.data.items || [];
      filesData = rawItems.map((item) => {
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
      renderFiles(filesData);
      updateBreadcrumb();
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
 * 更新面包屑导航
 */
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

/**
 * 导航到指定路径
 * @param {string} path - 路径
 */
function navigateTo(path) {
  currentPath = path || "/";
  loadFiles(currentPath);
}

/**
 * 进入文件夹
 * @param {string} path - 路径
 */
function enterFolder(path) {
  navigateTo(path);
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

  // 获取公开下载 URL
  const url = `${window.location.origin}${API_BASE}/download?path=${encodeURIComponent(path)}`;
  previewContent.innerHTML =
    '<div class="preview-placeholder"><i class="ti ti-loader"></i><p>加载中...</p></div>';

  if (imageExts.includes(ext)) {
    previewFileType = "image";
    previewContent.innerHTML = `<img src="${url}" alt="${escapeHtml(name)}" onload="onImageLoad()" onerror="onPreviewError()">`;
  } else if (videoExts.includes(ext)) {
    previewFileType = "video";
    previewContent.innerHTML = `<video controls src="${url}"></video>`;
  } else if (audioExts.includes(ext)) {
    previewFileType = "audio";
    previewContent.innerHTML = `<audio controls src="${url}"></audio>`;
  } else if (textExts.includes(ext)) {
    previewFileType = "text";
    try {
      const contentResponse = await fetch(url);
      const content = await contentResponse.text();
      previewContent.innerHTML = `<pre>${escapeHtml(content.substring(0, 100000))}</pre>`;
    } catch (e) {
      previewContent.innerHTML = `<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>无法加载文本内容</p></div>`;
    }
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
  const url = `${window.location.origin}${API_BASE}/download?path=${encodeURIComponent(path)}`;
  window.open(url, "_blank");
  showToast("下载已开始", "success");
}

/**
 * 复制分享链接（直链）
 * @param {string} path - 路径
 */
async function copyShareUrl(path) {
  try {
    // 先获取解析后的直链
    const response = await fetch(
      `${API_BASE}/download?path=${encodeURIComponent(path)}`,
    );
    const result = await response.json();

    let url;
    if (result.code === 200 && result.data && result.data.download_url) {
      // 使用解析后的直链
      url = result.data.download_url;
    } else {
      // 降级方案：使用公开访问链接
      url = `${window.location.origin}${API_BASE}/download?path=${encodeURIComponent(path)}`;
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
        <div class="context-menu-item" onclick="enterFolder('${escapeHtml(path)}')">
            <i class="ti ti-folder-open"></i> 打开
        </div>
      `
      : `
        <div class="context-menu-item" onclick="previewFile('${escapeHtml(path)}', '${escapeHtml(path.split("/").pop())}')">
            <i class="ti ti-eye"></i> 预览
        </div>
        <div class="context-menu-item" onclick="downloadFile('${escapeHtml(path)}')">
            <i class="ti ti-download"></i> 下载
        </div>
        <div class="context-menu-item" onclick="copyShareUrl('${escapeHtml(path)}')">
            <i class="ti ti-link"></i> 复制分享链接
        </div>
      `;

  menu.innerHTML = `
    ${actions}
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
