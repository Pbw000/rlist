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
 * 将 bigint 转为 big-endian 8 字节数组
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
 * 获取 Challenge（盐值）
 * @returns {Promise<{success: boolean, salt?: bigint, message?: string}>} - Challenge 结果
 */
async function getChallenge() {
  try {
    const response = await fetch("/api/challenge", {
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
 * 生成随机 nonce
 * @returns {string} - 随机 nonce
 */
function generateRandomNonce() {
  const array = new Uint8Array(16);
  crypto.getRandomValues(array);
  return Array.from(array)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/**
 * 为公开 API 请求构建带 Challenge 的请求体
 * @param {Object} baseBody - 基础请求体
 * @param {number} difficulty - 难度
 * @returns {Promise<Object>} - 带 Challenge 的请求体
 */
async function buildPublicRequest(baseBody, difficulty = 3) {
  const challengeResult = await getChallenge();
  if (!challengeResult.success) {
    throw new Error(challengeResult.message);
  }
  const salt = challengeResult.salt; // BigInt 类型

  // 使用秒级时间戳（与后端保持一致）
  const timestamp = Math.floor(Date.now() / 1000);
  const path = baseBody.path || "";

  // 使用 Worker 在后台计算 challenge
  const { nonce, claim } = await computeChallengeInWorker(
    salt,
    timestamp,
    path,
    difficulty,
  );

  return {
    ...baseBody,
    salt: salt.toString(),
    timestamp,
    nonce,
    claim,
  };
}

/**
 * 在 Web Worker 中计算 Challenge
 * @param {bigint} salt - Salt 值
 * @param {number} timestamp - 时间戳
 * @param {string} path - 路径
 * @param {number} difficulty - 难度
 * @returns {Promise<{nonce: string, claim: string}>}
 */
function computeChallengeInWorker(salt, timestamp, path, difficulty) {
  return new Promise((resolve, reject) => {
    // 创建 Worker
    const workerCode = `
      self.onmessage = async function(e) {
        const { salt, timestamp, path, difficulty } = e.data;

        // SHA512 实现
        async function sha512(data) {
          const hashBuffer = await crypto.subtle.digest("SHA-512", data);
          const hashArray = Array.from(new Uint8Array(hashBuffer));
          return hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");
        }

        // 字符串转字节
        function stringToBytes(str) {
          return new TextEncoder().encode(str);
        }

        // BigInt 转大端序字节
        function bigIntToBigEndianBytes(num) {
          const bytes = new Uint8Array(8);
          for (let i = 7; i >= 0; i--) {
            bytes[i] = Number(num & 0xffn);
            num = num >> 8n;
          }
          return bytes;
        }

        // 合并字节
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

        // 生成随机 nonce
        function generateRandomNonce() {
          const array = new Uint8Array(16);
          crypto.getRandomValues(array);
          return Array.from(array).map((b) => b.toString(16).padStart(2, "0")).join("");
        }

        const saltHex = salt.toString(16);
        const pathBytes = stringToBytes(path);
        const timestampBytes = bigIntToBigEndianBytes(BigInt(timestamp));
        const saltHexBytes = stringToBytes(saltHex);
        const targetZeros = "0".repeat(difficulty);

        let nonce = generateRandomNonce();
        let nonceBytes = stringToBytes(nonce);
        let combinedData = mergeBytes(nonceBytes, pathBytes, timestampBytes, saltHexBytes);
        let claim = await sha512(combinedData);

        // 寻找满足难度的 nonce
        while (!claim.startsWith(targetZeros)) {
          nonce = generateRandomNonce();
          nonceBytes = stringToBytes(nonce);
          combinedData = mergeBytes(nonceBytes, pathBytes, timestampBytes, saltHexBytes);
          claim = await sha512(combinedData);
        }

        self.postMessage({ nonce, claim });
      };
    `;

    const blob = new Blob([workerCode], { type: "application/javascript" });
    const worker = new Worker(URL.createObjectURL(blob));

    worker.onmessage = (e) => {
      resolve(e.data);
      worker.terminate();
    };

    worker.onerror = (error) => {
      reject(error);
      worker.terminate();
    };

    // 发送数据给 Worker
    worker.postMessage({ salt, timestamp, path, difficulty });
  });
}
