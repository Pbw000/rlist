/**
 * 文件管理模块
 * 提供文件列表、导航、操作等功能
 */

/**
 * 文件管理器类
 */
class FileManager {
  constructor(options = {}) {
    this.apiBase = options.apiBase || "/api";
    this.currentPath = "/";
    this.filesData = [];
    this.selectedFiles = new Set();
    this.onFilesLoaded = options.onFilesLoaded || (() => {});
    this.onError = options.onError || (() => {});

    // 分页相关
    this.currentCursor = null; // 开始偏移量
    this.hasMorePages = false;
    this.isLoadingMore = false;
    this.PAGE_SIZE = 20;
  }

  /**
   * 获取认证头
   */
  getAuthHeaders() {
    const headers = {};
    const authToken = localStorage.getItem("rlist_auth_token");
    if (authToken) {
      headers["AUTH-JWT-TOKEN"] = authToken;
    }
    return headers;
  }

  /**
   * 加载文件列表（支持分页）
   * @param {string} path - 路径
   * @param {boolean} reset - 是否重置分页
   */
  async loadFiles(path = this.currentPath, reset = true) {
    this.currentPath = path;

    // 重置分页状态
    if (reset) {
      this.currentCursor = null;
      this.hasMorePages = false;
      this.filesData = [];
    } else {
      // 加载更多时跳过
      if (this.isLoadingMore) return { success: false, message: "正在加载" };
      this.isLoadingMore = true;
    }

    try {
      const requestBody = {
        path: path,
        per_page: this.PAGE_SIZE,
      };

      // 添加游标参数
      if (this.currentCursor !== null) {
        requestBody.cursor = this.currentCursor;
      }

      const response = await fetch(`${this.apiBase}/fs/list`, {
        method: "POST",
        headers: {
          ...this.getAuthHeaders(),
          "Content-Type": "application/json",
        },
        body: JSON.stringify(requestBody),
      });

      if (response.status === 401) {
        showToast("认证失败，请重新登录", "error");
        logout();
        return { success: false, code: 401 };
      }

      if (response.status === 403) {
        showToast("权限不足，无法查看文件列表", "error");
        return { success: false, code: 403 };
      }

      const result = await response.json();

      if (result.code === 200 && result.data) {
        const rawItems = result.data.items || [];
        const newItems = rawItems.map((item) => {
          const isDir = item.File === undefined;
          return {
            name: item.File?.name || item.Directory?.name || "unknown",
            path: path.endsWith("/")
              ? path + (item.File?.name || item.Directory?.name || "")
              : path + "/" + (item.File?.name || item.Directory?.name || ""),
            size: item.File?.size || 0,
            file_type: isDir ? "dir" : "file",
            modified: item.File?.modified_at || item.Directory?.modified_at,
          };
        });

        // 更新分页状态
        this.currentCursor = result.data.next_cursor;
        this.hasMorePages =
          this.currentCursor !== null && this.currentCursor !== undefined;

        if (reset) {
          this.filesData = newItems;
        } else {
          this.filesData = [...this.filesData, ...newItems];
          this.isLoadingMore = false;
        }

        // 传递 append 参数
        this.onFilesLoaded(this.filesData, !reset);
        return { success: true, data: this.filesData };
      } else {
        this.onError(result.message || "加载失败");
        return { success: false, message: result.message };
      }
    } catch (error) {
      this.onError("网络错误：" + error.message);
      return { success: false, error: error.message };
    }
  }

  /**
   * 加载更多文件
   */
  async loadMoreFiles() {
    if (!this.hasMorePages || this.isLoadingMore) {
      return { success: false, message: "没有更多数据" };
    }
    return this.loadFiles(this.currentPath, false);
  }

  /**
   * 导航到指定路径
   * @param {string} path - 路径
   */
  navigateTo(path) {
    this.currentPath = path || "/";
    this.selectedFiles.clear();
    return this.loadFiles(this.currentPath);
  }

  /**
   * 进入文件夹
   * @param {string} path - 文件夹路径
   */
  enterFolder(path) {
    return this.navigateTo(path);
  }

  /**
   * 刷新文件列表
   */
  refresh() {
    return this.loadFiles(this.currentPath);
  }

  /**
   * 搜索文件
   * @param {string} query - 搜索关键词
   * @returns {Array} - 过滤后的文件列表
   */
  search(query) {
    if (!query) {
      return this.filesData;
    }
    return this.filesData.filter((f) =>
      f.name.toLowerCase().includes(query.toLowerCase()),
    );
  }

  /**
   * 切换文件选择状态
   * @param {string} path - 文件路径
   * @param {boolean} checked - 是否选中
   */
  toggleSelection(path, checked) {
    if (checked) {
      this.selectedFiles.add(path);
    } else {
      this.selectedFiles.delete(path);
    }
  }

  /**
   * 全选/取消全选
   * @param {boolean} checked - 是否全选
   */
  toggleSelectAll(checked) {
    if (checked) {
      this.filesData.forEach((f) => this.selectedFiles.add(f.path));
    } else {
      this.selectedFiles.clear();
    }
  }

  /**
   * 获取选中的文件数量
   * @returns {number} - 选中数量
   */
  getSelectedCount() {
    return this.selectedFiles.size;
  }

  /**
   * 创建文件夹
   * @param {string} name - 文件夹名称
   * @returns {Promise<Object>} - 操作结果
   */
  async createFolder(name) {
    const path = this.currentPath.endsWith("/")
      ? this.currentPath + name
      : this.currentPath + "/" + name;

    const result = await apiRequest("/fs/mkdir", {
      method: "POST",
      body: JSON.stringify({ path }),
    });

    if (result.code === 200) {
      await this.refresh();
      return { success: true };
    } else {
      return { success: false, message: result.message };
    }
  }

  /**
   * 重命名文件/文件夹
   * @param {string} path - 原路径
   * @param {string} newName - 新名称
   * @returns {Promise<Object>} - 操作结果
   */
  async rename(path, newName) {
    const result = await apiRequest("/fs/rename", {
      method: "POST",
      body: JSON.stringify({
        src_path: path,
        new_name: newName,
      }),
    });

    if (result.code === 200) {
      await this.refresh();
      return { success: true };
    } else {
      return { success: false, message: result.message };
    }
  }

  /**
   * 删除文件/文件夹
   * @param {string} path - 路径
   * @returns {Promise<Object>} - 操作结果
   */
  async remove(path) {
    const result = await apiRequest("/fs/remove", {
      method: "POST",
      body: JSON.stringify({ path }),
    });

    if (result.code === 200) {
      await this.refresh();
      return { success: true };
    } else {
      return { success: false, message: result.message };
    }
  }

  /**
   * 批量删除选中的文件
   * @returns {Promise<Object>} - 操作结果
   */
  async deleteSelected() {
    const paths = Array.from(this.selectedFiles);
    let successCount = 0;
    const authToken = localStorage.getItem("rlist_auth_token");

    for (const path of paths) {
      try {
        const headers = {
          "Content-Type": "application/json",
        };
        if (authToken) {
          headers["AUTH-JWT-TOKEN"] = authToken;
        }

        const response = await fetch(`${this.apiBase}/fs/remove`, {
          method: "POST",
          headers: headers,
          body: JSON.stringify({ path }),
        });

        if (response.ok) {
          successCount++;
        }
      } catch (error) {
        console.error("删除失败:", error);
      }
    }

    this.selectedFiles.clear();
    await this.refresh();

    return {
      success: successCount === paths.length,
      count: successCount,
      total: paths.length,
    };
  }

  /**
   * 复制/移动文件
   * @param {string} srcPath - 源路径
   * @param {string} dstPath - 目标路径
   * @param {string} action - 操作类型 (copy, move)
   * @returns {Promise<Object>} - 操作结果
   */
  async copyOrMove(srcPath, dstPath, action = "copy") {
    const endpoint = action === "copy" ? "/fs/copy" : "/fs/move";
    const result = await apiRequest(endpoint, {
      method: "POST",
      body: JSON.stringify({
        src_path: srcPath,
        dst_path: dstPath,
      }),
    });

    if (result.code === 200) {
      await this.refresh();
      return { success: true };
    } else {
      return { success: false, message: result.message };
    }
  }

  /**
   * 获取文件下载链接
   * @param {string} path - 文件路径
   * @returns {Promise<Object>} - 下载链接信息
   */
  async getDownloadUrl(path) {
    try {
      const response = await fetch(
        `${this.apiBase}/fs/get?path=${encodeURIComponent(path)}`,
        { headers: this.getAuthHeaders() },
      );

      if (response.status === 401) {
        showToast("认证失败，请重新登录", "error");
        logout();
        return { success: false, code: 401 };
      }

      if (response.status === 403) {
        showToast("权限不足，无法下载文件", "error");
        return { success: false, code: 403 };
      }

      const result = await response.json();

      if (result.code === 200 && result.data) {
        return { success: true, url: result.data.url };
      } else {
        return { success: false, message: result.message };
      }
    } catch (error) {
      return { success: false, error: error.message };
    }
  }

  /**
   * 下载文件
   * @param {string} path - 文件路径
   */
  async downloadFile(path) {
    const result = await this.getDownloadUrl(path);
    if (result.success) {
      window.open(result.url, "_blank");
      showToast("下载已开始", "success");
      return true;
    } else {
      showToast(result.message || "下载失败", "error");
      return false;
    }
  }

  /**
   * 获取文件预览 URL
   * @param {string} path - 文件路径
   * @returns {Promise<Object>} - 预览 URL 信息
   */
  async getPreviewUrl(path) {
    return await this.getDownloadUrl(path);
  }

  /**
   * 复制文件路径到剪贴板
   * @param {string} path - 文件路径
   * @returns {Promise<boolean>} - 是否成功
   */
  async copyPath(path) {
    const success = await copyToClipboard(path);
    if (success) {
      showToast("路径已复制到剪贴板", "success");
    } else {
      showToast("复制失败", "error");
    }
    return success;
  }

  /**
   * 获取文件分享链接
   * @param {string} path - 文件路径
   * @returns {Promise<string>} - 分享链接（直链）
   */
  async getShareUrl(path) {
    // 获取真实的下载直链
    const result = await this.getDownloadUrl(path);
    if (result.success) {
      return result.url; // 返回解析后的直链
    }
    // 降级方案：返回公开访问链接
    return `${window.location.origin}/obs/download?path=${encodeURIComponent(path)}`;
  }

  /**
   * 复制文件分享链接
   * @param {string} path - 文件路径
   * @returns {Promise<boolean>} - 是否成功
   */
  async copyShareUrl(path) {
    const url = await this.getShareUrl(path);
    const success = await copyToClipboard(url);
    if (success) {
      showToast("分享链接已复制到剪贴板", "success");
    } else {
      showToast("复制失败", "error");
    }
    return success;
  }
}
