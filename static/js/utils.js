/**
 * 公共工具函数
 * 用于文件管理前端页面
 */

/**
 * HTML 转义
 * @param {string} text - 需要转义的文本
 * @returns {string} - 转义后的文本
 */
function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

/**
 * 根据文件扩展名获取图标类名
 * @param {string} name - 文件名
 * @returns {string} - 图标 HTML
 */
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
    bmp: "ti-file-image",
    ico: "ti-file-image",
    mp3: "ti-file-music",
    wav: "ti-file-music",
    flac: "ti-file-music",
    aac: "ti-file-music",
    ogg: "ti-file-music",
    mp4: "ti-file-video",
    avi: "ti-file-video",
    mkv: "ti-file-video",
    mov: "ti-file-video",
    webm: "ti-file-video",
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
    sh: "ti-file-code",
  };
  const icon = iconMap[ext] || "ti-file";
  return `<i class="ti ${icon}"></i>`;
}

/**
 * 格式化文件大小
 * @param {number} bytes - 字节数
 * @returns {string} - 格式化后的大小
 */
function formatSize(bytes) {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
}

/**
 * 格式化日期
 * @param {string} dateStr - 日期字符串
 * @returns {string} - 格式化后的日期
 */
function formatDate(dateStr) {
  try {
    return new Date(dateStr).toLocaleDateString("zh-CN");
  } catch {
    return dateStr;
  }
}

/**
 * 显示提示消息
 * @param {string} message - 消息内容
 * @param {string} type - 消息类型 (success, error, info)
 */
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
 * @param {string} view - 视图类型 (list, grid)
 */
function setView(view) {
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
 * 计算文件 SHA256 hash
 * @param {File} file - 文件对象
 * @returns {Promise<string>} - hash 值
 */
async function calculateFileHash(file) {
  const buffer = await file.arrayBuffer();
  const hashBuffer = await crypto.subtle.digest("SHA-256", buffer);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  const hashHex = hashArray
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
  return hashHex;
}

/**
 * 解析后端返回的 Hash 枚举格式
 * @param {any} hashData - 后端返回的 hash 数据 ({ sha256: "..." } | { md5: "..." } | "empty")
 * @returns {{ algo: string, value: string } | null} - 返回算法和值，如果没有则返回 null
 */
function parseHash(hashData) {
  if (!hashData || hashData === "empty") {
    return null;
  }
  if (typeof hashData === "object") {
    if (hashData.sha256) {
      return { algo: "sha256", value: hashData.sha256 };
    }
    if (hashData.md5) {
      return { algo: "md5", value: hashData.md5 };
    }
  }
  return null;
}

/**
 * 格式化 Hash 为后端发送的格式
 * @param {string} hashValue - hash 值
 * @param {string} [algo="sha256"] - 算法类型
 * @returns {Object} - { sha256: "..." } 或 { md5: "..." }
 */
function formatHash(hashValue, algo = "sha256") {
  if (!hashValue) {
    return "empty";
  }
  return { [algo]: hashValue };
}

/**
 * 复制到剪贴板
 * @param {string} text - 要复制的文本
 * @returns {Promise<boolean>} - 是否成功
 */
async function copyToClipboard(text) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    // 降级方案
    const textArea = document.createElement("textarea");
    textArea.value = text;
    textArea.style.position = "fixed";
    textArea.style.left = "-999999px";
    document.body.appendChild(textArea);
    textArea.select();
    try {
      document.execCommand("copy");
      document.body.removeChild(textArea);
      return true;
    } catch {
      document.body.removeChild(textArea);
      return false;
    }
  }
}
