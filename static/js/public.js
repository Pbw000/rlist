// 公开访问页面 JavaScript
// 全局变量
const API_BASE = "/obs";
let currentPath = "/";
let filesData = [];
let currentView = localStorage.getItem("rlist_public_view") || "list";
let currentTheme = localStorage.getItem("rlist_public_theme") || "light";
let previewFilePath = "";

// 初始化
document.addEventListener("DOMContentLoaded", () => {
  // 初始化主题
  if (currentTheme === "dark") {
    document.documentElement.setAttribute("data-theme", "dark");
    document.getElementById("themeIcon").className = "ti ti-sun";
  }

  loadFiles();
});

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
  localStorage.setItem("rlist_public_theme", currentTheme);
}

// 视图切换
function setView(view) {
  currentView = view;
  localStorage.setItem("rlist_public_view", view);
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
    const response = await fetch(`${API_BASE}/list`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path: currentPath }),
    });

    const result = await response.json();

    if (result.code === 200 && result.data) {
      // 后端返回 FileList 结构：{ items, total, next_cursor }
      // Meta 枚举：File { name, size, modified_at } 或 Directory { name, modified_at }
      const rawItems = result.data.items || [];
      // 转换为前端期望的格式
      filesData = rawItems.map((item) => {
        const isDir = item.File === undefined; // 没有 File 字段就是目录
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
                         ondblclick="handleDoubleClick('${escapeHtml(file.path)}', '${file.file_type}')">
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
    // 使用公开下载接口获取文件 URL
    const url = `${window.location.origin}${API_BASE}/download?path=${encodeURIComponent(path)}`;
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
        const contentResponse = await fetch(url);
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
    // 使用公开下载接口
    const url = `${window.location.origin}${API_BASE}/download?path=${encodeURIComponent(path)}`;
    window.open(url, "_blank");
    showToast("下载已开始", "success");
  } catch (error) {
    showToast("网络错误：" + error.message, "error");
  }
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

// 回车键处理
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    hidePreviewModal();
  }
});
