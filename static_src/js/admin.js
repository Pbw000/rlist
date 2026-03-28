/**
 * 管理后台逻辑
 */

// 全局变量
let currentPage = "users";
let sidebarOpen = false;

// 初始化
document.addEventListener("DOMContentLoaded", () => {
  checkAuthAndLoad();
  initTheme();

  // 全局错误处理 - 捕获未处理的异常
  window.addEventListener("error", (e) => {
    console.error("全局错误:", e.error);
  });

  // 捕获未处理的 Promise rejection
  window.addEventListener("unhandledrejection", (e) => {
    console.error("未处理的 Promise rejection:", e.reason);
  });
});

/**
 * 切换侧边栏（移动端）
 */
function toggleSidebar() {
  try {
    const sidebar = document.querySelector(".admin-sidebar");
    sidebarOpen = !sidebarOpen;
    if (sidebarOpen) {
      sidebar.classList.add("mobile-open");
    } else {
      sidebar.classList.remove("mobile-open");
    }
  } catch (error) {
    console.error("切换侧边栏失败:", error);
  }
}

// 点击遮罩关闭侧边栏
document.addEventListener("click", (e) => {
  const sidebar = document.querySelector(".admin-sidebar");
  const menuToggle = document.querySelector(".menu-toggle");
  if (
    sidebarOpen &&
    !sidebar.contains(e.target) &&
    !menuToggle.contains(e.target)
  ) {
    sidebarOpen = false;
    sidebar.classList.remove("mobile-open");
  }
});

/**
 * 初始化主题
 */
function initTheme() {
  const currentTheme = localStorage.getItem("rlist_theme") || "light";
  if (currentTheme === "dark") {
    document.documentElement.setAttribute("data-theme", "dark");
  }
}

/**
 * 检查认证并加载
 */
async function checkAuthAndLoad() {
  const authToken = localStorage.getItem("rlist_auth_token");
  const currentUser = localStorage.getItem("rlist_current_user");

  if (!authToken) {
    showToast("请先登录", "error");
    window.location.href = "/index.html";
    return;
  }

  const isAuth = await checkAuth();
  if (!isAuth) {
    showToast("认证失败，请重新登录", "error");
    window.location.href = "/index.html";
    return;
  }

  // 检查是否为管理员
  const isAdmin = await checkIsAdmin();
  if (!isAdmin) {
    showToast("您不是管理员，无法访问管理后台", "error");
    window.location.href = "/index.html";
    return;
  }

  // 显示用户名
  document.getElementById("adminUsername").textContent =
    currentUser || "管理员";

  // 加载用户列表
  loadUsers();
}

/**
 * 切换页面
 * @param {string} page - 页面名称
 */
function switchPage(page) {
  try {
    currentPage = page;

    // 更新导航状态
    document.querySelectorAll(".nav-item").forEach((item) => {
      if (item.dataset.page === page) {
        item.classList.add("active");
      } else {
        item.classList.remove("active");
      }
    });

    // 更新页面标题
    const titles = {
      users: "用户管理",
      storages: "存储管理",
    };
    document.getElementById("pageTitle").textContent =
      titles[page] || "管理后台";

    // 切换内容区域
    document.getElementById("usersPage").style.display =
      page === "users" ? "block" : "none";
    document.getElementById("storagesPage").style.display =
      page === "storages" ? "block" : "none";

    // 加载对应数据
    if (page === "users") {
      loadUsers();
    } else if (page === "storages") {
      loadStorages();
    }
  } catch (error) {
    console.error("切换页面失败:", error);
    showToast("切换页面失败：" + error.message, "error");
  }
}

/**
 * 加载用户列表
 */
async function loadUsers() {
  const tbody = document.getElementById("usersTableBody");
  tbody.innerHTML = `
    <tr>
      <td colspan="11" class="empty-cell">
        <div class="loading-progress loading-progress-indeterminate">
          <div class="loading-progress-bar"><div class="loading-progress-fill"></div></div>
          <div class="loading-progress-text">正在加载用户列表...</div>
        </div>
      </td>
    </tr>
  `;

  const result = await listUsers();
  if (result.code === 200 && result.data) {
    const users = result.data;
    if (users.length === 0) {
      tbody.innerHTML =
        '<tr><td colspan="11" class="empty-cell">暂无用户</td></tr>';
      return;
    }

    tbody.innerHTML = users
      .map(
        (user) => `
        <tr>
          <td><strong>${escapeHtml(user.username)}</strong></td>
          <td><span class="root-dir-cell">${escapeHtml(user.root_dir || "无限制")}</span></td>
          <td>${renderPermissionBadge(user.permissions.read)}</td>
          <td>${renderPermissionBadge(user.permissions.download)}</td>
          <td>${renderPermissionBadge(user.permissions.upload)}</td>
          <td>${renderPermissionBadge(user.permissions.delete)}</td>
          <td>${renderPermissionBadge(user.permissions.move_obj)}</td>
          <td>${renderPermissionBadge(user.permissions.copy)}</td>
          <td>${renderPermissionBadge(user.permissions.create_dir)}</td>
          <td>${renderPermissionBadge(user.permissions.list)}</td>
          <td>
            ${
              user.username !== "admin"
                ? `<div class="action-buttons">
                    <button class="action-btn-sm edit" onclick="showEditPermissionsModal('${escapeHtml(user.username)}')" title="编辑权限">
                      <i class="ti ti-edit"></i>
                      <span>编辑</span>
                    </button>
                    <button class="action-btn-sm delete" onclick="confirmDeleteUser('${escapeHtml(user.username)}')" title="删除用户">
                      <i class="ti ti-trash"></i>
                      <span>删除</span>
                    </button>
                  </div>`
                : '<span style="color: var(--text-secondary); font-size: 12px;">不可修改</span>'
            }
          </td>
        </tr>
      `,
      )
      .join("");
  } else {
    tbody.innerHTML = `<tr><td colspan="11" class="empty-cell">加载失败：${result.message || "未知错误"}</td></tr>`;
  }
}

/**
 * 渲染权限徽章
 * @param {boolean} enabled - 是否启用
 * @returns {string} - HTML 字符串
 */
function renderPermissionBadge(enabled) {
  if (enabled) {
    return '<span class="permission-badge enabled"><i class="ti ti-check"></i></span>';
  } else {
    return '<span class="permission-badge disabled"><i class="ti ti-x"></i></span>';
  }
}

/**
 * 显示添加用户模态框
 */
function showAddUserModal() {
  try {
    document.getElementById("addUserModal").style.display = "flex";
    document.getElementById("addUsername").value = "";
    document.getElementById("addPassword").value = "";
    document.getElementById("addRootDir").value = "";
    document.getElementById("permRead").checked = true;
    document.getElementById("permDownload").checked = true;
    document.getElementById("permUpload").checked = true;
    document.getElementById("permDelete").checked = false;
    document.getElementById("permMove").checked = false;
    document.getElementById("permCopy").checked = false;
    document.getElementById("permCreateDir").checked = true;
    document.getElementById("permList").checked = true;
    document.getElementById("addUsername").focus();
  } catch (error) {
    console.error("显示添加用户模态框失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 隐藏添加用户模态框
 */
function hideAddUserModal() {
  try {
    document.getElementById("addUserModal").style.display = "none";
  } catch (error) {
    console.error("隐藏添加用户模态框失败:", error);
  }
}

/**
 * 确认添加用户
 */
async function confirmAddUser() {
  try {
    const username = document.getElementById("addUsername").value.trim();
    const password = document.getElementById("addPassword").value;

    if (!username) {
      showToast("请输入用户名", "error");
      return;
    }

    if (password.length < 8) {
      showToast("密码长度至少为 8 位", "error");
      return;
    }

    // 检查密码复杂度
    const hasLetter = /[a-zA-Z]/.test(password);
    const hasDigit = /[0-9]/.test(password);
    if (!hasLetter || !hasDigit) {
      showToast("密码必须包含字母和数字", "error");
      return;
    }

    const permissions = {
      read: document.getElementById("permRead").checked,
      download: document.getElementById("permDownload").checked,
      upload: document.getElementById("permUpload").checked,
      delete: document.getElementById("permDelete").checked,
      move_obj: document.getElementById("permMove").checked,
      copy: document.getElementById("permCopy").checked,
      create_dir: document.getElementById("permCreateDir").checked,
      list: document.getElementById("permList").checked,
    };

    const rootDir = document.getElementById("addRootDir").value.trim();
    const result = await addUser(
      username,
      password,
      permissions,
      rootDir || null,
    );
    if (result.code === 200) {
      hideAddUserModal();
      showToast("用户添加成功", "success");
      loadUsers();
    } else {
      showToast("添加失败：" + result.message, "error");
    }
  } catch (error) {
    console.error("添加用户失败:", error);
    showToast("添加失败：" + error.message, "error");
  }
}

/**
 * 确认删除用户
 * @param {string} username - 用户名
 */
async function confirmDeleteUser(username) {
  try {
    if (!confirm(`确定要删除用户 "${username}" 吗？此操作不可撤销！`)) {
      return;
    }

    const result = await removeUser(username);
    if (result.code === 200) {
      showToast("用户删除成功", "success");
      loadUsers();
    } else {
      showToast("删除失败：" + result.message, "error");
    }
  } catch (error) {
    console.error("删除用户失败:", error);
    showToast("删除失败：" + error.message, "error");
  }
}

/**
 * 加载存储列表
 */
async function loadStorages() {
  const tbody = document.getElementById("storagesTableBody");
  tbody.innerHTML = `
    <tr>
      <td colspan="5" class="empty-cell">
        <div class="loading-progress loading-progress-indeterminate">
          <div class="loading-progress-bar"><div class="loading-progress-fill"></div></div>
          <div class="loading-progress-text">正在加载存储列表...</div>
        </div>
      </td>
    </tr>
  `;

  const result = await listStorages();

  if (result.code === 200 && result.data) {
    const storages = result.data;
    // 合并公开和私有存储
    const allStorages = [
      ...(storages.public || []).map((s) => ({ ...s, type: "public" })),
      ...(storages.private || []).map((s) => ({ ...s, type: "private" })),
    ];

    if (allStorages.length === 0) {
      tbody.innerHTML =
        '<tr><td colspan="5" class="empty-cell">暂无存储</td></tr>';
      return;
    }

    tbody.innerHTML = allStorages
      .map(
        (storage) => `
        <tr>
          <td><strong>${escapeHtml(storage.name)}</strong></td>
          <td>${escapeHtml(storage.driver_name)}</td>
          <td><code style="font-size: 12px; color: var(--text-secondary);">${escapeHtml(storage.path)}</code></td>
          <td>
            <span class="storage-type-badge ${storage.type}">${storage.type === "public" ? "公开" : "私有"}</span>
            <span class="storage-status work"><i class="ti ti-check"></i> 正常</span>
          </td>
          <td>
            ${
              storage.name !== "default"
                ? `<button class="action-btn-sm delete" onclick="confirmDeleteStorage('${escapeHtml(storage.name)}', '${storage.type}', ${storage.idx})" title="删除存储">
                    <i class="ti ti-trash"></i>
                    <span>删除</span>
                  </button>`
                : '<span style="color: var(--text-secondary); font-size: 12px;">默认存储</span>'
            }
          </td>
        </tr>
      `,
      )
      .join("");
  } else {
    tbody.innerHTML = `<tr><td colspan="5" class="empty-cell">加载失败：${result.message || "未知错误"}</td></tr>`;
  }
}

/**
 * 显示添加存储模态框
 */
async function showAddStorageModal() {
  try {
    document.getElementById("addStorageModal").style.display = "flex";
    document.getElementById("storagePrefix").value = "";
    document.getElementById("storagePublic").checked = false;

    // 加载驱动列表
    const driversResult = await getStorageDrivers();
    const select = document.getElementById("storageDriver");
    select.innerHTML = '<option value="">请选择驱动</option>';

    if (driversResult.code === 200 && driversResult.data) {
      driversResult.data.forEach((driver) => {
        const option = document.createElement("option");
        option.value = driver.value;
        option.textContent = driver.label;
        select.appendChild(option);
      });
      // 保存驱动列表供后续使用
      window.availableDrivers = driversResult.data.map((d) => d.value);
    }

    document.getElementById("storageConfigContainer").innerHTML = "";
    document.getElementById("storagePrefix").focus();
  } catch (error) {
    console.error("显示添加存储模态框失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 隐藏添加存储模态框
 */
function hideAddStorageModal() {
  try {
    document.getElementById("addStorageModal").style.display = "none";
  } catch (error) {
    console.error("隐藏添加存储模态框失败:", error);
  }
}

/**
 * 驱动变更时加载配置模板
 */
async function onDriverChange() {
  const driver = document.getElementById("storageDriver").value;
  if (!driver) {
    document.getElementById("storageConfigContainer").innerHTML = "";
    return;
  }

  try {
    const result = await getStorageTemplate(driver);
    if (result.code === 200 && result.data && result.data.template) {
      const template = result.data.template;
      const configHtml = generateConfigForm(template, driver);
      document.getElementById("storageConfigContainer").innerHTML = configHtml;
    } else {
      document.getElementById("storageConfigContainer").innerHTML =
        '<p style="color: var(--text-secondary);">无法加载配置模板</p>';
    }
  } catch (error) {
    console.error("加载配置模板失败:", error);
    document.getElementById("storageConfigContainer").innerHTML =
      '<p style="color: var(--text-secondary);">加载失败</p>';
  }
}

/**
 * 根据模板生成配置表单（支持嵌套结构）
 */
function generateConfigForm(template, driver, parentKey = "") {
  let html = "";
  for (const [key, value] of Object.entries(template)) {
    const fullKey = parentKey ? `${parentKey}.${key}` : key;
    const labelText = key
      .replace(/_/g, " ")
      .replace(/\b\w/g, (l) => l.toUpperCase());

    // 处理嵌套对象
    if (typeof value === "object" && value !== null && !Array.isArray(value)) {
      html += `
        <div class="form-group" style="margin-bottom: 16px; padding: 12px; border: 1px solid var(--border-color); border-radius: 6px; background: var(--bg-secondary);">
          <label style="font-size: 14px; font-weight: 600; color: var(--text-primary); margin-bottom: 12px; display: block;">${labelText}</label>
          ${generateConfigForm(value, driver, fullKey)}
        </div>
      `;
    } else {
      const inputType =
        typeof value === "string"
          ? "text"
          : typeof value === "number"
            ? "number"
            : "text";
      const placeholder =
        typeof value === "string" ? value : JSON.stringify(value);

      html += `
        <div class="form-group" style="margin-bottom: 12px;">
          <label style="font-size: 13px; color: var(--text-secondary);">${labelText}</label>
          <input
            type="${inputType}"
            id="config_${fullKey.replace(/\./g, "_")}"
            data-key="${fullKey}"
            placeholder="${placeholder}"
            style="width: 100%; padding: 8px 12px; border: 1px solid var(--border-color); border-radius: 6px; background: var(--bg-primary); color: var(--text-primary); font-size: 14px;"
          />
        </div>
      `;
    }
  }
  return html;
}

/**
 * 确认添加存储
 */
async function confirmAddStorage() {
  try {
    const prefix = document.getElementById("storagePrefix").value.trim();
    const driver = document.getElementById("storageDriver").value;
    const isPublic = document.getElementById("storagePublic").checked;

    if (!prefix) {
      showToast("请输入存储前缀", "error");
      return;
    }

    if (!driver) {
      showToast("请选择存储驱动", "error");
      return;
    }

    // 收集配置（支持嵌套）
    const configInputs = document.querySelectorAll(
      "#storageConfigContainer input[data-key]",
    );
    const flatConfig = {};
    configInputs.forEach((input) => {
      const key = input.getAttribute("data-key");
      const value = input.value.trim();
      if (value) {
        flatConfig[key] = value;
      }
    });

    // 将扁平配置转换为嵌套对象
    const config = {};
    for (const [key, value] of Object.entries(flatConfig)) {
      const parts = key.split(".");
      let current = config;
      for (let i = 0; i < parts.length - 1; i++) {
        const part = parts[i];
        if (!(part in current)) {
          current[part] = {};
        }
        current = current[part];
      }
      current[parts[parts.length - 1]] = value;
    }

    // 构建驱动配置对象（动态转换：mcloud -> Mcloud, mcloud_partial -> McloudPartial）
    const pascalCaseDriver = driver
      .split("_")
      .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
      .join("");
    const driverConfig = { [pascalCaseDriver]: config };

    const result = await addStorage(prefix, driverConfig, isPublic);
    if (result.code === 200) {
      hideAddStorageModal();
      showToast("存储添加成功", "success");
      loadStorages();
    } else {
      showToast("添加失败：" + result.message, "error");
    }
  } catch (error) {
    console.error("添加存储失败:", error);
    showToast("添加失败：" + error.message, "error");
  }
}

/**
 * 显示编辑权限模态框
 * @param {string} username - 用户名
 */
async function showEditPermissionsModal(username) {
  try {
    const result = await listUsers();
    if (result.code !== 200 || !result.data) {
      showToast("获取用户列表失败", "error");
      return;
    }

    const user = result.data.find((u) => u.username === username);
    if (!user) {
      showToast("用户不存在", "error");
      return;
    }

    document.getElementById("editUsername").value = username;
    document.getElementById("editRootDir").value = user.root_dir || "";
    document.getElementById("editPermRead").checked = user.permissions.read;
    document.getElementById("editPermDownload").checked =
      user.permissions.download;
    document.getElementById("editPermUpload").checked = user.permissions.upload;
    document.getElementById("editPermDelete").checked = user.permissions.delete;
    document.getElementById("editPermMove").checked = user.permissions.move_obj;
    document.getElementById("editPermCopy").checked = user.permissions.copy;
    document.getElementById("editPermCreateDir").checked =
      user.permissions.create_dir;
    document.getElementById("editPermList").checked = user.permissions.list;

    document.getElementById("editPermissionsModal").style.display = "flex";
  } catch (error) {
    console.error("显示编辑权限模态框失败:", error);
    showToast("操作失败：" + error.message, "error");
  }
}

/**
 * 隐藏编辑权限模态框
 */
function hideEditPermissionsModal() {
  try {
    document.getElementById("editPermissionsModal").style.display = "none";
  } catch (error) {
    console.error("隐藏编辑权限模态框失败:", error);
  }
}

/**
 * 确认编辑权限
 */
async function confirmEditPermissions() {
  try {
    const username = document.getElementById("editUsername").value.trim();
    const rootDir = document.getElementById("editRootDir").value.trim();

    if (!username) {
      showToast("用户名无效", "error");
      return;
    }

    // 验证根目录格式
    if (rootDir && !rootDir.startsWith("/")) {
      showToast("根目录必须以 / 开头", "error");
      return;
    }

    const permissions = {
      read: document.getElementById("editPermRead").checked,
      download: document.getElementById("editPermDownload").checked,
      upload: document.getElementById("editPermUpload").checked,
      delete: document.getElementById("editPermDelete").checked,
      move_obj: document.getElementById("editPermMove").checked,
      copy: document.getElementById("editPermCopy").checked,
      create_dir: document.getElementById("editPermCreateDir").checked,
      list: document.getElementById("editPermList").checked,
    };

    // 更新权限
    const permResult = await updatePermissions(username, permissions);
    if (permResult.code !== 200) {
      showToast("权限更新失败：" + permResult.message, "error");
      return;
    }

    // 更新根目录
    const rootDirResult = await updateRootDir(username, rootDir || null);
    if (rootDirResult.code !== 200) {
      showToast("根目录更新失败：" + rootDirResult.message, "error");
      return;
    }

    hideEditPermissionsModal();
    showToast("权限和根目录更新成功", "success");
    loadUsers();
  } catch (error) {
    console.error("更新权限失败:", error);
    showToast("更新失败：" + error.message, "error");
  }
}

/**
 * 确认删除存储
 * @param {string} name - 存储名称
 * @param {string} type - 存储类型 (public/private)
 * @param {number} index - 存储索引
 */
async function confirmDeleteStorage(name, type, index) {
  try {
    if (!confirm(`确定要删除存储 "${name}" 吗？此操作不可撤销！`)) {
      return;
    }

    const endpoint =
      type === "public"
        ? `/admin/storage/pub/delete/${index}`
        : `/admin/storage/private/delete/${index}`;

    const result = await apiRequest(endpoint, {
      method: "DELETE",
    });

    if (result.code === 200) {
      showToast("存储删除成功", "success");
      loadStorages();
    } else {
      showToast("删除失败：" + result.message, "error");
    }
  } catch (error) {
    console.error("删除存储失败:", error);
    showToast("删除失败：" + error.message, "error");
  }
}

/**
 * 退出登录
 */
function logout() {
  try {
    localStorage.removeItem("rlist_auth_token");
    localStorage.removeItem("rlist_current_user");
    window.location.href = "/index.html";
  } catch (error) {
    console.error("退出登录失败:", error);
  }
}

/**
 * 键盘事件
 */
document.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    if (document.getElementById("addUserModal").style.display === "flex") {
      confirmAddUser();
    } else if (
      document.getElementById("addStorageModal").style.display === "flex"
    ) {
      confirmAddStorage();
    } else if (
      document.getElementById("editPermissionsModal").style.display === "flex"
    ) {
      confirmEditPermissions();
    }
  } else if (e.key === "Escape") {
    hideAddUserModal();
    hideAddStorageModal();
    hideEditPermissionsModal();
  }
});
