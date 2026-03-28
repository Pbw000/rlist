let currentPage="users",sidebarOpen=!1;function toggleSidebar(){try{var e=document.querySelector(".admin-sidebar");(sidebarOpen=!sidebarOpen)?e.classList.add("mobile-open"):e.classList.remove("mobile-open")}catch(e){console.error("切换侧边栏失败:",e)}}function initTheme(){"dark"===(localStorage.getItem("rlist_theme")||"light")&&document.documentElement.setAttribute("data-theme","dark")}async function checkAuthAndLoad(){var e=localStorage.getItem("rlist_auth_token"),t=localStorage.getItem("rlist_current_user");e?await checkAuth()?await checkIsAdmin()?(document.getElementById("adminUsername").textContent=t||"管理员",loadUsers()):(showToast("您不是管理员，无法访问管理后台","error"),window.location.href="/index.html"):(showToast("认证失败，请重新登录","error"),window.location.href="/index.html"):(showToast("请先登录","error"),window.location.href="/index.html")}function switchPage(t){try{currentPage=t,document.querySelectorAll(".nav-item").forEach(e=>{e.dataset.page===t?e.classList.add("active"):e.classList.remove("active")});var e={users:"用户管理",storages:"存储管理"};document.getElementById("pageTitle").textContent=e[t]||"管理后台",document.getElementById("usersPage").style.display="users"===t?"block":"none",document.getElementById("storagesPage").style.display="storages"===t?"block":"none","users"===t?loadUsers():"storages"===t&&loadStorages()}catch(e){console.error("切换页面失败:",e),showToast("切换页面失败："+e.message,"error")}}async function loadUsers(){var e,t=document.getElementById("usersTableBody"),o=(t.innerHTML=`
    <tr>
      <td colspan="11" class="empty-cell">
        <div class="loading">
          <div class="spinner"></div>
          <span>加载中...</span>
        </div>
      </td>
    </tr>
  `,await listUsers());200===o.code&&o.data?0===(e=o.data).length?t.innerHTML='<tr><td colspan="11" class="empty-cell">暂无用户</td></tr>':t.innerHTML=e.map(e=>`
        <tr>
          <td><strong>${escapeHtml(e.username)}</strong></td>
          <td><span class="root-dir-cell">${escapeHtml(e.root_dir||"无限制")}</span></td>
          <td>${renderPermissionBadge(e.permissions.read)}</td>
          <td>${renderPermissionBadge(e.permissions.download)}</td>
          <td>${renderPermissionBadge(e.permissions.upload)}</td>
          <td>${renderPermissionBadge(e.permissions.delete)}</td>
          <td>${renderPermissionBadge(e.permissions.move_obj)}</td>
          <td>${renderPermissionBadge(e.permissions.copy)}</td>
          <td>${renderPermissionBadge(e.permissions.create_dir)}</td>
          <td>${renderPermissionBadge(e.permissions.list)}</td>
          <td>
            ${"admin"!==e.username?`<div class="action-buttons">
                    <button class="action-btn-sm edit" onclick="showEditPermissionsModal('${escapeHtml(e.username)}')" title="编辑权限">
                      <i class="ti ti-edit"></i>
                      <span>编辑</span>
                    </button>
                    <button class="action-btn-sm delete" onclick="confirmDeleteUser('${escapeHtml(e.username)}')" title="删除用户">
                      <i class="ti ti-trash"></i>
                      <span>删除</span>
                    </button>
                  </div>`:'<span style="color: var(--text-secondary); font-size: 12px;">不可修改</span>'}
          </td>
        </tr>
      `).join(""):t.innerHTML=`<tr><td colspan="11" class="empty-cell">加载失败：${o.message||"未知错误"}</td></tr>`}function renderPermissionBadge(e){return e?'<span class="permission-badge enabled"><i class="ti ti-check"></i></span>':'<span class="permission-badge disabled"><i class="ti ti-x"></i></span>'}function showAddUserModal(){try{document.getElementById("addUserModal").style.display="flex",document.getElementById("addUsername").value="",document.getElementById("addPassword").value="",document.getElementById("addRootDir").value="",document.getElementById("permRead").checked=!0,document.getElementById("permDownload").checked=!0,document.getElementById("permUpload").checked=!0,document.getElementById("permDelete").checked=!1,document.getElementById("permMove").checked=!1,document.getElementById("permCopy").checked=!1,document.getElementById("permCreateDir").checked=!0,document.getElementById("permList").checked=!0,document.getElementById("addUsername").focus()}catch(e){console.error("显示添加用户模态框失败:",e),showToast("操作失败："+e.message,"error")}}function hideAddUserModal(){try{document.getElementById("addUserModal").style.display="none"}catch(e){console.error("隐藏添加用户模态框失败:",e)}}async function confirmAddUser(){try{var e,t,o,r,d,s=document.getElementById("addUsername").value.trim(),a=document.getElementById("addPassword").value;s?a.length<8?showToast("密码长度至少为 8 位","error"):(e=/[a-zA-Z]/.test(a),t=/[0-9]/.test(a),e&&t?(o={read:document.getElementById("permRead").checked,download:document.getElementById("permDownload").checked,upload:document.getElementById("permUpload").checked,delete:document.getElementById("permDelete").checked,move_obj:document.getElementById("permMove").checked,copy:document.getElementById("permCopy").checked,create_dir:document.getElementById("permCreateDir").checked,list:document.getElementById("permList").checked},r=document.getElementById("addRootDir").value.trim(),200===(d=await addUser(s,a,o,r||null)).code?(hideAddUserModal(),showToast("用户添加成功","success"),loadUsers()):showToast("添加失败："+d.message,"error")):showToast("密码必须包含字母和数字","error")):showToast("请输入用户名","error")}catch(e){console.error("添加用户失败:",e),showToast("添加失败："+e.message,"error")}}async function confirmDeleteUser(e){try{var t;confirm(`确定要删除用户 "${e}" 吗？此操作不可撤销！`)&&(200===(t=await removeUser(e)).code?(showToast("用户删除成功","success"),loadUsers()):showToast("删除失败："+t.message,"error"))}catch(e){console.error("删除用户失败:",e),showToast("删除失败："+e.message,"error")}}async function loadStorages(){var e,t=document.getElementById("storagesTableBody"),o=(t.innerHTML=`
    <tr>
      <td colspan="5" class="empty-cell">
        <div class="loading">
          <div class="spinner"></div>
          <span>加载中...</span>
        </div>
      </td>
    </tr>
  `,await listStorages());200===o.code&&o.data?0===(e=[...((e=o.data).public||[]).map(e=>({...e,type:"public"})),...(e.private||[]).map(e=>({...e,type:"private"}))]).length?t.innerHTML='<tr><td colspan="5" class="empty-cell">暂无存储</td></tr>':t.innerHTML=e.map(e=>`
        <tr>
          <td><strong>${escapeHtml(e.name)}</strong></td>
          <td>${escapeHtml(e.driver_name)}</td>
          <td><code style="font-size: 12px; color: var(--text-secondary);">${escapeHtml(e.path)}</code></td>
          <td>
            <span class="storage-type-badge ${e.type}">${"public"===e.type?"公开":"私有"}</span>
            <span class="storage-status work"><i class="ti ti-check"></i> 正常</span>
          </td>
          <td>
            ${"default"!==e.name?`<button class="action-btn-sm delete" onclick="confirmDeleteStorage('${escapeHtml(e.name)}', '${e.type}', ${e.idx})" title="删除存储">
                    <i class="ti ti-trash"></i>
                    <span>删除</span>
                  </button>`:'<span style="color: var(--text-secondary); font-size: 12px;">默认存储</span>'}
          </td>
        </tr>
      `).join(""):t.innerHTML=`<tr><td colspan="5" class="empty-cell">加载失败：${o.message||"未知错误"}</td></tr>`}async function showAddStorageModal(){try{document.getElementById("addStorageModal").style.display="flex",document.getElementById("storagePrefix").value="",document.getElementById("storagePublic").checked=!1;var e=await getStorageDrivers();let o=document.getElementById("storageDriver");o.innerHTML='<option value="">请选择驱动</option>',200===e.code&&e.data&&(e.data.forEach(e=>{var t=document.createElement("option");t.value=e.value,t.textContent=e.label,o.appendChild(t)}),window.availableDrivers=e.data.map(e=>e.value)),document.getElementById("storageConfigContainer").innerHTML="",document.getElementById("storagePrefix").focus()}catch(e){console.error("显示添加存储模态框失败:",e),showToast("操作失败："+e.message,"error")}}function hideAddStorageModal(){try{document.getElementById("addStorageModal").style.display="none"}catch(e){console.error("隐藏添加存储模态框失败:",e)}}async function onDriverChange(){var e=document.getElementById("storageDriver").value;if(e)try{var t,o=await getStorageTemplate(e);200===o.code&&o.data&&o.data.template?(t=generateConfigForm(o.data.template,e),document.getElementById("storageConfigContainer").innerHTML=t):document.getElementById("storageConfigContainer").innerHTML='<p style="color: var(--text-secondary);">无法加载配置模板</p>'}catch(e){console.error("加载配置模板失败:",e),document.getElementById("storageConfigContainer").innerHTML='<p style="color: var(--text-secondary);">加载失败</p>'}else document.getElementById("storageConfigContainer").innerHTML=""}function generateConfigForm(e,t,o=""){let r="";for(var[d,s]of Object.entries(e)){var a,n,i=o?o+"."+d:d,d=d.replace(/_/g," ").replace(/\b\w/g,e=>e.toUpperCase());"object"!=typeof s||null===s||Array.isArray(s)?(a="string"!=typeof s&&"number"==typeof s?"number":"text",n="string"==typeof s?s:JSON.stringify(s),r+=`
        <div class="form-group" style="margin-bottom: 12px;">
          <label style="font-size: 13px; color: var(--text-secondary);">${d}</label>
          <input
            type="${a}"
            id="config_${i.replace(/\./g,"_")}"
            data-key="${i}"
            placeholder="${n}"
            style="width: 100%; padding: 8px 12px; border: 1px solid var(--border-color); border-radius: 6px; background: var(--bg-primary); color: var(--text-primary); font-size: 14px;"
          />
        </div>
      `):r+=`
        <div class="form-group" style="margin-bottom: 16px; padding: 12px; border: 1px solid var(--border-color); border-radius: 6px; background: var(--bg-secondary);">
          <label style="font-size: 14px; font-weight: 600; color: var(--text-primary); margin-bottom: 12px; display: block;">${d}</label>
          ${generateConfigForm(s,t,i)}
        </div>
      `}return r}async function confirmAddStorage(){try{var e=document.getElementById("storagePrefix").value.trim(),t=document.getElementById("storageDriver").value,r=document.getElementById("storagePublic").checked;if(e)if(t){var d=document.querySelectorAll("#storageConfigContainer input[data-key]");let o={};d.forEach(e=>{var t=e.getAttribute("data-key"),e=e.value.trim();e&&(o[t]=e)});var s,a,n={};for([s,a]of Object.entries(o)){var i=s.split(".");let t=n;for(let e=0;e<i.length-1;e++){var c=i[e];c in t||(t[c]={}),t=t[c]}t[i[i.length-1]]=a}var l=t.split("_").map(e=>e.charAt(0).toUpperCase()+e.slice(1)).join(""),m=await addStorage(e,{[l]:n},r);200===m.code?(hideAddStorageModal(),showToast("存储添加成功","success"),loadStorages()):showToast("添加失败："+m.message,"error")}else showToast("请选择存储驱动","error");else showToast("请输入存储前缀","error")}catch(e){console.error("添加存储失败:",e),showToast("添加失败："+e.message,"error")}}async function showEditPermissionsModal(t){try{var e,o=await listUsers();200===o.code&&o.data?(e=o.data.find(e=>e.username===t))?(document.getElementById("editUsername").value=t,document.getElementById("editRootDir").value=e.root_dir||"",document.getElementById("editPermRead").checked=e.permissions.read,document.getElementById("editPermDownload").checked=e.permissions.download,document.getElementById("editPermUpload").checked=e.permissions.upload,document.getElementById("editPermDelete").checked=e.permissions.delete,document.getElementById("editPermMove").checked=e.permissions.move_obj,document.getElementById("editPermCopy").checked=e.permissions.copy,document.getElementById("editPermCreateDir").checked=e.permissions.create_dir,document.getElementById("editPermList").checked=e.permissions.list,document.getElementById("editPermissionsModal").style.display="flex"):showToast("用户不存在","error"):showToast("获取用户列表失败","error")}catch(e){console.error("显示编辑权限模态框失败:",e),showToast("操作失败："+e.message,"error")}}function hideEditPermissionsModal(){try{document.getElementById("editPermissionsModal").style.display="none"}catch(e){console.error("隐藏编辑权限模态框失败:",e)}}async function confirmEditPermissions(){try{var e,t,o,r=document.getElementById("editUsername").value.trim(),d=document.getElementById("editRootDir").value.trim();r?d&&!d.startsWith("/")?showToast("根目录必须以 / 开头","error"):(e={read:document.getElementById("editPermRead").checked,download:document.getElementById("editPermDownload").checked,upload:document.getElementById("editPermUpload").checked,delete:document.getElementById("editPermDelete").checked,move_obj:document.getElementById("editPermMove").checked,copy:document.getElementById("editPermCopy").checked,create_dir:document.getElementById("editPermCreateDir").checked,list:document.getElementById("editPermList").checked},200!==(t=await updatePermissions(r,e)).code?showToast("权限更新失败："+t.message,"error"):200!==(o=await updateRootDir(r,d||null)).code?showToast("根目录更新失败："+o.message,"error"):(hideEditPermissionsModal(),showToast("权限和根目录更新成功","success"),loadUsers())):showToast("用户名无效","error")}catch(e){console.error("更新权限失败:",e),showToast("更新失败："+e.message,"error")}}async function confirmDeleteStorage(e,t,o){try{var r;confirm(`确定要删除存储 "${e}" 吗？此操作不可撤销！`)&&(200===(r=await apiRequest("public"===t?"/admin/storage/pub/delete/"+o:"/admin/storage/private/delete/"+o,{method:"DELETE"})).code?(showToast("存储删除成功","success"),loadStorages()):showToast("删除失败："+r.message,"error"))}catch(e){console.error("删除存储失败:",e),showToast("删除失败："+e.message,"error")}}function logout(){try{localStorage.removeItem("rlist_auth_token"),localStorage.removeItem("rlist_current_user"),window.location.href="/index.html"}catch(e){console.error("退出登录失败:",e)}}document.addEventListener("DOMContentLoaded",()=>{checkAuthAndLoad(),initTheme(),window.addEventListener("error",e=>{console.error("全局错误:",e.error)}),window.addEventListener("unhandledrejection",e=>{console.error("未处理的 Promise rejection:",e.reason)})}),document.addEventListener("click",e=>{var t=document.querySelector(".admin-sidebar"),o=document.querySelector(".menu-toggle");!sidebarOpen||t.contains(e.target)||o.contains(e.target)||(sidebarOpen=!1,t.classList.remove("mobile-open"))}),document.addEventListener("keydown",e=>{"Enter"===e.key?"flex"===document.getElementById("addUserModal").style.display?confirmAddUser():"flex"===document.getElementById("addStorageModal").style.display?confirmAddStorage():"flex"===document.getElementById("editPermissionsModal").style.display&&confirmEditPermissions():"Escape"===e.key&&(hideAddUserModal(),hideAddStorageModal(),hideEditPermissionsModal())});