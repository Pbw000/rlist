/**
 * 文件上传模块
 * 支持 Direct 模式和 Relay 模式上传
 */

// 使用 api.js 中定义的 API_BASE 和 getAuthHeaders

/**
 * 上传任务类
 */
class UploadTask {
  constructor(file, path) {
    this.file = file;
    this.path = path;
    this.progress = 0;
    this.status = "waiting"; // waiting, uploading, completed, error
    this.message = "";
    this.onProgress = null;
  }

  updateProgress(progress, status, message = "") {
    this.progress = progress;
    this.status = status;
    this.message = message;
    if (this.onProgress) {
      this.onProgress(this);
    }
  }
}

/**
 * 文件上传管理器
 */
class UploadManager {
  constructor(options = {}) {
    this.apiBase = options.apiBase || API_BASE;
    this.currentPath = options.currentPath || "/";
    this.tasks = [];
    this.onTaskProgress = options.onTaskProgress || (() => {});
    this.onAllCompleted = options.onAllCompleted || (() => {});
  }

  /**
   * 设置当前路径
   * @param {string} path - 路径
   */
  setCurrentPath(path) {
    this.currentPath = path;
  }

  /**
   * 添加上传文件
   * @param {File} file - 文件对象
   * @returns {UploadTask} - 上传任务
   */
  addFile(file) {
    const path = this.currentPath.endsWith("/")
      ? this.currentPath + file.name
      : this.currentPath + "/" + file.name;

    const task = new UploadTask(file, path);
    task.onProgress = (t) => this.onTaskProgress(t);
    this.tasks.push(task);
    return task;
  }

  /**
   * 开始上传所有文件
   * @returns {Promise<void>}
   */
  async uploadAll() {
    for (const task of this.tasks) {
      if (task.status === "waiting") {
        await this.uploadTask(task);
      }
    }

    const allCompleted = this.tasks.every(
      (t) => t.status === "completed" || t.status === "error",
    );
    if (allCompleted) {
      this.onAllCompleted(this.tasks);
    }
  }

  /**
   * 上传单个任务
   * @param {UploadTask} task - 上传任务
   */
  async uploadTask(task) {
    const { file, path } = task;

    try {
      // 计算文件 hash
      task.updateProgress(0, "uploading", "计算文件 hash...");
      const hash = await calculateFileHash(file);

      // 获取上传信息
      task.updateProgress(5, "uploading", "获取上传信息...");
      const uploadInfoResp = await fetch(`${this.apiBase}/fs/upload-info`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          ...getAuthHeaders(),
        },
        body: JSON.stringify({
          path: path,
          size: file.size,
          hash: { sha256: hash },
        }),
      });

      if (uploadInfoResp.status === 401) {
        task.updateProgress(0, "error", "认证失败");
        return;
      }

      if (uploadInfoResp.status === 403) {
        task.updateProgress(0, "error", "权限不足");
        return;
      }

      const uploadInfoResult = await uploadInfoResp.json();

      if (uploadInfoResult.code !== 200) {
        task.updateProgress(0, "error", uploadInfoResult.message);
        return;
      }

      let uploadSuccess = false;

      // 判断上传模式
      if (uploadInfoResult.data && uploadInfoResult.data.mode === "direct") {
        try {
          uploadSuccess = await this.uploadDirect(task, uploadInfoResult.data);
        } catch (e) {
          console.warn(`Direct 模式失败：`, e.message);
          uploadSuccess = false;
        }
      }

      // Direct 模式失败，fallback 到 Relay 模式
      if (!uploadSuccess) {
        await this.uploadRelay(task, path, hash);
        uploadSuccess = true;
      }

      if (uploadSuccess) {
        task.updateProgress(100, "completed", "上传完成");
      }
    } catch (error) {
      task.updateProgress(0, "error", `错误：${error.message}`);
    }
  }

  /**
   * Direct 模式上传
   * @param {UploadTask} task - 上传任务
   * @param {Object} uploadInfo - 上传信息
   * @returns {Promise<boolean>} - 是否成功
   */
  async uploadDirect(task, uploadInfo) {
    const { file } = task;
    const {
      upload_url,
      method,
      form_fields,
      headers,
      complete_url,
      complete_params,
    } = uploadInfo;

    // 秒传情况
    if (upload_url === "about:blank") {
      return true;
    }

    try {
      if (form_fields && Object.keys(form_fields).length > 0) {
        // 需要表单字段的上传（如 S3）
        const formData = new FormData();
        Object.entries(form_fields).forEach(([key, value]) => {
          formData.append(key, value);
        });
        formData.append("file", file);

        const fetchOptions = {
          method: method || "POST",
          body: formData,
        };

        if (headers) {
          fetchOptions.headers = headers;
        }

        await this.uploadWithXhr(task, upload_url, fetchOptions, file.size);

        if (complete_url) {
          await this.callCompleteUrl(
            complete_url,
            file,
            form_fields,
            task.path,
            complete_params,
          );
        }
        return true;
      } else {
        // 直接上传文件内容（如 mcloud）
        const uploadHeaders = headers ? { ...headers } : {};
        if (!uploadHeaders["Content-Type"]) {
          uploadHeaders["Content-Type"] = "application/octet-stream";
        }

        const fetchOptions = {
          method: method || "PUT",
          headers: uploadHeaders,
        };

        await this.uploadWithXhr(
          task,
          upload_url,
          fetchOptions,
          file.size,
          file,
        );

        if (complete_url) {
          await this.callCompleteUrl(
            complete_url,
            file,
            form_fields,
            task.path,
            complete_params,
          );
        }
        return true;
      }
    } catch (error) {
      throw error;
    }
  }

  /**
   * Relay 模式上传
   * @param {UploadTask} task - 上传任务
   * @param {string} path - 路径
   * @param {string} hash - 文件 hash
   * @returns {Promise<boolean>} - 是否成功
   */
  async uploadRelay(task, path, hash) {
    const { file } = task;
    const timeoutMs = Math.max(10 * 60 * 1000, file.size * 2);

    const url = `${this.apiBase}/fs/upload?path=${encodeURIComponent(path)}&size=${file.size}&hash=${JSON.stringify({ sha256: hash })}`;

    try {
      await this.uploadWithXhr(
        task,
        url,
        { method: "PUT" },
        file.size,
        file,
        timeoutMs,
      );
      return true;
    } catch (error) {
      throw error;
    }
  }

  /**
   * 使用 XHR 进行带进度的上传
   * @param {UploadTask} task - 上传任务
   * @param {string} url - 上传 URL
   * @param {Object} fetchOptions - fetch 选项
   * @param {number} fileSize - 文件大小
   * @param {File} file - 文件对象
   * @param {number} timeoutMs - 超时时间
   * @returns {Promise<boolean>}
   */
  uploadWithXhr(
    task,
    url,
    fetchOptions,
    fileSize,
    file = null,
    timeoutMs = 600000,
  ) {
    return new Promise((resolve, reject) => {
      const xhr = new XMLHttpRequest();
      xhr.open(fetchOptions.method || "PUT", url, true);

      const isFormData = fetchOptions.body instanceof FormData;
      if (fetchOptions.headers && !isFormData) {
        Object.entries(fetchOptions.headers).forEach(([key, value]) => {
          xhr.setRequestHeader(key, value);
        });
      }

      const timeoutId = setTimeout(() => {
        xhr.abort();
        reject(new Error("上传超时"));
      }, timeoutMs);

      xhr.upload.addEventListener("progress", (event) => {
        if (event.lengthComputable) {
          const percent = Math.round((event.loaded / event.total) * 100);
          task.updateProgress(percent, "uploading", "上传中...");
        }
      });

      xhr.addEventListener("load", () => {
        clearTimeout(timeoutId);
        if (xhr.status >= 200 && xhr.status < 400) {
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

      xhr.addEventListener("error", () => {
        clearTimeout(timeoutId);
        reject(new Error("网络错误"));
      });

      if (isFormData) {
        xhr.send(fetchOptions.body);
      } else if (file) {
        xhr.send(file);
      } else if (fetchOptions.body) {
        xhr.send(fetchOptions.body);
      } else {
        xhr.send();
      }
    });
  }

  /**
   * 调用 complete_url 完成上传
   * @param {string} completeUrl - complete URL
   * @param {File} file - 文件
   * @param {Object} formFields - 表单字段
   * @param {string} originalPath - 原始路径
   * @param {Object} completeParamsFromInfo - 完成上传参数（从 upload_info 响应中获取）
   */
  async callCompleteUrl(
    completeUrl,
    file,
    formFields,
    originalPath,
    completeParamsFromInfo,
  ) {
    try {
      // 优先使用传入的 complete_params，fallback 到旧方式解析
      let uploadId = "";
      let fileId = "";
      let contentHash = null;

      if (completeParamsFromInfo) {
        uploadId = completeParamsFromInfo.upload_id || "";
        fileId = completeParamsFromInfo.file_id || "";
        contentHash = completeParamsFromInfo.content_hash;
      }

      // 如果没有 complete_params，尝试从 URL 和 formFields 中解析
      if (!uploadId || !fileId) {
        const url = new URL(completeUrl, window.location.origin);
        const params = new URLSearchParams(url.search);

        fileId =
          fileId ||
          params.get("file_id") ||
          (formFields && formFields.fileId) ||
          params.get("fileId") ||
          "";
        uploadId =
          uploadId ||
          params.get("upload_id") ||
          (formFields && formFields.uploadId) ||
          params.get("uploadId") ||
          "";
      }

      // 如果没有 content_hash，计算文件 hash
      if (!contentHash) {
        contentHash = { sha256: await calculateFileHash(file) };
      }

      // 构建完成上传请求体
      const requestBody = {
        path: originalPath || "",
        info: {
          upload_id: uploadId,
          file_id: fileId,
          content_hash: contentHash,
        },
      };

      // 获取认证头
      const authToken = localStorage.getItem("rlist_auth_token");
      const headers = {
        "Content-Type": "application/json",
      };
      if (authToken) {
        headers["AUTH-JWT-TOKEN"] = authToken;
      }

      // 解析 completeUrl，获取路径
      const url = new URL(completeUrl, window.location.origin);
      const fullUrl = `${window.location.origin}${url.pathname}`;

      const response = await fetch(fullUrl, {
        method: "POST",
        headers: headers,
        body: JSON.stringify(requestBody),
      });

      if (!response.ok) {
        console.warn(`调用 complete 接口失败：`, await response.text());
      }
    } catch (error) {
      console.warn(`调用 complete_url 失败:`, error.message);
    }
  }

  /**
   * 获取所有任务状态
   * @returns {Array} - 任务列表
   */
  getTasks() {
    return this.tasks;
  }

  /**
   * 获取上传进度统计
   * @returns {Object} - 进度统计
   */
  getProgress() {
    const total = this.tasks.length;
    const completed = this.tasks.filter((t) => t.status === "completed").length;
    const failed = this.tasks.filter((t) => t.status === "error").length;
    const uploading = this.tasks.filter((t) => t.status === "uploading").length;
    const waiting = this.tasks.filter((t) => t.status === "waiting").length;

    return {
      total,
      completed,
      failed,
      uploading,
      waiting,
      percent: total > 0 ? Math.round((completed / total) * 100) : 0,
    };
  }

  /**
   * 清除已完成的任务
   */
  clearCompleted() {
    this.tasks = this.tasks.filter(
      (t) => t.status !== "completed" && t.status !== "error",
    );
  }
}
