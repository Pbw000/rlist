let fileManager=null,uploadManager=null,currentView=localStorage.getItem("rlist_view")||"list",previewFilePath="",contextMenuTarget=null,selectedPathForAction=null,isPublicStorageMode=!1,currentStoragePath="/",pathHistory=[],pathHistoryIndex=-1;function initTheme(){var e;"dark"===(localStorage.getItem("rlist_theme")||"light")&&(document.documentElement.setAttribute("data-theme","dark"),e=document.getElementById("themeIcon"))&&(e.className="ti ti-sun")}function toggleTheme(){var e="dark"===document.documentElement.getAttribute("data-theme"),t=document.getElementById("themeIcon");e?(document.documentElement.removeAttribute("data-theme"),t&&(t.className="ti ti-moon"),localStorage.setItem("rlist_theme","light")):(document.documentElement.setAttribute("data-theme","dark"),t&&(t.className="ti ti-sun"),localStorage.setItem("rlist_theme","dark"))}async function checkAuthAndLoad(){var e=localStorage.getItem("rlist_auth_token"),t=localStorage.getItem("rlist_current_user");e&&await checkAuth()?showMainInterface(t):showLoginInterface()}function showMainInterface(e){document.getElementById("authContainer").style.display="none",document.getElementById("mainContainer").style.display="block",document.getElementById("userInfo").style.display="flex",document.getElementById("usernameDisplay").textContent=e;e=document.getElementById("storageSwitch");e&&(e.style.display="flex");var e=new URLSearchParams(window.location.search).get("path");pathHistoryIndex=(pathHistory=e?(e=decodeURIComponent(e),isPublicStorageMode?currentStoragePath=e:fileManager.currentPath=e,[e]):["/"],0),setView(currentView),loadCurrentStorageFiles(),updateNavButtons()}function showLoginInterface(){document.getElementById("authContainer").style.display="flex",document.getElementById("mainContainer").style.display="none"}async function handleLogin(){try{let e=document.getElementById("loginUsername").value.trim();var t,o=document.getElementById("loginPassword").value,a=document.getElementById("authMessage");e&&o?(a.textContent="登录中...",a.className="",(t=await login(e,o)).success?(a.textContent="登录成功，正在跳转...",a.className="success",setTimeout(()=>{showMainInterface(e)},1e3)):(a.textContent=t.message,a.className="error")):(a.textContent="请输入用户名和密码",a.className="error")}catch(e){console.error("登录失败:",e);o=document.getElementById("authMessage");o.textContent="登录失败："+e.message,o.className="error"}}function toggleStorageMode(e){isPublicStorageMode=e,currentStoragePath="/",loadCurrentStorageFiles()}async function loadPublicStorageFiles(){var t=document.getElementById("fileList");if(t)try{t.innerHTML='<div class="loading"><div class="spinner"></div></div>';var e,o={path:currentStoragePath},a=await buildPublicRequest(o),r=await(await fetch("/obs/list",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(a)})).json();200===r.code&&r.data?(e=(r.data.items||[]).map(e=>{var t=void 0===e.File;return{name:e.File?.name||e.Directory?.name||"unknown",path:currentStoragePath.endsWith("/")?currentStoragePath+(e.File?.name||e.Directory?.name||""):currentStoragePath+"/"+(e.File?.name||e.Directory?.name||""),size:e.File?.size||0,file_type:t?"dir":"file",modified:e.File?.modified_at||e.Directory?.modified_at}}),fileManager.filesData=e,fileManager.selectedFiles.clear(),renderFiles(e),updateBreadcrumb(),updateDeleteButton()):(t.innerHTML='<div class="empty-state"><i class="ti ti-folder-x"></i><p>加载失败</p></div>',showToast("加载文件列表失败："+r.message,"error"))}catch(e){console.error("加载公开存储文件列表失败:",e),t.innerHTML='<div class="empty-state"><i class="ti ti-wifi-off"></i><p>无法连接到服务器</p></div>',showToast("网络错误："+e.message,"error")}}async function showPublicStorage(){try{var e,t,o=document.getElementById("publicStorageModal"),a=document.getElementById("publicStorageContent"),r=(o.style.display="flex",a.innerHTML='<div class="loading"><div class="spinner"></div></div>',await fetch("/api/admin/storage/list",{headers:getAuthHeaders()}));403===r.status?a.innerHTML=`
        <div class="empty-state">
          <i class="ti ti-lock"></i>
          <p>只有管理员可以查看存储列表</p>
          <p style="margin-top: 8px; font-size: 13px; color: var(--text-secondary);">您可以直接使用顶部的公开/私有存储切换开关</p>
        </div>
      `:r.ok&&200===(e=await r.json()).code&&e.data?0===(t=e.data.public||[]).length?a.innerHTML=`
            <div class="empty-state">
              <i class="ti ti-folder-open"></i>
              <p>暂无公开存储</p>
            </div>
          `:a.innerHTML=`
            <div class="storage-list">
              ${t.map(e=>`
                <div class="storage-item" onclick="openPublicStorage('${escapeHtml(e.path)}', '${escapeHtml(e.name)}')">
                  <i class="ti ti-world"></i>
                  <div class="storage-item-info">
                    <div class="storage-item-name">${escapeHtml(e.name)}</div>
                    <div class="storage-item-path">${escapeHtml(e.path)}</div>
                  </div>
                  <i class="ti ti-chevron-right"></i>
                </div>
              `).join("")}
            </div>
          `:a.innerHTML='<div class="empty-state"><i class="ti ti-alert-circle"></i><p>获取公开存储列表失败</p></div>'}catch(e){console.error("显示公开存储列表失败:",e),document.getElementById("publicStorageContent").innerHTML=`<div class="empty-state"><i class="ti ti-alert-circle"></i><p>网络错误：${escapeHtml(e.message)}</p></div>`}}function hidePublicStorageModal(){try{document.getElementById("publicStorageModal").style.display="none"}catch(e){console.error("隐藏公开存储模态框失败:",e)}}function openPublicStorage(e,t){try{isPublicStorageMode=!0,currentStoragePath=e;var o=document.getElementById("storageSwitchToggle");o&&(o.checked=!0),hidePublicStorageModal(),loadCurrentStorageFiles(),showToast("已切换到公开存储："+t,"success")}catch(e){console.error("打开公开存储失败:",e),showToast("操作失败："+e.message,"error")}}function navigateTo(e,t=!0){try{var o,a=e||"/";isPublicStorageMode?currentStoragePath=a:fileManager.navigateTo(a),t&&((pathHistory=pathHistory.slice(0,pathHistoryIndex+1)).push(a),pathHistoryIndex++,o=buildUrlWithPath(a),history.pushState({path:a},"",o)),resetScrollListener(),updateBreadcrumb(),loadCurrentStorageFiles(),updateNavButtons()}catch(e){console.error("导航失败:",e),showToast("导航失败："+e.message,"error")}}function buildUrlWithPath(e){var t=new URL(window.location.href);return"/"!==e?t.searchParams.set("path",encodeURIComponent(e)):t.searchParams.delete("path"),t.toString()}function updateNavButtons(){var e=document.getElementById("backBtn"),t=document.getElementById("forwardBtn");e&&(e.disabled=pathHistoryIndex<=0),t&&(t.disabled=pathHistoryIndex>=pathHistory.length-1)}function goBack(){try{0<pathHistoryIndex&&(pathHistoryIndex--,pathHistory[pathHistoryIndex],history.back())}catch(e){console.error("后退失败:",e),showToast("操作失败："+e.message,"error")}}function goForward(){try{pathHistoryIndex<pathHistory.length-1&&(pathHistoryIndex++,pathHistory[pathHistoryIndex],history.forward())}catch(e){console.error("前进失败:",e),showToast("操作失败："+e.message,"error")}}function togglePathInput(){try{var e,t=document.getElementById("breadcrumb"),o=document.getElementById("pathInputWrapper"),a=document.getElementById("pathInput");"none"===t.style.display?(t.style.display="flex",o.style.display="none"):(t.style.display="none",o.style.display="flex",e=isPublicStorageMode?currentStoragePath:fileManager?.currentPath||"/",a.value=e,a.focus(),a.select())}catch(e){console.error("切换路径输入框失败:",e),showToast("操作失败："+e.message,"error")}}function confirmPathInput(){try{var e=document.getElementById("pathInput").value.trim();e&&navigateTo(normalizePath(e)),togglePathInput()}catch(e){console.error("确认路径输入失败:",e),showToast("路径无效："+e.message,"error")}}function cancelPathInput(){try{togglePathInput()}catch(e){console.error("取消路径输入失败:",e),showToast("操作失败："+e.message,"error")}}function normalizePath(e){var t,o=[];for(t of(e=e.startsWith("/")?e:"/"+e).split("/").filter(e=>e&&"."!==e))".."===t?0<o.length&&o.pop():o.push(t);return"/"+o.join("/")}function loadCurrentStorageFiles(){try{isPublicStorageMode?loadPublicStorageFiles():fileManager.loadFiles(fileManager.currentPath)}catch(e){console.error("加载文件列表失败:",e),showToast("加载文件列表失败："+e.message,"error")}}function refresh(){try{loadCurrentStorageFiles(),showToast("刷新成功","success")}catch(e){console.error("刷新失败:",e),showToast("刷新失败："+e.message,"error")}}function handleSearch(t){try{fileManager&&(isPublicStorageMode?renderFiles((fileManager.filesData||[]).filter(e=>e.name.toLowerCase().includes(t.toLowerCase()))):renderFiles(fileManager.search(t)))}catch(e){console.error("搜索文件失败:",e),showToast("搜索失败："+e.message,"error")}}function updateBreadcrumb(){try{var e=document.getElementById("breadcrumb");if(e){let o=(isPublicStorageMode?currentStoragePath:fileManager?.currentPath||"/").split("/").filter(e=>e),a='<a href="#" onclick="navigateTo(\'/\'); return false;" class="breadcrumb-item"><i class="ti ti-home"></i><span>首页</span></a>',r="";o.forEach((e,t)=>{r+="/"+e;t=t===o.length-1;a=(a+='<span class="breadcrumb-separator">/</span>')+(t?`<a href="#" onclick="navigateTo('${escapeHtml(r)}'); return false;" class="breadcrumb-item active">
        <span>${escapeHtml(e)}</span>
      </a>`:`<a href="#" onclick="navigateTo('${escapeHtml(r)}'); return false;" class="breadcrumb-item">
        <span>${escapeHtml(e)}</span>
      </a>`)}),e.innerHTML=a}}catch(e){console.error("更新面包屑失败:",e),showToast("更新面包屑失败："+e.message,"error")}}function renderFiles(e,o=!1){var a=document.getElementById("fileList");if(a)try{if(e&&0!==e.length){var r,i,n=e.filter(e=>"dir"===e.file_type),s=e.filter(e=>"file"===e.file_type),l=[...n,...s];let t=!isPublicStorageMode;o?(r=`
        ${l.map(e=>`
            <div class="file-item ${fileManager.selectedFiles.has(e.path)?"selected":""}"
                 data-path="${escapeHtml(e.path)}"
                 data-type="${e.file_type}"
                 oncontextmenu="showContextMenu(event, '${escapeHtml(e.path)}', '${e.file_type}')">
                ${t?`
                <div>
                    <input type="checkbox" class="checkbox"
                           ${fileManager.selectedFiles.has(e.path)?"checked":""}
                           onchange="toggleSelection('${escapeHtml(e.path)}', this.checked, event)">
                </div>
                `:""}
                <div class="file-main" onclick="handleFileClick('${escapeHtml(e.path)}', '${e.file_type}', event)">
                    <div class="file-icon ${"dir"===e.file_type?"folder":""}">${"dir"===e.file_type?'<i class="ti ti-folder"></i>':getFileIcon(e.name)}</div>
                    <div>
                        <div class="file-name">${escapeHtml(e.name)}</div>
                        <div class="file-meta">${"file"===e.file_type?formatSize(e.size):"文件夹"}</div>
                    </div>
                </div>
                <div class="file-size">${"file"===e.file_type?formatSize(e.size):""}</div>
                <div class="file-date">${e.modified?formatDate(e.modified):""}</div>
                <div class="file-actions">
                    ${"dir"===e.file_type?`
                        <button class="action-btn" onclick="enterFolderWithAnimation('${escapeHtml(e.path)}')" title="打开">
                            <i class="ti ti-folder-open"></i>
                        </button>
                    `:`
                        <button class="action-btn" onclick="previewFile('${escapeHtml(e.path)}', '${escapeHtml(e.name)}')" title="预览">
                            <i class="ti ti-eye"></i>
                        </button>
                        <button class="action-btn" onclick="downloadFile('${escapeHtml(e.path)}')" title="下载">
                            <i class="ti ti-download"></i>
                        </button>
                    `}
                    <button class="action-btn" onclick="showContextMenuForFile('${escapeHtml(e.path)}', '${e.file_type}')" title="更多">
                        <i class="ti ti-dots"></i>
                    </button>
                </div>
            </div>
        `).join("")}
      `,(i=a.querySelector(".loading-more"))?i.insertAdjacentHTML("beforebegin",r):a.insertAdjacentHTML("beforeend",r)):a.innerHTML=`
        <div class="file-list-header">
            ${t?'<div><input type="checkbox" class="checkbox" onchange="toggleSelectAll(this.checked)"></div>':""}
            <div>名称</div>
            <div>大小</div>
            <div>修改日期</div>
            <div>操作</div>
        </div>
        ${l.map(e=>`
            <div class="file-item ${fileManager.selectedFiles.has(e.path)?"selected":""}"
                 data-path="${escapeHtml(e.path)}"
                 data-type="${e.file_type}"
                 oncontextmenu="showContextMenu(event, '${escapeHtml(e.path)}', '${e.file_type}')">
                ${t?`
                <div>
                    <input type="checkbox" class="checkbox"
                           ${fileManager.selectedFiles.has(e.path)?"checked":""}
                           onchange="toggleSelection('${escapeHtml(e.path)}', this.checked, event)">
                </div>
                `:""}
                <div class="file-main" onclick="handleFileClick('${escapeHtml(e.path)}', '${e.file_type}', event)">
                    <div class="file-icon ${"dir"===e.file_type?"folder":""}">${"dir"===e.file_type?'<i class="ti ti-folder"></i>':getFileIcon(e.name)}</div>
                    <div>
                        <div class="file-name">${escapeHtml(e.name)}</div>
                        <div class="file-meta">${"file"===e.file_type?formatSize(e.size):"文件夹"}</div>
                    </div>
                </div>
                <div class="file-size">${"file"===e.file_type?formatSize(e.size):""}</div>
                <div class="file-date">${e.modified?formatDate(e.modified):""}</div>
                <div class="file-actions">
                    ${"dir"===e.file_type?`
                        <button class="action-btn" onclick="enterFolderWithAnimation('${escapeHtml(e.path)}')" title="打开">
                            <i class="ti ti-folder-open"></i>
                        </button>
                    `:`
                        <button class="action-btn" onclick="previewFile('${escapeHtml(e.path)}', '${escapeHtml(e.name)}')" title="预览">
                            <i class="ti ti-eye"></i>
                        </button>
                        <button class="action-btn" onclick="downloadFile('${escapeHtml(e.path)}')" title="下载">
                            <i class="ti ti-download"></i>
                        </button>
                    `}
                    <button class="action-btn" onclick="showContextMenuForFile('${escapeHtml(e.path)}', '${e.file_type}')" title="更多">
                        <i class="ti ti-dots"></i>
                    </button>
                </div>
            </div>
        `).join("")}
      `,setupScrollListener()}else a.innerHTML='<div class="empty-state"><i class="ti ti-folder-open"></i><p>此目录为空</p></div>';updateDeleteButton()}catch(e){console.error("渲染文件列表失败:",e),a.innerHTML=`
      <div class="empty-state">
        <i class="ti ti-alert-circle"></i>
        <p>渲染文件列表失败</p>
        <p style="margin-top: 8px; color: var(--text-secondary); font-size: 12px;">${escapeHtml(e.message)}</p>
        <button class="btn btn-primary" onclick="loadCurrentStorageFiles()" style="margin-top: 16px;">
          <i class="ti ti-refresh"></i> 重试
        </button>
      </div>
    `,showToast("渲染文件列表失败："+e.message,"error")}}document.addEventListener("DOMContentLoaded",()=>{new URLSearchParams(window.location.search).get("path");fileManager=new FileManager({onFilesLoaded:renderFiles,onError:e=>showToast(e,"error")}),uploadManager=new UploadManager({onTaskProgress:updateUploadProgress,onAllCompleted:onUploadAllCompleted}),initTheme(),checkAuthAndLoad();var e=document.getElementById("searchInput"),e=(e&&e.addEventListener("input",e=>{handleSearch(e.target.value)}),document.addEventListener("click",e=>{e.target.closest(".context-menu")||hideContextMenu()}),window.addEventListener("popstate",e=>{e.state&&void 0!==e.state.path&&(isPublicStorageMode?currentStoragePath=e.state.path:fileManager.currentPath=e.state.path,-1!==(e=pathHistory.indexOf(e.state.path))&&(pathHistoryIndex=e),updateBreadcrumb(),loadCurrentStorageFiles(),updateNavButtons())}),document.getElementById("pathInput"));e&&e.addEventListener("keypress",e=>{"Enter"===e.key&&confirmPathInput()}),window.addEventListener("error",e=>{console.error("全局错误:",e.error)}),window.addEventListener("unhandledrejection",e=>{console.error("未处理的 Promise rejection:",e.reason)})});let scrollListenerInitialized=!1;function setupScrollListener(){var e;scrollListenerInitialized||(e=document.querySelector(".file-list-container"))&&(e.addEventListener("scroll",handleScroll),scrollListenerInitialized=!0)}function resetScrollListener(){scrollListenerInitialized=!1}function handleScroll(){var e,t,o=document.querySelector(".file-list-container");o&&({scrollTop:o,scrollHeight:e,clientHeight:t}=o,e-o-t<=100)&&(isPublicStorageMode?loadMoreFiles():fileManager&&fileManager.loadMoreFiles())}function enterFolder(e){try{navigateTo(e)}catch(e){console.error("进入文件夹失败:",e),showToast("进入文件夹失败："+e.message,"error")}}function handleDoubleClick(e,t){try{"dir"===t?enterFolder(e):previewFile(e,e.split("/").pop())}catch(e){console.error("双击处理失败:",e),showToast("操作失败："+e.message,"error")}}function handleFileClick(e,t,o){try{o.target.closest(".checkbox")||o.target.closest(".file-actions")||("dir"===t?enterFolderWithAnimation(e):isPublicStorageMode||toggleSelection(e,!fileManager.selectedFiles.has(e),o))}catch(e){console.error("处理文件点击失败:",e),showToast("操作失败："+e.message,"error")}}function enterFolderWithAnimation(t){try{let e=document.getElementById("fileList");e&&(e.classList.add("nav-entering"),navigateTo(t),setTimeout(()=>{e.classList.remove("nav-entering")},400))}catch(e){console.error("进入文件夹失败:",e),showToast("进入文件夹失败："+e.message,"error")}}function navigateUpWithAnimation(){let e=document.getElementById("fileList");e&&(e.classList.add("nav-leave"),setTimeout(()=>{navigateTo(".."),updateBreadcrumb(),e.classList.remove("nav-leave")},200))}function toggleSelection(e,t,o){try{o&&o.stopPropagation(),isPublicStorageMode?showToast("公开存储模式下不支持选择操作","error"):(fileManager.toggleSelection(e,t),updateDeleteButton(),renderFiles(fileManager.filesData))}catch(e){console.error("切换选择状态失败:",e),showToast("操作失败："+e.message,"error")}}function toggleSelectAll(e){try{isPublicStorageMode?showToast("公开存储模式下不支持选择操作","error"):(fileManager.toggleSelectAll(e),updateDeleteButton(),renderFiles(fileManager.filesData))}catch(e){console.error("全选/取消全选失败:",e),showToast("操作失败："+e.message,"error")}}function updateDeleteButton(){var e,t=document.getElementById("deleteBtn");isPublicStorageMode?t.style.display="none":(e=fileManager.getSelectedCount(),t.style.display=0<e?"flex":"none",0<e&&(t.innerHTML=`<i class="ti ti-trash"></i> 删除选中 (${e})`))}async function deleteSelected(){try{var e;isPublicStorageMode?showToast("公开存储模式下不支持删除操作","error"):0!==(e=fileManager.getSelectedCount())&&showDeleteConfirmModal(e)}catch(e){console.error("删除选中的文件失败:",e),showToast("操作失败："+e.message,"error")}}function showDeleteConfirmModal(e){try{var t=document.getElementById("deleteConfirmModal");document.getElementById("deleteMessage").textContent=`确定要删除选中的 ${e} 个项目吗？`,t.style.display="flex"}catch(e){console.error("显示删除确认弹窗失败:",e),showToast("操作失败："+e.message,"error")}}function hideDeleteConfirmModal(){try{document.getElementById("deleteConfirmModal").style.display="none"}catch(e){console.error("隐藏删除确认弹窗失败:",e)}}function handleDeleteModalOverlayClick(e){try{e.target===e.currentTarget&&hideDeleteConfirmModal()}catch(e){console.error("处理删除弹窗背景点击失败:",e)}}async function confirmDelete(){hideDeleteConfirmModal();try{var e,t;window.deleteCallback?(e=window.deleteCallback,window.deleteCallback=null,await e()):(t=await fileManager.deleteSelected(),showToast(`已删除 ${t.count}/${t.total} 个项目`,t.count===t.total?"success":"error"))}catch(e){console.error("删除失败:",e),showToast("删除失败："+e.message,"error")}}function showNewFolderModal(){try{isPublicStorageMode?showToast("公开存储模式下不支持创建文件夹","error"):(document.getElementById("newFolderModal").style.display="flex",document.getElementById("folderNameInput").value="",document.getElementById("folderNameInput").focus())}catch(e){console.error("显示新建文件夹模态框失败:",e),showToast("操作失败："+e.message,"error")}}function hideNewFolderModal(){document.getElementById("newFolderModal").style.display="none"}async function createFolder(){try{var e,t,o;isPublicStorageMode?showToast("公开存储模式下不支持创建文件夹","error"):(e=document.getElementById("folderNameInput").value.trim())?e.includes("/")||e.includes("\\")?showToast("文件夹名称不能包含斜杠","error"):((t=fileManager.currentPath).endsWith("/"),(o=await fileManager.createFolder(e)).success?(hideNewFolderModal(),showToast("创建文件夹成功","success"),loadCurrentStorageFiles()):showToast("创建文件夹失败："+o.message,"error")):showToast("请输入文件夹名称","error")}catch(e){console.error("创建文件夹失败:",e),showToast("创建文件夹失败："+e.message,"error")}}function showRenameModal(e,t){try{selectedPathForAction=e,document.getElementById("renameModal").style.display="flex",document.getElementById("renameInput").value=t,document.getElementById("renameInput").focus(),document.getElementById("renameInput").select()}catch(e){console.error("显示重命名模态框失败:",e),showToast("操作失败："+e.message,"error")}}function hideRenameModal(){try{document.getElementById("renameModal").style.display="none"}catch(e){console.error("隐藏重命名模态框失败:",e)}}async function confirmRename(){try{var e,t=document.getElementById("renameInput").value.trim();t?(e=await fileManager.rename(selectedPathForAction,t)).success?(hideRenameModal(),showToast("重命名成功","success")):showToast("重命名失败："+e.message,"error"):showToast("请输入新名称","error")}catch(e){console.error("重命名失败:",e),showToast("重命名失败："+e.message,"error")}}let previewZoom=1,previewRotation=0,previewPdfDoc=null,previewFileType="",previewFileName="",previewXhr=null;async function previewFile(t,o){try{var a=o.split(".").pop().toLowerCase(),r=document.getElementById("previewContent"),i=document.getElementById("previewFileName"),n=(previewFilePath=t,previewZoom=1,previewRotation=0,previewPdfDoc=null,previewFileName=o,i&&(i.textContent=o),document.getElementById("zoomLevel")),s=(n&&(n.textContent="100%"),["jpg","jpeg","png","gif","svg","webp","bmp","ico"]),l=["mp4","webm","ogg","mov","avi","mkv"],c=["mp3","wav","flac","aac","ogg"],d=["txt","md","log","json","xml","yaml","yml","js","ts","py","java","c","cpp","go","rs","html","css","sh"],p=["pdf"];let e;if(isPublicStorageMode)try{var u=await buildPublicRequest({path:t}),m=await(await fetch("/obs/download",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(u)})).json();if(200!==m.code||!m.data||!m.data.download_url)return void showToast("获取下载链接失败："+m.message,"error");e=m.data.download_url}catch(e){return console.error("获取下载链接失败:",e),void showToast("获取下载链接失败","error")}else{var h=await fileManager.getPreviewUrl(t);if(!h.success)return void showToast("获取文件失败："+h.message,"error");e=h.url}r.innerHTML=`
    <div class="preview-loading">
      <div class="spinner"></div>
      <p class="loading-text">正在加载文件...</p>
      <div class="progress-bar"><div class="progress" style="width: 0%"></div></div>
      <p class="loading-percent">0%</p>
    </div>`,s.includes(a)?(previewFileType="image",await loadImageWithProgress(e,r,o)):l.includes(a)?(previewFileType="video",await loadVideoWithProgress(e,r)):c.includes(a)?(previewFileType="audio",await loadAudioWithProgress(e,r)):d.includes(a)?(previewFileType="text",await loadTextWithProgress(e,r)):p.includes(a)?(previewFileType="pdf",await renderPdfPreview(e,r)):(previewFileType="other",r.innerHTML='<div class="preview-placeholder"><i class="ti ti-file"></i><p>此文件类型不支持预览</p><p style="margin-top:8px;color:var(--text-secondary)">您可以下载文件后查看</p></div>'),document.getElementById("previewModal").style.display="flex"}catch(e){console.error("预览文件失败:",e),document.getElementById("previewContent").innerHTML=`<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>预览失败</p><p style="margin-top:8px;color:var(--text-secondary)">${escapeHtml(e.message)}</p></div>`,showToast("预览失败："+e.message,"error")}}function loadImageWithProgress(e,r,i){return new Promise((t,o)=>{let a=new XMLHttpRequest;(previewXhr=a).open("GET",e,!0),a.responseType="blob",a.setRequestHeader("AUTH-JWT-TOKEN",localStorage.getItem("rlist_auth_token")||""),a.onprogress=function(e){var t,o,a;e.lengthComputable&&(e=Math.round(e.loaded/e.total*100),t=r.querySelector(".progress"),o=r.querySelector(".loading-percent"),a=r.querySelector(".loading-text"),t&&(t.style.width=e+"%"),o&&(o.textContent=e+"%"),a)&&(a.textContent="正在加载图片...")},a.onload=function(){var e;200===a.status?(e=a.response,e=URL.createObjectURL(e),r.innerHTML=`<img src="${e}" alt="${escapeHtml(i)}" onload="onImageLoad()" onerror="onPreviewError()">`,t()):(r.innerHTML='<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>加载失败</p></div>',o(new Error("加载失败"))),previewXhr=null},a.onerror=function(){r.innerHTML='<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>网络错误</p></div>',o(new Error("网络错误")),previewXhr=null},a.send()})}function loadVideoWithProgress(e,r){return new Promise((t,o)=>{let a=new XMLHttpRequest;(previewXhr=a).open("GET",e,!0),a.responseType="blob",a.setRequestHeader("AUTH-JWT-TOKEN",localStorage.getItem("rlist_auth_token")||""),a.onprogress=function(e){var t,o,a;e.lengthComputable&&(e=Math.round(e.loaded/e.total*100),t=r.querySelector(".progress"),o=r.querySelector(".loading-percent"),a=r.querySelector(".loading-text"),t&&(t.style.width=e+"%"),o&&(o.textContent=e+"%"),a)&&(a.textContent="正在加载视频...")},a.onload=function(){var e;200===a.status?(e=a.response,e=URL.createObjectURL(e),r.innerHTML=`<video controls src="${e}" onloadedmetadata="onMediaLoad()"></video>`,t()):(r.innerHTML='<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>加载失败</p></div>',o(new Error("加载失败"))),previewXhr=null},a.onerror=function(){r.innerHTML='<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>网络错误</p></div>',o(new Error("网络错误")),previewXhr=null},a.send()})}function loadAudioWithProgress(e,r){return new Promise((t,o)=>{let a=new XMLHttpRequest;(previewXhr=a).open("GET",e,!0),a.responseType="blob",a.setRequestHeader("AUTH-JWT-TOKEN",localStorage.getItem("rlist_auth_token")||""),a.onprogress=function(e){var t,o,a;e.lengthComputable&&(e=Math.round(e.loaded/e.total*100),t=r.querySelector(".progress"),o=r.querySelector(".loading-percent"),a=r.querySelector(".loading-text"),t&&(t.style.width=e+"%"),o&&(o.textContent=e+"%"),a)&&(a.textContent="正在加载音频...")},a.onload=function(){var e;200===a.status?(e=a.response,e=URL.createObjectURL(e),r.innerHTML=`<audio controls src="${e}"></audio>`,t()):(r.innerHTML='<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>加载失败</p></div>',o(new Error("加载失败"))),previewXhr=null},a.onerror=function(){r.innerHTML='<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>网络错误</p></div>',o(new Error("网络错误")),previewXhr=null},a.send()})}async function loadTextWithProgress(e,r){try{let t=new XMLHttpRequest;(previewXhr=t).open("GET",e,!0),t.setRequestHeader("AUTH-JWT-TOKEN",localStorage.getItem("rlist_auth_token")||""),t.onprogress=function(e){var t,o,a;e.lengthComputable?(e=Math.round(e.loaded/e.total*100),a=r.querySelector(".progress"),t=r.querySelector(".loading-percent"),o=r.querySelector(".loading-text"),a&&(a.style.width=e+"%"),t&&(t.textContent=e+"%"),o&&(o.textContent="正在加载文本...")):(a=r.querySelector(".loading-text"))&&(a.textContent="正在加载文本...")},t.onload=function(){var e;200===t.status?(e=t.responseText,r.innerHTML=`<pre>${escapeHtml(e.substring(0,1e5))}</pre>`):r.innerHTML='<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>加载失败</p></div>',previewXhr=null},t.onerror=function(){r.innerHTML='<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>网络错误</p></div>',previewXhr=null},t.send()}catch(e){r.innerHTML='<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>无法加载文本内容</p></div>'}}function onImageLoad(){}function onMediaLoad(){}function onPreviewError(){document.getElementById("previewContent").innerHTML=`<div class="preview-placeholder">
    <i class="ti ti-alert-circle"></i>
    <p>加载失败</p>
  </div>`,showToast("预览加载失败","error")}function zoomIn(){var e;"pdf"===previewFileType?(previewZoom+=.25,updateZoomDisplay(),previewPdfDoc&&rerenderPdf()):"image"===previewFileType&&(previewZoom+=.25,updateZoomDisplay(),e=document.querySelector(".preview-content img"))&&(e.style.transform=`scale(${previewZoom})`)}function zoomOut(){var e;.5<previewZoom&&("pdf"===previewFileType?(previewZoom-=.25,updateZoomDisplay(),previewPdfDoc&&rerenderPdf()):"image"===previewFileType&&(previewZoom-=.25,updateZoomDisplay(),e=document.querySelector(".preview-content img"))&&(e.style.transform=`scale(${previewZoom})`))}function rotatePreview(){var e;previewRotation=(previewRotation+90)%360,"pdf"===previewFileType&&previewPdfDoc?rerenderPdf():"image"===previewFileType&&(e=document.querySelector(".preview-content img"))&&(e.style.transform=`rotate(${previewRotation}deg) scale(${previewZoom})`)}function updateZoomDisplay(){var e=document.getElementById("zoomLevel");e&&(e.textContent=Math.round(100*previewZoom)+"%")}async function rerenderPdf(){if(previewPdfDoc){var t=document.getElementById("pdfPageContainer");if(t){t.innerHTML='<div class="preview-placeholder"><i class="ti ti-loader"></i><p>渲染中...</p></div>';for(let e=1;e<=previewPdfDoc.numPages;e++){var o=document.createElement("div"),a=(o.className="pdf-page",o.style.marginBottom="16px",document.createElement("canvas")),o=(a.className="pdf-canvas",a.id="pdf-page-"+e,o.appendChild(a),t.appendChild(o),await previewPdfDoc.getPage(e)),r=o.getViewport({scale:previewZoom,rotation:previewRotation});a.height=r.height,a.width=r.width,await o.render({canvasContext:a.getContext("2d"),viewport:r}).promise}}}}async function renderPdfPreview(e,a){if("undefined"==typeof pdfjsLib&&(a.innerHTML=`
      <div class="pdf-loading">
        <div class="spinner"></div>
        <p class="loading-text">正在加载 PDF 组件...</p>
        <div class="progress-bar"><div class="progress" style="width: 30%"></div></div>
      </div>`,await waitForPdfJs(),"undefined"==typeof pdfjsLib))a.innerHTML=`<div class="preview-placeholder">
        <i class="ti ti-alert-circle"></i>
        <p>PDF 预览功能加载失败</p>
        <p style="margin-top:8px;color:var(--text-secondary)">请检查网络连接</p>
      </div>`;else{a.innerHTML=`
    <div class="pdf-loading">
      <div class="spinner"></div>
      <p class="loading-text">正在加载 PDF 文件...</p>
      <div class="progress-bar"><div class="progress" style="width: 0%"></div></div>
      <p class="loading-percent">0%</p>
    </div>`;try{var t=pdfjsLib.getDocument({url:e,cMapUrl:"https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/cmaps/",cMapPacked:!0}),o=(t.onProgress=function(e){var e=Math.round(e.loaded/e.total*100),t=a.querySelector(".progress"),o=a.querySelector(".loading-percent");t&&(t.style.width=e+"%"),o&&(o.textContent=e+"%")},previewPdfDoc=await t.promise,a.innerHTML='<div class="pdf-page-container" id="pdfPageContainer"></div>',document.getElementById("pdfPageContainer"));for(let e=1;e<=previewPdfDoc.numPages;e++){var r=document.createElement("div"),i=(r.className="pdf-page",r.style.marginBottom="16px",document.createElement("canvas")),n=(i.className="pdf-canvas",i.id="pdf-page-"+e,r.appendChild(i),o.appendChild(r),await previewPdfDoc.getPage(e)),s=n.getViewport({scale:previewZoom,rotation:previewRotation});i.height=s.height,i.width=s.width,await n.render({canvasContext:i.getContext("2d"),viewport:s}).promise}var l=document.createElement("div");l.style.color="var(--text-secondary)",l.style.fontSize="12px",l.style.marginTop="8px",l.textContent=`共 ${previewPdfDoc.numPages} 页`,o.appendChild(l)}catch(e){console.error("PDF 渲染失败:",e),a.innerHTML=`<div class="preview-placeholder">
      <i class="ti ti-alert-circle"></i>
      <p>PDF 加载失败</p>
      <p style="margin-top:8px;color:var(--text-secondary)">${e.message}</p>
    </div>`}}}function waitForPdfJs(){return new Promise(e=>{let t=0,o=setInterval(()=>{("undefined"!=typeof pdfjsLib||50<=++t)&&(clearInterval(o),e())},100)})}function hidePreviewModal(){try{previewXhr&&(previewXhr.abort(),previewXhr=null),document.getElementById("previewModal").style.display="none",document.getElementById("previewContent").innerHTML="",previewPdfDoc=null,previewZoom=1,previewRotation=0}catch(e){console.error("隐藏预览模态框失败:",e)}}function downloadFromPreview(){previewFilePath&&downloadFile(previewFilePath)}async function downloadFile(e){try{var t,o,a;isPublicStorageMode?(t=await buildPublicRequest({path:e}),200===(o=await(await fetch("/obs/download",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(t)})).json()).code&&o.data&&o.data.download_url?(a=o.data.download_url,window.open(a,"_blank"),showToast("下载已开始","success")):showToast("下载失败："+o.message,"error")):await fileManager.downloadFile(e)}catch(e){console.error("下载文件失败:",e),showToast("下载失败："+e.message,"error")}}async function handleUploadFiles(e){try{if(e&&0!==e.length)if(isPublicStorageMode)showToast("公开存储模式下不支持上传文件","error");else{uploadManager.setCurrentPath(fileManager.currentPath);for(var t of e)uploadManager.addFile(t);showUploadProgressModal(),await uploadManager.uploadAll()}}catch(e){console.error("处理文件上传失败:",e),showToast("上传失败："+e.message,"error")}}function updateUploadProgress(e){var t,o,a=document.getElementById("uploadProgressContent"),r="upload-"+e.file.name.replace(/[^a-zA-Z0-9]/g,"-");let i=document.getElementById(r);i?(t=i.querySelector(".upload-item-status"),o=i.querySelector(".upload-item-progress .progress"),t&&(t.textContent=e.message||"等待中..."),o&&(o.style.width=e.progress+"%")):((i=document.createElement("div")).className="upload-item",i.id=r,i.innerHTML=`
      <div class="upload-item-name">${escapeHtml(e.file.name)}</div>
      <div class="upload-item-status">${e.message||"等待中..."}</div>
      <div class="upload-item-progress">
        <div class="progress" style="width: ${e.progress}%"></div>
      </div>
    `,a.appendChild(i)),a.scrollTop=a.scrollHeight}function onUploadAllCompleted(e){var t=e.filter(e=>"completed"===e.status).length,e=e.filter(e=>"error"===e.status).length;showToast(`上传完成：成功 ${t}, 失败 `+e,0<t?"success":"error"),fileManager.refresh()}function showUploadProgressModal(){try{var e=document.getElementById("uploadProgressModal");document.getElementById("uploadProgressContent").innerHTML="",e.style.display="flex"}catch(e){console.error("显示上传进度模态框失败:",e),showToast("操作失败："+e.message,"error")}}function hideUploadProgressModal(){try{document.getElementById("uploadProgressModal").style.display="none"}catch(e){console.error("隐藏上传进度模态框失败:",e)}}function showCopyMoveModal(e){try{var t;e&&""!==e.trim()?(selectedPathForAction=e,(t=document.getElementById("copyMoveModal")).dataset.path=e,t.style.display="flex",document.getElementById("targetPathInput").value="",document.getElementById("pathSelectorStatus").textContent="点击输入框选择路径"):showToast("No files matching the criteria were found or all were skipped","error")}catch(e){console.error("显示复制/移动模态框失败:",e),showToast("操作失败："+e.message,"error")}}function hideCopyMoveModal(){document.getElementById("copyMoveModal").style.display="none"}function showPathSelector(){try{document.getElementById("pathSelectorModal").style.display="flex",loadPathSelector("/")}catch(e){console.error("显示路径选择器失败:",e),showToast("操作失败："+e.message,"error")}}function hidePathSelector(){try{document.getElementById("pathSelectorModal").style.display="none"}catch(e){console.error("隐藏路径选择器失败:",e)}}function handleModalOverlayClick(e){try{e.target===e.currentTarget&&hidePathSelector()}catch(e){console.error("处理模态框背景点击失败:",e)}}async function loadPathSelector(o){var a=document.getElementById("pathSelectorContent");console.log("加载路径选择器，路径:",o),a.innerHTML='<div class="loading"><div class="spinner"></div></div>';try{var e=await fetch(fileManager.apiBase+"/fs/list",{method:"POST",headers:{...fileManager.getAuthHeaders(),"Content-Type":"application/json"},body:JSON.stringify({path:o})});if(401===e.status)showToast("认证失败，请重新登录","error"),logout();else if(403===e.status)showToast("权限不足，无法查看目录","error");else{var r=await e.json();if(console.log("路径选择器 API 返回:",r),200===r.code&&r.data){var i,n=r.data.items||[],s=(console.log("原始项目:",n),n.filter(e=>void 0!==e.Directory).map(e=>({name:e.Directory.name||"unknown",path:o.endsWith("/")?o+(e.Directory.name||""):o+"/"+(e.Directory.name||"")}))),l=(console.log("目录列表:",s),{name:"/"===o?"根目录":o.split("/").pop()||"当前目录",path:o});let e="",t=("/"!==o&&(i=o.substring(0,o.lastIndexOf("/"))||"/",e=`
          <div class="file-item" data-action="parent" data-path="${escapeHtml(i)}">
              <div class="file-main">
                  <div class="file-icon"><i class="ti ti-arrow-up"></i></div>
                  <div class="file-name">.. (返回上级)</div>
              </div>
          </div>
        `),"");t=0===s.length?`
          <div class="empty-state" style="padding: 20px; text-align: center; color: var(--text-secondary);">
              <i class="ti ti-folder-off" style="font-size: 32px;"></i>
              <p style="margin-top: 8px;">此目录为空</p>
          </div>
        `:s.map(e=>`
              <div class="file-item" data-action="dir" data-path="${escapeHtml(e.path)}">
                  <div class="file-main" style="flex: 1; cursor: pointer;">
                      <div class="file-icon"><i class="ti ti-folder"></i></div>
                      <div class="file-name">${escapeHtml(e.name)}</div>
                  </div>
                  <button class="action-btn-sm" onclick="event.stopPropagation(); selectPath('${escapeHtml(e.path)}')" style="margin-right: 8px;" title="选择此文件夹">
                      <i class="ti ti-check"></i> 选择
                  </button>
              </div>
            `).join(""),a.innerHTML=`
        ${e}
        <div class="file-item" data-action="current" data-path="${escapeHtml(l.path)}" style="cursor: pointer;">
            <div class="file-main">
                <div class="file-icon"><i class="ti ti-check"></i></div>
                <div class="file-name">${escapeHtml(l.name)} (当前)</div>
            </div>
        </div>
        ${t}
      `;var c=a.querySelectorAll(".file-item");console.log("绑定的文件项数量:",c.length),c.forEach((a,e)=>{var t=a.getAttribute("data-action"),o=a.getAttribute("data-path");console.log(`项目 ${e}:`,{action:t,targetPath:o}),a.addEventListener("click",e=>{e.preventDefault(),e.stopPropagation();var t,e=a.getAttribute("data-action"),o=a.getAttribute("data-path");console.log("点击路径选择器项目:",e,o),"parent"===e&&o?(console.log("返回上级目录:",o),loadPathSelector(o)):"dir"===e&&o?(console.log("进入子目录:",o),t=o.endsWith("/")?o:o+"/",document.getElementById("targetPathInput").value=t,loadPathSelector(o)):"current"===e&&o&&(console.log("选择当前目录:",o),selectPath(o))})}),document.getElementById("pathSelectorStatus").textContent="当前路径："+o}else a.innerHTML='<div class="empty-state"><p>加载失败</p></div>'}}catch(e){console.error("加载路径选择器失败:",e),a.innerHTML='<div class="empty-state"><p>加载失败</p></div>',showToast("网络错误："+e.message,"error")}}function selectPath(e){try{var t=e.endsWith("/")?e:e+"/";document.getElementById("targetPathInput").value=t,hidePathSelector()}catch(e){console.error("选择路径失败:",e),showToast("操作失败："+e.message,"error")}}function confirmPathSelection(){try{let e=document.getElementById("targetPathInput").value;e?(e.endsWith("/")||(e+="/",document.getElementById("targetPathInput").value=e),hidePathSelector()):showToast("请先选择路径","error")}catch(e){console.error("确认路径选择失败:",e),showToast("操作失败："+e.message,"error")}}async function confirmCopyMove(){try{let e=document.getElementById("targetPathInput").value.trim();var t,o,a,r,i,n=document.getElementById("copyMoveType").value;e?selectedPathForAction&&""!==selectedPathForAction.trim()?(document.getElementById("confirmDeleteBtn"),o=(t=document.querySelector("#copyMoveModal .modal-footer")).innerHTML,t.innerHTML=`
      <button class="btn btn-secondary" onclick="hideCopyMoveModal()">
        取消
      </button>
      <button class="btn btn-primary" disabled>
        <i class="ti ti-loader ti-spin"></i> 处理中...
      </button>
    `,a=selectedPathForAction.split("/").pop()||selectedPathForAction,"/"!==e&&!e.endsWith("/")||(r="/"===e?"":e.endsWith("/")?e.slice(0,-1):e,e=r+"/"+a),i=await fileManager.copyOrMove(selectedPathForAction,e,n),t.innerHTML=o,i.success?(hideCopyMoveModal(),showToast(`${"copy"===n?"复制":"移动"}成功`,"success")):showToast(`${"copy"===n?"复制":"移动"}失败：`+i.message,"error")):(showToast("No files matching the criteria were found or all were skipped","error"),hideCopyMoveModal()):showToast("请输入目标路径","error")}catch(e){console.error("复制/移动失败:",e),document.querySelector("#copyMoveModal .modal-footer").innerHTML=`
      <button class="btn btn-secondary" onclick="hideCopyMoveModal()">
        取消
      </button>
      <button class="btn btn-primary" onclick="confirmCopyMove()">
        确定
      </button>
    `,showToast("操作失败："+e.message,"error")}}function showContextMenu(e,t,o){try{e.preventDefault(),e.stopPropagation(),contextMenuTarget={path:t,type:o};var a=document.getElementById("contextMenu"),r="dir"===o?`
        <div class="context-menu-item" onclick="hideContextMenu(); enterFolder('${escapeHtml(t)}')">
            <i class="ti ti-folder-open"></i> 打开
        </div>
        <div class="context-menu-item" onclick="hideContextMenu(); showCopyMoveModal('${escapeHtml(t)}')">
            <i class="ti ti-copy"></i> 复制/移动
        </div>
      `:`
        <div class="context-menu-item" onclick="hideContextMenu(); previewFile('${escapeHtml(t)}', '${escapeHtml(t.split("/").pop())}')">
            <i class="ti ti-eye"></i> 预览
        </div>
        <div class="context-menu-item" onclick="hideContextMenu(); downloadFile('${escapeHtml(t)}')">
            <i class="ti ti-download"></i> 下载
        </div>
        <div class="context-menu-item" onclick="hideContextMenu(); showCopyMoveModal('${escapeHtml(t)}')">
            <i class="ti ti-copy"></i> 复制/移动
        </div>
        <div class="context-menu-item" onclick="hideContextMenu(); copyShareUrl('${escapeHtml(t)}')">
            <i class="ti ti-link"></i> 复制分享链接
        </div>
      `;a.innerHTML=`
    ${r}
    <div class="context-menu-divider"></div>
    <div class="context-menu-item" onclick="hideContextMenu(); showRenameModal('${escapeHtml(t)}', '${escapeHtml(t.split("/").pop())}')">
        <i class="ti ti-edit"></i> 重命名
    </div>
    <div class="context-menu-item" onclick="hideContextMenu(); deleteFile('${escapeHtml(t)}')">
        <i class="ti ti-trash"></i> 删除
    </div>
    <div class="context-menu-divider"></div>
    <div class="context-menu-item" onclick="hideContextMenu(); copyPath('${escapeHtml(t)}')">
        <i class="ti ti-link"></i> 复制路径
    </div>
  `,a.style.display="block",a.style.left=e.clientX+"px",a.style.top=e.clientY+"px"}catch(e){console.error("显示右键菜单失败:",e),showToast("操作失败："+e.message,"error")}}function showContextMenuForFile(o,a){try{let e=document.getElementById("contextMenu"),t=(e.style.display="none",contextMenuTarget={path:o,type:a},event.target.getBoundingClientRect());setTimeout(()=>{e.style.display="block",e.style.left=t.left+"px",e.style.top=t.bottom+8+"px"},0)}catch(e){console.error("显示右键菜单失败:",e),showToast("操作失败："+e.message,"error")}}function hideContextMenu(){try{document.getElementById("contextMenu").style.display="none"}catch(e){console.error("隐藏右键菜单失败:",e)}}async function copyPath(e){try{await fileManager.copyPath(e),hideContextMenu()}catch(e){console.error("复制路径失败:",e),showToast("操作失败："+e.message,"error")}}async function copyShareUrl(e){try{var t,o,a;isPublicStorageMode?(t=await buildPublicRequest({path:e}),200===(o=await(await fetch("/obs/download",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(t)})).json()).code&&o.data&&o.data.download_url?(a=o.data.download_url,await copyToClipboard(a)?showToast("分享链接已复制到剪贴板","success"):showToast("复制失败","error")):showToast("获取链接失败："+o.message,"error")):await fileManager.copyShareUrl(e),hideContextMenu()}catch(e){console.error("复制分享链接失败:",e),showToast("操作失败："+e.message,"error")}}async function deleteFile(t){try{showDeleteConfirmModalWithCallback(t,async()=>{try{var e=await fileManager.remove(t);e.success?showToast("删除成功","success"):showToast("删除失败："+e.message,"error"),hideContextMenu()}catch(e){console.error("删除文件失败:",e),showToast("删除失败："+e.message,"error")}})}catch(e){console.error("删除文件失败:",e),showToast("操作失败："+e.message,"error")}}function showDeleteConfirmModalWithCallback(e,t){try{var o=document.getElementById("deleteConfirmModal"),a=document.getElementById("deleteMessage"),r=e.split("/").pop();a.textContent=`确定要删除 "${r}" 吗？`,window.deleteCallback=t,o.style.display="flex"}catch(e){console.error("显示删除确认弹窗失败:",e),showToast("操作失败："+e.message,"error")}}function openAdminPanel(){try{localStorage.getItem("rlist_auth_token")?window.location.href="/admin.html":showToast("请先登录","error")}catch(e){console.error("打开管理后台失败:",e),showToast("操作失败："+e.message,"error")}}function showRefreshCacheModal(){try{var e=document.getElementById("refreshCacheModal"),t=document.getElementById("refreshCachePath");fileManager&&fileManager.currentPath?t.value=fileManager.currentPath||"/":t.value="/",e.style.display="flex"}catch(e){console.error("显示刷新缓存模态框失败:",e),showToast("操作失败："+e.message,"error")}}function hideRefreshCacheModal(){try{document.getElementById("refreshCacheModal").style.display="none"}catch(e){console.error("隐藏刷新缓存模态框失败:",e)}}async function confirmRefreshCache(){try{var t=document.getElementById("refreshCachePath");let e=t.value.trim();e||(e="/",t.value="/"),e.startsWith("/")||(e="/"+e);var o=await(await fetch("/api/fs/refresh-cache",{method:"POST",headers:{"Content-Type":"application/json",...getAuthHeaders()},body:JSON.stringify({path:e})})).json();200===o.code?(showToast("缓存刷新成功","success"),hideRefreshCacheModal(),refresh()):showToast("刷新缓存失败："+o.message,"error")}catch(e){console.error("刷新缓存失败:",e),showToast("刷新缓存失败："+e.message,"error")}}document.addEventListener("keydown",e=>{"Enter"===e.key?"flex"===document.getElementById("newFolderModal").style.display?createFolder():"flex"===document.getElementById("renameModal").style.display?confirmRename():"flex"===document.getElementById("copyMoveModal").style.display?confirmCopyMove():"flex"===document.getElementById("pathSelectorModal").style.display&&confirmPathSelection():"Escape"===e.key&&(hideNewFolderModal(),hideRenameModal(),hidePreviewModal(),hideCopyMoveModal(),hidePathSelector(),hideContextMenu())});