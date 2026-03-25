/**
 * 管理后台逻辑
 */

// 全局变量
let currentPage = "users";

// 初始化
document.addEventListener("DOMContentLoaded", () => {
  checkAuthAndLoad();
  initTheme();
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
  document.getElementById("pageTitle").textContent = titles[page] || "管理后台";

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
}

/**
 * 加载用户列表
 */
async function loadUsers() {
  const tbody = document.getElementById("usersTableBody");
  tbody.innerHTML = `
    <tr>
      <td colspan="10" class="empty-cell">
        <div class="loading">
          <div class="spinner"></div>
          <span>加载中...</span>
        </div>
      </td>
    </tr>
  `;

  const result = await listUsers();
  if (result.code === 200 && result.data) {
    const users = result.data;
    if (users.length === 0) {
      tbody.innerHTML =
        '<tr><td colspan="10" class="empty-cell">暂无用户</td></tr>';
      return;
    }

    tbody.innerHTML = users
      .map(
        (user) => `
        <tr>
          <td><strong>${escapeHtml(user.username)}</strong></td>
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
    tbody.innerHTML = `<tr><td colspan="10" class="empty-cell">加载失败：${result.message || "未知错误"}</td></tr>`;
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
  document.getElementById("addUserModal").style.display = "flex";
  document.getElementById("addUsername").value = "";
  document.getElementById("addPassword").value = "";
  document.getElementById("permRead").checked = true;
  document.getElementById("permDownload").checked = true;
  document.getElementById("permUpload").checked = true;
  document.getElementById("permDelete").checked = false;
  document.getElementById("permMove").checked = false;
  document.getElementById("permCopy").checked = false;
  document.getElementById("permCreateDir").checked = true;
  document.getElementById("permList").checked = true;
  document.getElementById("addUsername").focus();
}

/**
 * 隐藏添加用户模态框
 */
function hideAddUserModal() {
  document.getElementById("addUserModal").style.display = "none";
}

/**
 * 确认添加用户
 */
async function confirmAddUser() {
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

  const result = await addUser(username, password, permissions);
  if (result.code === 200) {
    hideAddUserModal();
    showToast("用户添加成功", "success");
    loadUsers();
  } else {
    showToast("添加失败：" + result.message, "error");
  }
}

/**
 * 确认删除用户
 * @param {string} username - 用户名
 */
async function confirmDeleteUser(username) {
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
}

/**
 * 加载存储列表
 */
async function loadStorages() {
  const tbody = document.getElementById("storagesTableBody");
  tbody.innerHTML = `
    <tr>
      <td colspan="4" class="empty-cell">
        <div class="loading">
          <div class="spinner"></div>
          <span>加载中...</span>
        </div>
      </td>
    </tr>
  `;

  const result = await apiRequest("/admin/storage/list", {
    method: "GET",
  });

  if (result.code === 200 && result.data) {
    const storages = result.data;
    // 合并公开和私有存储
    const allStorages = [
      ...(storages.public || []).map((s) => ({ ...s, type: "public" })),
      ...(storages.private || []).map((s) => ({ ...s, type: "private" })),
    ];

    if (allStorages.length === 0) {
      tbody.innerHTML =
        '<tr><td colspan="4" class="empty-cell">暂无存储</td></tr>';
      return;
    }

    tbody.innerHTML = allStorages
      .map(
        (storage, index) => `
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
                ? `<button class="action-btn-sm delete" onclick="confirmDeleteStorage('${escapeHtml(storage.name)}', '${storage.type}', ${index})" title="删除存储">
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
    tbody.innerHTML = `<tr><td colspan="4" class="empty-cell">加载失败：${result.message || "未知错误"}</td></tr>`;
  }
}

/**
 * 显示添加存储模态框
 */
function showAddStorageModal() {
  document.getElementById("addStorageModal").style.display = "flex";
  document.getElementById("storageName").value = "";
  document.getElementById("storageDriver").value = "local";
  document.getElementById("storagePath").value = "";
  document.getElementById("storageName").focus();
}

/**
 * 隐藏添加存储模态框
 */
function hideAddStorageModal() {
  document.getElementById("addStorageModal").style.display = "none";
}

/**
 * 确认添加存储
 */
async function confirmAddStorage() {
  const name = document.getElementById("storageName").value.trim();
  const driver = document.getElementById("storageDriver").value;
  const path = document.getElementById("storagePath").value.trim();

  if (!name) {
    showToast("请输入存储名称", "error");
    return;
  }

  if (!path) {
    showToast("请输入存储路径", "error");
    return;
  }

  // TODO: 实现添加存储 API
  showToast("添加存储功能开发中", "error");
  hideAddStorageModal();
}

/**
 * 显示编辑权限模态框
 * @param {string} username - 用户名
 */
async function showEditPermissionsModal(username) {
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
}

/**
 * 隐藏编辑权限模态框
 */
function hideEditPermissionsModal() {
  document.getElementById("editPermissionsModal").style.display = "none";
}

/**
 * 确认编辑权限
 */
async function confirmEditPermissions() {
  const username = document.getElementById("editUsername").value.trim();

  if (!username) {
    showToast("用户名无效", "error");
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

  const result = await updatePermissions(username, permissions);
  if (result.code === 200) {
    hideEditPermissionsModal();
    showToast("权限更新成功", "success");
    loadUsers();
  } else {
    showToast("更新失败：" + result.message, "error");
  }
}

/**
 * 确认删除存储
 * @param {string} name - 存储名称
 * @param {string} type - 存储类型 (public/private)
 * @param {number} index - 存储索引
 */
async function confirmDeleteStorage(name, type, index) {
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
}

/**
 * 退出登录
 */
function logout() {
  localStorage.removeItem("rlist_auth_token");
  localStorage.removeItem("rlist_current_user");
  window.location.href = "/index.html";
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
